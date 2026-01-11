# Rust ERP Web (Axum)

Backend cho hệ thống **ERP Web** được xây dựng bằng **Rust** sử dụng **Axum framework**, hướng tới hiệu năng cao, an toàn bộ nhớ và kiến trúc rõ ràng. Project phù hợp cho **đồ án, học tập** hoặc làm **base backend** cho hệ thống ERP.

---

## 🚀 Tính năng (định hướng)

* Authentication / Authorization
* Quản lý người dùng & phân quyền
* CRUD các module ERP (users, products, orders, ...)
* REST API chuẩn JSON
* Logging & error handling
* Database migration

---

## 🛠 Công nghệ sử dụng

* **Rust** (stable)
* **Axum** – Web framework
* **Tokio** – Async runtime
* **Tower / Tower-HTTP** – Middleware (CORS, tracing, timeout)
* **Serde** – Serialize / Deserialize JSON
* **SQLx** – Database async + migration
* **dotenvy** – Load biến môi trường
* **tracing** – Logging & observability

---

## 📂 Cấu trúc thư mục

```text
rust-erp-web/
├── src/
│   ├── main.rs            # Entry point
│   ├── app.rs             # Khởi tạo router & middleware
│   ├── config.rs          # Load config từ env
│   ├── db.rs              # Database pool (SQLx)
│   ├── routes/            # Định nghĩa routes
│   ├── handlers/          # Xử lý HTTP request
│   ├── services/          # Business logic
│   ├── models/            # Entity / DTO
│   └── error.rs           # AppError & response JSON
├── migrations/            # SQLx migrations
├── .env                   # Biến môi trường (local/dev)
├── Cargo.toml
└── README.md
```

---

## ⚙️ Cấu hình môi trường

Tạo file `.env` tại thư mục root:

```env
APP_HOST=127.0.0.1
APP_PORT=3000

DATABASE_URL=postgres://postgres:password@localhost:5432/erp_db

RUST_LOG=info
```

> ⚠️ `.env` chỉ dùng cho môi trường local/dev.

---

## ▶️ Chạy project

### 1. Cài Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Clone project

```bash
git clone https://github.com/your-username/rust-erp-web.git
cd rust-erp-web
```

### 3. Chạy migration (nếu dùng SQLx)

```bash
sqlx migrate run
```

### 4. Chạy server

```bash
cargo run
```

Server chạy tại:

```
http://127.0.0.1:3000
```

---

## 📡 API mẫu

### Health check

```http
GET /health
```

Response:

```json
{ "status": "ok" }
```

---

## 🧪 Testing

```bash
cargo test
```

---

## 📝 Ghi chú

* Project ưu tiên code rõ ràng, dễ đọc
* Phù hợp cho đồ án hoặc prototype
* Có thể mở rộng thêm JWT, Swagger, RBAC

---

## 📄 License

MIT (hoặc tuỳ chỉnh)
