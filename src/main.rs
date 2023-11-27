use sea_orm::DbErr;
// use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
// use tracing_subscriber::util::SubscriberInitExt;

use dotenvy::dotenv;
mod api;
mod app;
mod middleware;
mod repo;
mod seed;
mod tests;
use app::{db, server};

#[tokio::main]
async fn main() -> Result<(), DbErr> {
    // let stdout_log = tracing_subscriber::fmt::layer().pretty();
    // tracing_subscriber::registry().with(stdout_log).init();

    dotenv().expect(".env file not found");

    let connection = db::start().await?;
    server::start(connection).await;

    Ok(())
}
