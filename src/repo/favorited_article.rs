use entity::entities::{favorited_article, prelude::FavoritedArticle};
use sea_orm::{DatabaseConnection, DbErr, DeleteResult, EntityTrait, InsertResult};

pub async fn favorite_article(
    db: &DatabaseConnection,
    favorite_article: favorited_article::ActiveModel,
) -> Result<InsertResult<favorited_article::ActiveModel>, DbErr> {
    FavoritedArticle::insert(favorite_article).exec(db).await
}

// .on_conflict(
//     OnConflict::columns(favorited_article::Column::iter())
//         .do_nothing()
//         .to_owned(),
// )

pub async fn unfavorite_article(
    db: &DatabaseConnection,
    favorite_article: favorited_article::ActiveModel,
) -> Result<DeleteResult, DbErr> {
    FavoritedArticle::delete(favorite_article).exec(db).await
}

pub async fn empty_favorited_article_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    FavoritedArticle::delete_many().exec(db).await
}
