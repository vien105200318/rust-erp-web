use axum::{
    extract::FromRef,
    routing::get,
    Router,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;
use tokio::sync::broadcast;

// 👇 1. Thêm dòng này
use tower_http::services::ServeDir;

mod handlers;
mod models;
use handlers::{get_channels, get_history, get_users, get_dm_history, handler_chat_ws, handler_hello, login, register, mark_channel_read};

// ... (Giữ nguyên AppState và các struct khác) ...
#[derive(Clone)]
struct AppState {
    pool: PgPool,
    tx: broadcast::Sender<String>,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}
impl FromRef<AppState> for broadcast::Sender<String> {
    fn from_ref(state: &AppState) -> Self {
        state.tx.clone()
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Không tìm thấy file .env");
    let database_url = env::var("DATABASE_URL").expect("Chưa set DATABASE_URL");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Không thể kết nối DB");

    let (tx, _rx) = broadcast::channel(100);
    let app_state = AppState { pool, tx };

    println!("✅ Đã kết nối Neon Postgres!");

    let app = Router::new()
        .route("/", get(handler_hello))
        // 👇 2. Đăng ký đường dẫn cho thư mục assets
        .nest_service("/assets", ServeDir::new("assets"))
        .route("/ws", get(handler_chat_ws))
        .route("/history", get(get_history))
        .route("/dm_history", get(get_dm_history))
        .route("/channels", get(get_channels))
        .route("/chanels/reads", axum::routing::post(mark_channel_read))
        .route("/users", get(get_users))
        .route("/register", axum::routing::post(register))
        .route("/login", axum::routing::post(login))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 Discord Mini đang chạy tại http://localhost:3000");

    axum::serve(listener, app).await.unwrap();
}