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
    http::StatusCode,
    Extension, Json,
};
use entity::entities::comment;
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub async fn create_comment(
    Path(slug): Path<String>,
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
    Json(payload): Json<CreateCommentDto>,
) -> Result<Json<CommentDto>, (StatusCode, String)> {
    let current_user_id = token.id;
    let input = payload.comment;

    // Find Article
    let commented_article = get_article_model_by_slug(&db, &slug)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Article not finded".to_string(),
        ))?;

    let comment_model = comment::ActiveModel {
        id: Set(Uuid::new_v4()),
        body: Set(input.body),
        author_id: Set(current_user_id),
        article_id: Set(commented_article.id),
        ..Default::default()
    };

    let cmnt_res = insert_comment(&db, comment_model)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let comment = get_comment_by_id(&db, cmnt_res.last_insert_id, current_user_id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Comment not inserted".to_string(),
        ))?;

    let comment_dto = CommentDto { comment };
    Ok(Json(comment_dto))
}

pub async fn list_comments(
    Path(slug): Path<String>,
    maybe_token: Option<Extension<Token>>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<CommentsDto>, (StatusCode, String)> {
    // Find Article
    let commented_article = get_article_model_by_slug(&db, &slug)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Article not finded".to_string(),
        ))?;

    let comments =
        get_comments_by_article_id(&db, commented_article.id, maybe_token.map(|tkn| tkn.id))
            .await
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let comments_dto = CommentsDto { comments };
    Ok(Json(comments_dto))
}

pub async fn delete_comment(
    Path((_slug, comment_id)): Path<(String, Uuid)>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<()>, (StatusCode, String)> {
    let _delete_res = repo_delete_comment(&db, comment_id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(Json(()))
}

#[derive(Debug, Serialize)]
pub struct TagsDto {
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CommentsDto {
    comments: Vec<CommentWithAuthor>,
}

#[derive(Debug, Serialize)]
pub struct CommentDto {
    comment: CommentWithAuthor,
}

#[derive(Clone, Debug, Deserialize)]
struct CreateComment {
    body: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentDto {
    comment: CreateComment,
}
