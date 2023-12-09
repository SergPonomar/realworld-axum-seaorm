use crate::seed::{empty_all_tables, populate_seeds};
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
    let _empty_res = empty_all_tables(&connection).await;
    let _seed_res = populate_seeds(&connection).await;

    Migrator::up(&connection, None).await?;

    Ok(connection)
}