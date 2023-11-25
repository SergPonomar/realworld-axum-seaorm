use entity::entities::{follower, prelude::Follower};
use sea_orm::{DatabaseConnection, DbErr, DeleteResult, EntityTrait, InsertResult};

pub async fn create_follower(
    db: &DatabaseConnection,
    follower: follower::ActiveModel,
) -> Result<InsertResult<follower::ActiveModel>, DbErr> {
    Follower::insert(follower).exec(db).await
}

// .on_conflict(
//     OnConflict::columns(follower::Column::iter())
//         .do_nothing()
//         .to_owned(),
// )

pub async fn delete_follower(
    db: &DatabaseConnection,
    follower: follower::ActiveModel,
) -> Result<DeleteResult, DbErr> {
    Follower::delete(follower).exec(db).await
}

pub async fn empty_follower_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    Follower::delete_many().exec(db).await
}
