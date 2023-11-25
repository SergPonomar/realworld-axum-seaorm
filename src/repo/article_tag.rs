use entity::entities::prelude::ArticleTag;
use entity::entities::{article_tag, tag};
use sea_orm::{
    query::*, ColumnTrait, DatabaseConnection, DbErr, DeleteResult, EntityTrait, InsertResult,
    RelationTrait, TryInsertResult,
};
use uuid::Uuid;

pub async fn create_article_tags(
    db: &DatabaseConnection,
    article_tags: Vec<article_tag::ActiveModel>,
) -> Result<TryInsertResult<InsertResult<article_tag::ActiveModel>>, DbErr> {
    ArticleTag::insert_many(article_tags)
        .on_empty_do_nothing()
        .exec(db)
        .await
}

// .on_conflict(
//     OnConflict::columns(article_tag::Column::iter())
//         .do_nothing()
//         .to_owned(),
// )

pub async fn insert_article_tag(
    db: &DatabaseConnection,
    article_tag: article_tag::ActiveModel,
) -> Result<InsertResult<article_tag::ActiveModel>, DbErr> {
    ArticleTag::insert(article_tag).exec(db).await
}

pub async fn get_article_tags(
    db: &DatabaseConnection,
    article_id: Uuid,
) -> Result<Vec<String>, DbErr> {
    ArticleTag::find()
        .join(JoinType::LeftJoin, article_tag::Relation::Tag.def())
        .filter(article_tag::Column::ArticleId.eq(article_id))
        .select_only()
        .column(tag::Column::TagName)
        .into_tuple::<String>()
        .all(db)
        .await
}

pub async fn empty_article_tag_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    ArticleTag::delete_many().exec(db).await
}
