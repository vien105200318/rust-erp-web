use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    RequestPartsExt,
};

use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use jsonwebtoken::{decode, DecodingKey, Validation};

// --- User & Auth ---
#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct UserPublic {
    pub username: String,
}
#[derive(Deserialize, Debug)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Debug)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
}

// Cấu trúc JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

// --- Middleware: AuthUser ---
pub struct AuthUser {
    pub username: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Lấy Token từ Header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Thiếu Token đăng nhập"))?;

        // 2. Kiểm tra Token
        // ⚠️ Đảm bảo key này khớp với handlers.rs
        let token_data = decode::<Claims>(
            bearer.token(),
            &DecodingKey::from_secret(b"SECRET_KEY"),
            &Validation::default(),
        )
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Token không hợp lệ hoặc đã hết hạn"))?;

        Ok(AuthUser {
            username: token_data.claims.sub,
        })
    }
}

// --- Chat Models ---

// 👇 ĐÂY LÀ CÁI BẠN ĐANG THIẾU HOẶC SAI
#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct Channel {
    pub id: i64,
    pub name: String, // <-- Phải có dòng này thì SQL mới map được
}

#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct Message {
    pub id: i64,
    pub channel_id: Option<i64>, // Tin nhắn thuộc kênh nào
    pub username: String,
    pub content: String,
}

// Client gửi lên phải có channel_id
#[derive(Deserialize, Debug)]
pub struct CreateMessage {
    pub channel_id: i64,
    pub content: String,
}