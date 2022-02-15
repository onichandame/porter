use sqlx::sqlite;
use sqlx::{self, migrate};

pub async fn run_migration(db: &sqlite::SqlitePool) {
    migrate!().run(db).await.unwrap();
}
