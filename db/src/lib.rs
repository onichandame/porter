use migration::run_migration;
use sqlx::sqlite;
use std::env;

mod migration;

pub type ConnectionPool = sqlite::SqlitePool;

pub async fn new_connection_pool() -> ConnectionPool {
    let db_addr_key = "DATABASE_URL";
    let db_addr = match env::var("UNITTEST") {
        Ok(_) => String::from("sqlite://:memory:"),
        _other => env::var(&db_addr_key).expect("DATABASE_URL not set"),
    };
    let pool = sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_addr)
        .await
        .unwrap();
    run_migration(&pool).await;
    pool
}
