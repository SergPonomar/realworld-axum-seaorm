use dotenvy::dotenv;
use sea_orm::DbErr;

mod api;
mod app;
mod middleware;
mod repo;
mod seed;
#[allow(dead_code)]
mod tests;
use app::{db, server};

#[tokio::main]
async fn main() -> Result<(), DbErr> {
    dotenv().expect(".env file not found");

    let connection = db::start().await?;
    server::start(connection).await;

    Ok(())
}
