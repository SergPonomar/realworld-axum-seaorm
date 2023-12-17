use super::error::ApiErr;
use crate::middleware::auth::Token;
use crate::repo::{
    article::get_article_model_by_slug,
    comment::{
        delete_comment as repo_delete_comment, get_comment_by_id, get_comments_by_article_id,
        insert_comment, CommentWithAuthor,
    },
};
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use entity::entities::comment;
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Axum handler for creating article comment.
/// Returns json object with comment on success, otherwise returns an `api error`.
pub async fn create_comment(
    Path(slug): Path<String>,
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
    Json(payload): Json<CreateCommentDto>,
) -> Result<Json<CommentDto>, ApiErr> {
    let current_user_id = token.id;
    let input = payload.comment;

    let commented_article = get_article_model_by_slug(&db, &slug)
        .await?
        .ok_or(ApiErr::ArticleNotExist)?;

    let comment_model = comment::ActiveModel {
        id: Set(Uuid::new_v4()),
        body: Set(input.body),
        author_id: Set(current_user_id),
        article_id: Set(commented_article.id),
        ..Default::default()
    };

    let cmnt_res = insert_comment(&db, comment_model).await?;

    let comment = get_comment_by_id(&db, cmnt_res.last_insert_id, Some(current_user_id))
        .await?
        .ok_or(ApiErr::CommentNotExist)?;

    let comment_dto = CommentDto { comment };
    Ok(Json(comment_dto))
}

/// Axum handler for fetch all article `comments`.
/// Returns json object with list of comments on success, otherwise returns an `api error`.
pub async fn list_comments(
    Path(slug): Path<String>,
    maybe_token: Option<Extension<Token>>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<CommentsDto>, ApiErr> {
    let commented_article = get_article_model_by_slug(&db, &slug)
        .await?
        .ok_or(ApiErr::ArticleNotExist)?;

    let comments =
        get_comments_by_article_id(&db, commented_article.id, maybe_token.map(|tkn| tkn.id))
            .await?;

    let comments_dto = CommentsDto { comments };
    Ok(Json(comments_dto))
}

/// Axum handler for delete comment by provided comment id.
/// Returns empty json object on success, otherwise returns an `api error`.
pub async fn delete_comment(
    Path((_slug, comment_id)): Path<(String, Uuid)>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<()>, ApiErr> {
    let del_res = repo_delete_comment(&db, comment_id).await?;

    if del_res.rows_affected > 0 {
        Ok(Json(()))
    } else {
        Err(ApiErr::CommentNotExist)
    }
}

/// Struct describing JSON object, returned by handler. Contains list of comments.
#[derive(Debug, Serialize)]
pub struct CommentsDto {
    comments: Vec<CommentWithAuthor>,
}

/// Struct describing JSON object, returned by handler. Contains comment.
#[derive(Debug, Serialize)]
pub struct CommentDto {
    comment: CommentWithAuthor,
}

/// Struct describing JSON object from comment creation request. Contains comment.
#[derive(Debug, Deserialize)]
pub struct CreateCommentDto {
    comment: CreateComment,
}

#[derive(Clone, Debug, Deserialize)]
struct CreateComment {
    body: String,
}

#[cfg(test)]
mod test_create_comment {
    use super::{create_comment, CreateComment, CreateCommentDto};
    use crate::api::error::ApiErr;
    use crate::middleware::auth::Token;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::{
        extract::{Path, State},
        Extension, Json,
    };
    use dotenvy::dotenv;
    use entity::entities::{article, user};

    #[tokio::test]
    async fn create_new_comment() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1]))
            .comments(Migration)
            .followers(Migration)
            .build()
            .await?;
        let current_user: user::Model = users.unwrap().into_iter().next().unwrap();
        let article: article::Model = articles.unwrap().into_iter().next().unwrap();
        let comment_text = "comment";

        let comment_data = CreateCommentDto {
            comment: CreateComment {
                body: comment_text.to_owned(),
            },
        };

        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let result = create_comment(
            Path(article.slug),
            State(connection),
            Extension(token),
            Json(comment_data),
        )
        .await?;
        let Json(result) = result;
        assert_eq!(result.comment.body, comment_text.to_owned());

        Ok(())
    }

    #[tokio::test]
    async fn comment_for_not_existing_article() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1]))
            .comments(Migration)
            .followers(Migration)
            .build()
            .await?;
        let current_user: user::Model = users.unwrap().into_iter().next().unwrap();

        let comment_data = CreateCommentDto {
            comment: CreateComment {
                body: "comment".to_owned(),
            },
        };

        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let result = create_comment(
            Path("not existing slug".to_owned()),
            State(connection),
            Extension(token),
            Json(comment_data),
        )
        .await;

        matches!(result, Err(ApiErr::ArticleNotExist));

        Ok(())
    }
}

#[cfg(test)]
mod test_list_comments {
    use super::list_comments;
    use crate::api::error::ApiErr;
    use crate::{
        middleware::auth::Token,
        tests::{
            Operation::{Insert, Migration},
            TestData, TestDataBuilder, TestErr,
        },
    };
    use axum::{
        extract::{Path, State},
        Extension, Json,
    };
    use dotenvy::dotenv;
    use entity::entities::{article, user};
    use std::vec;

    #[tokio::test]
    async fn get_existing_comments() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1]))
            .comments(Insert(vec![(2, 1), (2, 2), (3, 1), (5, 1)]))
            .followers(Migration)
            .build()
            .await?;

        let current_user: user::Model = users.unwrap().into_iter().next().unwrap();
        let article: article::Model = articles.unwrap().into_iter().next().unwrap();

        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let result = list_comments(
            Path(article.slug),
            Some(Extension(token)),
            State(connection),
        )
        .await?;
        let Json(result) = result;

        assert_eq!(result.comments.len(), 3);

        Ok(())
    }

    #[tokio::test]
    async fn article_with_no_comments() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1]))
            .comments(Migration)
            .followers(Migration)
            .build()
            .await?;

        let current_user: user::Model = users.unwrap().into_iter().next().unwrap();
        let article: article::Model = articles.unwrap().into_iter().next().unwrap();

        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let result = list_comments(
            Path(article.slug),
            Some(Extension(token)),
            State(connection),
        )
        .await?;
        let Json(result) = result;

        assert_eq!(result.comments.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn not_exist_article() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1]))
            .comments(Insert(vec![(2, 1), (2, 2), (3, 1), (5, 1)]))
            .followers(Migration)
            .build()
            .await?;

        let current_user: user::Model = users.unwrap().into_iter().next().unwrap();

        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let result = list_comments(
            Path("not existing article".to_owned()),
            Some(Extension(token)),
            State(connection),
        )
        .await;

        matches!(result, Err(ApiErr::ArticleNotExist));

        Ok(())
    }
}

#[cfg(test)]
mod test_delete_comment {
    use super::delete_comment;
    use crate::api::error::ApiErr;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::extract::{Path, State};
    use entity::entities::comment;
    use std::vec;
    use uuid::Uuid;

    #[tokio::test]
    async fn delete_existing_comment() -> Result<(), TestErr> {
        let (connection, TestData { comments, .. }) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1]))
            .comments(Insert(vec![(2, 1), (2, 2), (3, 1), (5, 1)]))
            .followers(Migration)
            .build()
            .await?;

        let comment: comment::Model = comments.unwrap().into_iter().next().unwrap();

        let _result =
            delete_comment(Path(("slug".to_owned(), comment.id)), State(connection)).await?;

        Ok(())
    }

    #[tokio::test]
    async fn delete_non_existing_comment() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1]))
            .comments(Insert(vec![(2, 1), (2, 2), (3, 1), (5, 1)]))
            .followers(Migration)
            .build()
            .await?;

        let result =
            delete_comment(Path(("slug".to_owned(), Uuid::new_v4())), State(connection)).await;

        matches!(result, Err(ApiErr::CommentNotExist));

        Ok(())
    }
}
