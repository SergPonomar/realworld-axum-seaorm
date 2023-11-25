use super::user::{author_followed_by_current_user, Profile};
use entity::entities::{comment, prelude::Comment, user};
use sea_orm::{
    entity::prelude::DateTime, query::*, ColumnTrait, DatabaseConnection, DbErr, DeleteResult,
    EntityTrait, FromQueryResult, QueryFilter, RelationTrait,
};
use serde::Serialize;
use uuid::Uuid;

pub async fn insert_comment(
    db: &DatabaseConnection,
    comment: comment::ActiveModel,
) -> Result<InsertResult<comment::ActiveModel>, DbErr> {
    Comment::insert(comment).exec(db).await
}

pub async fn get_comment_by_id(
    db: &DatabaseConnection,
    id: Uuid,
    current_user_id: Uuid,
) -> Result<Option<CommentWithAuthor>, DbErr> {
    Comment::find_by_id(id)
        .join(JoinType::LeftJoin, comment::Relation::User.def())
        .column(user::Column::Username)
        .column_as(
            author_followed_by_current_user(Some(current_user_id)),
            "following",
        )
        .into_model::<CommentWithAuthor>()
        .one(db)
        .await
}

pub async fn get_comments_by_article_id(
    db: &DatabaseConnection,
    article_id: Uuid,
    current_user_id: Option<Uuid>,
) -> Result<Vec<CommentWithAuthor>, DbErr> {
    Comment::find()
        .join(JoinType::LeftJoin, comment::Relation::User.def())
        .filter(comment::Column::ArticleId.eq(article_id))
        .column(user::Column::Username)
        .column_as(
            author_followed_by_current_user(current_user_id),
            "following",
        )
        .into_model::<CommentWithAuthor>()
        .all(db)
        .await
}

pub async fn delete_comment(
    db: &DatabaseConnection,
    comment_id: Uuid,
) -> Result<DeleteResult, DbErr> {
    Comment::delete_by_id(comment_id).exec(db).await
}

pub async fn empty_comment_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    Comment::delete_many().exec(db).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentWithAuthor {
    id: i32,
    body: String,
    created_at: Option<DateTime>,
    updated_at: Option<DateTime>,
    author: Profile,
}

impl FromQueryResult for CommentWithAuthor {
    fn from_query_result(res: &sea_orm::QueryResult, pre: &str) -> Result<Self, sea_orm::DbErr> {
        Ok(Self {
            id: res.try_get(pre, "id")?,
            body: res.try_get(pre, "body")?,
            created_at: res.try_get(pre, "created_at")?,
            updated_at: res.try_get(pre, "updated_at")?,
            author: Profile::from_query_result(res, pre)?,
        })
    }
}
