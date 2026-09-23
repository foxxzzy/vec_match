use matching_engine_demo::{api, db};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let pool = db::init_pool()
        .await
        .expect("Failed to initialize database pool");

    let app = api::router(pool);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 8080)).await?;

    axum::serve(listener, app).await
}
