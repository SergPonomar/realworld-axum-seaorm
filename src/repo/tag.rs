use entity::entities::{prelude::Tag, tag};
use migration::{Alias, Expr};
use sea_orm::{
    DatabaseConnection, DbErr, DeleteResult, EntityTrait, InsertResult, QueryFilter, QuerySelect,
    TryInsertResult,
};

pub async fn create_tags(
    db: &DatabaseConnection,
    tags: Vec<tag::ActiveModel>,
) -> Result<TryInsertResult<InsertResult<tag::ActiveModel>>, DbErr> {
    Tag::insert_many(tags).on_empty_do_nothing().exec(db).await
}

pub async fn insert_tag(
    db: &DatabaseConnection,
    tag: tag::ActiveModel,
) -> Result<InsertResult<tag::ActiveModel>, DbErr> {
    Tag::insert(tag).exec(db).await
}

pub async fn get_tags_ids(
    db: &DatabaseConnection,
    tags: Vec<String>,
) -> Result<Vec<tag::Model>, DbErr> {
    Tag::find()
        .filter(Expr::expr(Expr::col(tag::Column::TagName).cast_as(Alias::new("text"))).is_in(tags))
        .all(db)
        .await
}

pub async fn get_tags(db: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
    Tag::find()
        .select_only()
        .column(tag::Column::TagName)
        .into_tuple::<String>()
        .all(db)
        .await
}

pub async fn empty_tag_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    Tag::delete_many().exec(db).await
}
