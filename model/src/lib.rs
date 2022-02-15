use db;
use sea_orm::{DatabaseConnection, SqlxSqliteConnector};

pub use sea_orm::{
    ActiveModelTrait, ActiveValue, EntityTrait, ModelTrait, NotSet, Set, Unchanged, Value,
};

pub mod gate;
pub mod service;

pub type Database = DatabaseConnection;

pub async fn new_database() -> Database {
    SqlxSqliteConnector::from_sqlx_sqlite_pool(db::new_connection_pool().await)
}
