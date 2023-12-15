use super::user::{author_followed_by_current_user, Profile};
use entity::entities::{comment, prelude::Comment, user};
use sea_orm::{
    entity::prelude::DateTime, query::*, ColumnTrait, DatabaseConnection, DbErr, DeleteResult,
    EntityTrait, FromQueryResult, QueryFilter, RelationTrait,
};
use serde::Serialize;
use uuid::Uuid;

/// Insert `comment` for the provided `ActiveModel`.
/// Returns `InsertResult` with last inserted id on success, otherwise
/// returns an `database error`.
/// Empty input produce error as not allowed on database level.
/// See [`InsertResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.InsertResult.html)
/// documentation for more details.
pub async fn insert_comment(
    db: &DatabaseConnection,
    comment: comment::ActiveModel,
) -> Result<InsertResult<comment::ActiveModel>, DbErr> {
    Comment::insert(comment).exec(db).await
}

/// Fetch `comment` with additional info (see ArticleWithAuthor for details) for the provided `id`.
/// Optional identifier used to determine whether the logged in user is a follower of the author.
/// Returns optional `comment` on success, otherwise returns an `database error`.
pub async fn get_comment_by_id(
    db: &DatabaseConnection,
    id: Uuid,
    current_user_id: Option<Uuid>,
) -> Result<Option<CommentWithAuthor>, DbErr> {
    Comment::find_by_id(id)
        .join(JoinType::LeftJoin, comment::Relation::User.def())
        .column(user::Column::Username)
        .column(user::Column::Bio)
        .column(user::Column::Image)
        .column_as(
            author_followed_by_current_user(current_user_id),
            "following",
        )
        .into_model::<CommentWithAuthor>()
        .one(db)
        .await
}

/// Fetch `comments` with additional info (see ArticleWithAuthor for details) for the provided `article id`.
/// Optional identifier used to determine whether the logged in user is a follower of the author.
/// Returns list of `comments` on success, otherwise returns an `database error`.
pub async fn get_comments_by_article_id(
    db: &DatabaseConnection,
    article_id: Uuid,
    current_user_id: Option<Uuid>,
) -> Result<Vec<CommentWithAuthor>, DbErr> {
    Comment::find()
        .join(JoinType::LeftJoin, comment::Relation::User.def())
        .filter(comment::Column::ArticleId.eq(article_id))
        .column(user::Column::Username)
        .column(user::Column::Bio)
        .column(user::Column::Image)
        .column_as(
            author_followed_by_current_user(current_user_id),
            "following",
        )
        .into_model::<CommentWithAuthor>()
        .all(db)
        .await
}

/// Delete `comment` for the provided id.
/// Returns `DeleteResult` with affected rows count on success, otherwise
/// returns an `database error`.
/// See [`DeleteResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.DeleteResult.html)
/// documentation for more details.
pub async fn delete_comment(
    db: &DatabaseConnection,
    comment_id: Uuid,
) -> Result<DeleteResult, DbErr> {
    Comment::delete_by_id(comment_id).exec(db).await
}

/// Delete all existing `comment records` from database.
/// Returns `DeleteResult` with affected rows count on success, otherwise
/// returns an `database error`.
/// See [`DeleteResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.DeleteResult.html)
/// documentation for more details.
pub async fn empty_comment_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    Comment::delete_many().exec(db).await
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommentWithAuthor {
    id: Uuid,
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

#[cfg(test)]
mod test_insert_comment {
    use super::insert_comment;
    use crate::tests::{Operation::Insert, TestData, TestDataBuilder, TestErr};
    use chrono::Local;
    use entity::entities::{comment, prelude::Comment};
    use sea_orm::Set;
    use std::vec;
    use uuid::Uuid;

    #[tokio::test]
    async fn insert_not_exist_data() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                articles, users, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .comments(Insert(vec![(1, 1), (2, 1), (2, 2)]))
            .build()
            .await?;

        let author_id = users.as_ref().unwrap()[1].id;
        let article_id = articles.as_ref().unwrap()[4].id;
        let comment_id = Uuid::new_v4();

        let model = comment::ActiveModel {
            id: Set(comment_id),
            body: Set("body".to_owned()),
            author_id: Set(author_id),
            article_id: Set(article_id),
            created_at: Set(Some(Local::now().naive_local())),
            updated_at: Set(Some(Local::now().naive_local())),
        };

        let insert_result = insert_comment(&connection, model).await?;
        assert_eq!(insert_result.last_insert_id, comment_id);

        Ok(())
    }

    #[tokio::test]
    async fn insert_not_existing_author() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .comments(Insert(vec![(1, 1), (2, 1), (2, 2)]))
            .build()
            .await?;

        let article_id = articles.as_ref().unwrap()[4].id;

        let model = comment::ActiveModel {
            id: Set(Uuid::new_v4()),
            body: Set("body".to_owned()),
            author_id: Set(Uuid::new_v4()),
            article_id: Set(article_id),
            created_at: Set(Some(Local::now().naive_local())),
            updated_at: Set(Some(Local::now().naive_local())),
        };

        let insert_result = insert_comment(&connection, model).await;
        assert!(insert_result
            .is_err_and(|err| err.to_string().ends_with("FOREIGN KEY constraint failed")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_not_existing_article() -> Result<(), TestErr> {
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .comments(Insert(vec![(1, 1), (2, 1), (2, 2)]))
            .build()
            .await?;

        let author_id = users.as_ref().unwrap()[1].id;

        let model = comment::ActiveModel {
            id: Set(Uuid::new_v4()),
            body: Set("body".to_owned()),
            author_id: Set(author_id),
            article_id: Set(Uuid::new_v4()),
            created_at: Set(Some(Local::now().naive_local())),
            updated_at: Set(Some(Local::now().naive_local())),
        };

        let insert_result = insert_comment(&connection, model).await;
        assert!(insert_result
            .is_err_and(|err| err.to_string().ends_with("FOREIGN KEY constraint failed")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_data() -> Result<(), TestErr> {
        let (connection, TestData { comments, .. }) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .comments(Insert(vec![(1, 1), (2, 1), (2, 2)]))
            .build()
            .await?;

        let actives = TestDataBuilder::activate_models::<Comment, comment::ActiveModel>(&comments);
        let model = actives.into_iter().next().unwrap();

        let insert_result = insert_comment(&connection, model).await;
        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: comment.id")));

        Ok(())
    }
}

#[cfg(test)]
mod test_get_comment_by_id {
    use super::{get_comment_by_id, CommentWithAuthor};
    use crate::repo::user::Profile;
    use crate::tests::{Operation::Insert, TestData, TestDataBuilder, TestErr};
    use std::vec;
    use uuid::Uuid;

    #[tokio::test]
    async fn get_existing_comment() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, comments, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .comments(Insert(vec![(1, 1), (2, 1), (2, 2)]))
            .build()
            .await?;

        let author = users.unwrap().into_iter().nth(1).unwrap();
        let comment = comments.unwrap().into_iter().nth(1).unwrap();

        let expected = CommentWithAuthor {
            id: comment.id,
            body: comment.body,
            author: Profile {
                username: author.username.clone(),
                bio: author.bio.clone(),
                image: author.image.clone(),
                following: false,
            },
            created_at: comment.created_at,
            updated_at: comment.updated_at,
        };

        let result = get_comment_by_id(&connection, comment.id, None).await?;
        assert_eq!(result, Some(expected));

        Ok(())
    }

    #[tokio::test]
    async fn none_existing_id() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .comments(Insert(vec![(1, 1), (2, 1), (2, 2)]))
            .build()
            .await?;

        let result = get_comment_by_id(&connection, Uuid::new_v4(), None).await?;
        let expected = None;
        assert_eq!(result, expected);

        Ok(())
    }
}

#[cfg(test)]
mod test_get_comments_by_article_id {
    use super::{get_comments_by_article_id, CommentWithAuthor};
    use crate::tests::{Operation::Insert, TestData, TestDataBuilder, TestErr};
    use std::vec;
    use uuid::Uuid;

    #[tokio::test]
    async fn get_existing_article_id() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .comments(Insert(vec![(1, 1), (2, 1), (2, 2), (3, 3)]))
            .build()
            .await?;

        let article = articles.unwrap().into_iter().next().unwrap();
        let result = get_comments_by_article_id(&connection, article.id, None).await?;
        assert_eq!(result.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn none_existing_article_id() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .comments(Insert(vec![(1, 1), (2, 1), (2, 2)]))
            .build()
            .await?;

        let result = get_comments_by_article_id(&connection, Uuid::new_v4(), None).await?;
        let expected: Vec<CommentWithAuthor> = vec![];
        assert_eq!(result, expected);

        Ok(())
    }
}

#[cfg(test)]
mod test_delete_comment {
    use super::delete_comment;
    use crate::tests::{Operation::Insert, TestData, TestDataBuilder, TestErr};

    #[tokio::test]
    async fn delete_existing_data() -> Result<(), TestErr> {
        let (connection, TestData { comments, .. }) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .comments(Insert(vec![(1, 1), (2, 1), (2, 2)]))
            .build()
            .await?;

        let comment_id = comments.unwrap().into_iter().next().unwrap().id;
        let delete_result = delete_comment(&connection, comment_id).await?;
        assert_eq!(delete_result.rows_affected, 1_u64);

        Ok(())
    }
}

#[cfg(test)]
mod test_empty_comment_table {
    use super::empty_comment_table;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestDataBuilder, TestErr,
    };
    use entity::entities::{comment, prelude::Comment};
    use sea_orm::EntityTrait;

    #[tokio::test]
    async fn delete_existing_followers() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .comments(Insert(vec![(1, 1), (2, 1), (2, 2)]))
            .build()
            .await?;

        let delete_result = empty_comment_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 3_u64);

        let expected: Vec<comment::Model> = Vec::new();
        let result = Comment::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn delete_empty_table() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Migration)
            .articles(Migration)
            .comments(Migration)
            .build()
            .await?;

        let delete_result = empty_comment_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 0_u64);

        let expected: Vec<comment::Model> = Vec::new();
        let result = Comment::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }
}
