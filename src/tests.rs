use migration::SchemaManager;
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection, DbErr};

pub async fn init_test_db_connection() -> Result<DatabaseConnection, DbErr> {
    Database::connect("sqlite::memory:").await
}

// pub async fn create_table_for_test_db<E>(
//     connection: &DatabaseConnection,
//     entity: E,
// ) -> Result<ExecResult, DbErr>
// where
//     E: EntityTrait,
// {
//     let schema = Schema::new(DbBackend::Sqlite);
//     let stmt: TableCreateStatement = schema.create_table_from_entity(entity);

//     // Create table from entity
//     connection
//         .execute(connection.get_database_backend().build(&stmt))
//         .await
// }

/// Execute migration for provided connection. Useful for table creation.
pub async fn execute_migration(
    connection: &DatabaseConnection,
    migration_name: &str,
) -> Result<(), DbErr> {
    let manager = SchemaManager::new(connection);
    let migrations = Migrator::migrations();
    let migration = migrations
        .iter()
        .find(|mgr| mgr.name() == migration_name)
        .unwrap();

    migration.up(&manager).await
}
