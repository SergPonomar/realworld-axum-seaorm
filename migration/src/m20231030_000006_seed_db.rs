use crate::m20231030_000001_create_user_table::User;
use crate::m20231030_000002_create_article_table::Article;
use sea_orm_migration::prelude::*;
// use sea_orm_migration::sea_orm::{entity::*, query::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create test user
        let user = Query::insert()
            .into_table(User::Table)
            .columns([
                User::Email,
                User::Username,
                User::Bio,
                User::Image,
            ])
            .values_panic([
                "admin@sergponomar.com".into(),
                "Admin".into(),
                "Developer bio".into(),
                "https://upload.wikimedia.org/wikipedia/commons/1/1f/Logo_of_YouTube_%282005-2006%29.svg".into()
            ])
            .to_owned();

        manager.exec_stmt(user).await?;

        // Create test article
        let article = Query::insert()
            .into_table(Article::Table)
            .columns([
                Article::Slug,
                Article::Title,
                Article::Description,
                Article::Body,
                Article::Favorited,
                Article::FavoritesCount,
                Article::AuthorId,
            ])
            .values_panic([
                "test-article".into(),
                "test article".into(),
                "Description: the best description in the world".into(),
                "Mega body of article".into(),
                false.into(),
                0.into(),
                1.into(),
            ])
            .to_owned();

        manager.exec_stmt(article).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // manager
        //     .truncate_table(Table::truncate().table(Article::Table).to_owned())
        //     .await
        manager
            .truncate_table(
                Table::truncate()
                    .table(User::Table)
                    .to_owned()
                    .extra("CASCADE".to_string()),
            )
            .await
    }
}
