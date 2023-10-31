use crate::m20231030_000002_create_article_table::Article;
use crate::m20231030_000004_create_tag_table::Tag;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ArticleTag::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ArticleTag::TagId).integer().not_null())
                    .col(ColumnDef::new(ArticleTag::ArticleSlug).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_article-tag_tag")
                            .from(ArticleTag::Table, ArticleTag::TagId)
                            .to(Tag::Table, Tag::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_article-tag_article")
                            .from(ArticleTag::Table, ArticleTag::ArticleSlug)
                            .to(Article::Table, Article::Slug)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-article-tag")
                    .if_not_exists()
                    .table(ArticleTag::Table)
                    .col(ArticleTag::TagId)
                    .col(ArticleTag::ArticleSlug)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tag::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ArticleTag {
    Table,
    TagId,
    ArticleSlug,
}
