use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State, Query},
    response::{Html, IntoResponse},
    Json,
    http::StatusCode,
};
use sqlx::PgPool;
use tokio::sync::broadcast;
use futures::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{encode, decode, DecodingKey, EncodingKey, Header, Validation};
use std::time::{SystemTime, UNIX_EPOCH};

// Import đầy đủ models
use crate::models::{
    AuthUser, Claims, Message as MessageModel, LoginRequest, LoginResponse,
    RegisterRequest, User, Channel, CreateMessage, UserPublic,
    PrivateMessage, CreateDM, WsMessage, ClientTyping
};

const SECRET_KEY: &[u8] = b"SECRET_KEY";

// --- 1. Giao diện & Auth ---
pub async fn handler_hello() -> Html<&'static str> {
    Html(include_str!("../index.html"))
}

pub async fn register(State(pool): State<PgPool>, Json(payload): Json<RegisterRequest>) -> impl IntoResponse {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(payload.password.as_bytes(), &salt).unwrap().to_string();

    let result = sqlx::query!("INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING id", payload.username, password_hash)
        .fetch_one(&pool).await;

    match result {
        Ok(_) => (StatusCode::CREATED, "Đăng ký thành công").into_response(),
        Err(_) => (StatusCode::CONFLICT, "Tên đăng nhập đã tồn tại").into_response(),
    }
}

pub async fn login(State(pool): State<PgPool>, Json(payload): Json<LoginRequest>) -> impl IntoResponse {
    let user = sqlx::query_as!(User, "SELECT id, username, password_hash FROM users WHERE username = $1", payload.username)
        .fetch_optional(&pool).await.unwrap();

    if let Some(user) = user {
        let parsed_hash = PasswordHash::new(&user.password_hash).unwrap();
        if Argon2::default().verify_password(payload.password.as_bytes(), &parsed_hash).is_ok() {
            let expiration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as usize + 86400;
            let claims = Claims { sub: user.username.clone(), exp: expiration };
            let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(SECRET_KEY)).unwrap();
            return Json(LoginResponse { token, username: user.username }).into_response();
        }
    }
    (StatusCode::UNAUTHORIZED, "Sai tài khoản hoặc mật khẩu").into_response()
}

// --- 2. Public API (Channels & Users) ---
pub async fn get_channels(_user: AuthUser, State(pool): State<PgPool>) -> Json<Vec<Channel>> {
    let channels = sqlx::query_as!(Channel, "SELECT id, name FROM channels ORDER BY id ASC")
        .fetch_all(&pool).await.unwrap_or(vec![]);
    Json(channels)
}

pub async fn get_users(_user: AuthUser, State(pool): State<PgPool>) -> Json<Vec<UserPublic>> {
    let users = sqlx::query_as!(UserPublic, "SELECT username FROM users ORDER BY username ASC")
        .fetch_all(&pool).await.unwrap_or(vec![]);
    Json(users)
}

// --- 3. History API (Channel & DM) ---

#[derive(Deserialize)]
pub struct HistoryParams { channel_id: i64 }

pub async fn get_history(_user: AuthUser, Query(params): Query<HistoryParams>, State(pool): State<PgPool>) -> Json<Vec<MessageModel>> {
    let msgs = sqlx::query_as!(MessageModel, "SELECT id, channel_id, username, content FROM messages WHERE channel_id = $1 ORDER BY id ASC LIMIT 50", params.channel_id)
        .fetch_all(&pool).await.unwrap_or(vec![]);
    Json(msgs)
}

// MỚI: API lấy lịch sử chat riêng
#[derive(Deserialize)]
pub struct DmHistoryParams { with_user: String }

pub async fn get_dm_history(
    user: AuthUser,
    Query(params): Query<DmHistoryParams>,
    State(pool): State<PgPool>
) -> Json<Vec<PrivateMessage>> {
    let msgs = sqlx::query_as!(
        PrivateMessage,
        "SELECT id, sender, receiver, content FROM private_messages
         WHERE (sender = $1 AND receiver = $2) OR (sender = $2 AND receiver = $1)
         ORDER BY id ASC LIMIT 50",
        user.username,
        params.with_user
    )
        .fetch_all(&pool)
        .await
        .unwrap_or(vec![]);

    Json(msgs)
}

// --- 4. WebSocket (Nâng cấp Logic) ---

#[derive(Deserialize)]
pub struct WsParams { token: String }

pub async fn handler_chat_ws(ws: WebSocketUpgrade, Query(params): Query<WsParams>, State(pool): State<PgPool>, State(tx): State<broadcast::Sender<String>>) -> impl IntoResponse {
    let validation = decode::<Claims>(&params.token, &DecodingKey::from_secret(SECRET_KEY), &Validation::default());
    match validation {
        Ok(token_data) => ws.on_upgrade(move |socket| handle_socket(socket, pool, tx, token_data.claims.sub)),
        Err(_) => (StatusCode::UNAUTHORIZED, "Token WebSocket không hợp lệ").into_response(),
    }
}

async fn handle_socket(socket: WebSocket, pool: PgPool, tx: broadcast::Sender<String>, username: String) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() { break; }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // Client gửi JSON, ta thử parse xem nó là loại nào

            // TRƯỜNG HỢP 1: Chat Kênh
            if let Ok(input) = serde_json::from_str::<CreateMessage>(&text) {
                // Lưu DB
                let _ = sqlx::query!("INSERT INTO messages (channel_id, username, content) VALUES ($1, $2, $3)", input.channel_id, username, input.content)
                    .execute(&pool).await;

                // Gửi broadcast
                let msg = WsMessage::Channel {
                    channel_id: input.channel_id,
                    username: username.clone(),
                    content: input.content
                };
                let _ = tx.send(serde_json::to_string(&msg).unwrap());
            }
            // TRƯỜNG HỢP 2: Chat Riêng (DM)
            else if let Ok(dm_input) = serde_json::from_str::<CreateDM>(&text) {
                // Lưu DB
                let _ = sqlx::query!("INSERT INTO private_messages (sender, receiver, content) VALUES ($1, $2, $3)", username, dm_input.receiver, dm_input.content)
                    .execute(&pool).await;

                // Gửi broadcast
                let msg = WsMessage::DM {
                    sender: username.clone(),
                    receiver: dm_input.receiver,
                    content: dm_input.content
                };
                let _ = tx.send(serde_json::to_string(&msg).unwrap());
            }
            // TRƯỜNG HỢP 3: Typing (Đang gõ...)
            else if let Ok(typing) = serde_json::from_str::<ClientTyping>(&text) {
                // KHÔNG LƯU DB, chỉ bắn tin hiệu
                let msg = WsMessage::Typing {
                    username: username.clone(),
                    channel_id: typing.channel_id,
                    sender: if typing.receiver.is_some() {
                        Some(username.clone())
                    } else {
                        None
                    },
                };
                let _ = tx.send(serde_json::to_string(&msg).unwrap());
            }
        }
    });

    tokio::select! { _ = (&mut send_task) => recv_task.abort(), _ = (&mut recv_task) => send_task.abort() };
}