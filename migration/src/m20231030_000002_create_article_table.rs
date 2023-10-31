use crate::m20231030_000001_create_user_table::User;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Article::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Article::Slug)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Article::Title)
                            .string()
                            .unique_key()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Article::Description).text())
                    .col(ColumnDef::new(Article::Body).text())
                    .col(ColumnDef::new(Article::Favorited).boolean())
                    .col(ColumnDef::new(Article::FavoritesCount).integer())
                    .col(ColumnDef::new(Article::AuthorId).integer().not_null())
                    .col(
                        ColumnDef::new(Article::CreatedAt)
                            .timestamp()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Article::UpdatedAt)
                            .timestamp()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_article_user")
                            .from(Article::Table, Article::AuthorId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Article::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Article {
    Table,
    Slug,
    Title,
    Description,
    Body,
    Favorited,
    FavoritesCount,
    AuthorId,
    CreatedAt,
    UpdatedAt,
}
