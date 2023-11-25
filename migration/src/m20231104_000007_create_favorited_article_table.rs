use crate::m20231030_000001_create_user_table::User;
use crate::m20231030_000002_create_article_table::Article;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FavoritedArticle::Table)
                    .if_not_exists()
                    .primary_key(
                        Index::create()
                            .name("idx-favorited_article")
                            .if_not_exists()
                            .table(FavoritedArticle::Table)
                            .col(FavoritedArticle::ArticleId)
                            .col(FavoritedArticle::UserId),
                    )
                    .col(
                        ColumnDef::new(FavoritedArticle::ArticleId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(FavoritedArticle::UserId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_favorited_article-article")
                            .from(FavoritedArticle::Table, FavoritedArticle::ArticleId)
                            .to(Article::Table, Article::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_favorited_article-user")
                            .from(FavoritedArticle::Table, FavoritedArticle::UserId)
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
                    .name("idx-favorited_article")
                    .if_not_exists()
                    .table(FavoritedArticle::Table)
                    .col(FavoritedArticle::UserId)
                    .col(FavoritedArticle::ArticleId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FavoritedArticle::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum FavoritedArticle {
    Table,
    ArticleId,
    UserId,
}
