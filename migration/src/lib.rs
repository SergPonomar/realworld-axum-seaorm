pub use sea_orm_migration::prelude::*;

mod m20231030_000001_create_user_table;
mod m20231030_000002_create_article_table;
mod m20231030_000003_create_comment_table;
mod m20231030_000004_create_tag_table;
mod m20231030_000005_create_article_tag_table;
mod m20231101_000006_create_follower_table;
mod m20231104_000007_create_favorited_article_table;
mod m20231112_000008_add_user_password;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20231030_000001_create_user_table::Migration),
            Box::new(m20231030_000002_create_article_table::Migration),
            Box::new(m20231030_000003_create_comment_table::Migration),
            Box::new(m20231030_000004_create_tag_table::Migration),
            Box::new(m20231030_000005_create_article_tag_table::Migration),
            Box::new(m20231101_000006_create_follower_table::Migration),
            Box::new(m20231104_000007_create_favorited_article_table::Migration),
            Box::new(m20231112_000008_add_user_password::Migration),
        ]
    }
}
