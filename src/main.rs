use sea_orm::DbErr;
// use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
// use tracing_subscriber::util::SubscriberInitExt;

use dotenvy::dotenv;
mod app;
use app::{db, server};

#[tokio::main]
async fn main() -> Result<(), DbErr> {
    // let stdout_log = tracing_subscriber::fmt::layer().pretty();
    // tracing_subscriber::registry().with(stdout_log).init();

    dotenv().expect(".env file not found");
    let _connection = db::start().await?;
    server::start().await;

    Ok(())
}
