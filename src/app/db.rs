use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use std::env;

const DATABASE_URL: &str = "DATABASE_URL";
const DATABASE_SCHEMA: &str = "DATABASE_SCHEMA";

pub async fn start() -> Result<DatabaseConnection, DbErr> {
    let url = env::var(DATABASE_URL).expect("DATABASE_URL environment variable not set");
    let schema = env::var(DATABASE_SCHEMA).unwrap_or("public".to_string());
    let connect_options = ConnectOptions::new(url)
        .set_schema_search_path(schema)
        .to_owned();

    let connection: DatabaseConnection = Database::connect(connect_options).await?;

    Migrator::up(&connection, None).await?;

    test_connection(&connection).await;

    Ok(connection)
}

async fn test_connection(db: &DatabaseConnection) {
    assert!(db.ping().await.is_ok());
    let _ = db.clone().close().await;
    assert!(matches!(db.ping().await, Err(DbErr::ConnectionAcquire(_))));
}
