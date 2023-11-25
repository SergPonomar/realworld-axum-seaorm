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
                    .col(ColumnDef::new(Article::Id).uuid().not_null().primary_key())
                    .col(
                        ColumnDef::new(Article::Slug)
                            .string()
                            .unique_key()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Article::Title).string().not_null())
                    .col(ColumnDef::new(Article::Description).string().not_null())
                    .col(ColumnDef::new(Article::Body).text().not_null())
                    .col(ColumnDef::new(Article::AuthorId).uuid().not_null())
                    .col(
                        ColumnDef::new(Article::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Article::UpdatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp()),
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
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-article")
                    .if_not_exists()
                    .table(Article::Table)
                    .col(Article::AuthorId)
                    .col(Article::Title)
                    .unique()
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
    Id,
    Slug,
    Title,
    Description,
    Body,
    AuthorId,
    CreatedAt,
    UpdatedAt,
}
