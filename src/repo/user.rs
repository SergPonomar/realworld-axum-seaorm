use crate::middleware::auth::create_token;
use entity::entities::{follower, prelude::User, user};
use migration::SimpleExpr;
use sea_orm::{
    prelude::Uuid, query::*, ColumnTrait, DatabaseConnection, DbErr, DeleteResult, EntityTrait,
    FromQueryResult, InsertResult, QueryFilter, RelationTrait,
};
use serde::Serialize;

pub async fn get_user_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<user::Model>, DbErr> {
    User::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await
}

pub async fn get_user_by_username(
    db: &DatabaseConnection,
    username: &str,
) -> Result<Option<user::Model>, DbErr> {
    User::find()
        .filter(user::Column::Username.eq(username))
        .one(db)
        .await
}

pub async fn get_user_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<user::Model>, DbErr> {
    User::find_by_id(id).one(db).await
}

pub async fn get_user_with_token_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<UserWithToken>, DbErr> {
    User::find_by_id(id)
        .into_model::<UserWithToken>()
        .one(db)
        .await
}

pub async fn create_user(
    db: &DatabaseConnection,
    user: user::ActiveModel,
) -> Result<InsertResult<user::ActiveModel>, DbErr> {
    User::insert(user).exec(db).await
}

pub async fn update_user(
    db: &DatabaseConnection,
    user: user::ActiveModel,
) -> Result<user::Model, DbErr> {
    User::update(user).exec(db).await
}

pub async fn get_profile_by_username(
    db: &DatabaseConnection,
    username: &str,
    current_user_id: Option<Uuid>,
) -> Result<Option<Profile>, DbErr> {
    User::find()
        .filter(user::Column::Username.eq(username))
        .column_as(
            author_followed_by_current_user(current_user_id),
            "following",
        )
        .into_model::<Profile>()
        .one(db)
        .await
}

pub fn author_followed_by_current_user(user_id: Option<Uuid>) -> SimpleExpr {
    match user_id {
        Some(id) => user::Column::Id.in_subquery(
            User::find()
                .join(JoinType::InnerJoin, follower::Relation::User1.def().rev())
                .select_only()
                .column(follower::Column::UserId)
                .filter(follower::Column::FollowerId.eq(id))
                .into_query(),
        ),
        None => false.into(),
    }
}

pub async fn empty_user_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    User::delete_many().exec(db).await
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UserWithToken {
    token: String,
    email: String,
    username: String,
    bio: Option<String>,
    image: Option<String>,
}

#[derive(Clone, Debug, PartialEq, FromQueryResult, Eq, Serialize)]
pub struct Profile {
    username: String,
    bio: Option<String>,
    image: Option<String>,
    following: bool,
}

impl FromQueryResult for UserWithToken {
    fn from_query_result(res: &sea_orm::QueryResult, pre: &str) -> Result<Self, sea_orm::DbErr> {
        let id: Uuid = res.try_get(pre, "id")?;

        Ok(Self {
            token: create_token(id).unwrap(),
            email: res.try_get(pre, "email")?,
            username: res.try_get(pre, "username")?,
            bio: res.try_get(pre, "bio")?,
            image: res.try_get(pre, "image")?,
        })
    }
}

impl From<user::Model> for UserWithToken {
    fn from(model: user::Model) -> Self {
        Self {
            token: create_token(model.id).unwrap(),
            email: model.email,
            username: model.username,
            bio: model.bio,
            image: model.image,
        }
    }
}
