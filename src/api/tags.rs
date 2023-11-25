use crate::repo::tag::get_tags;
use axum::{extract::State, http::StatusCode, Json};
use sea_orm::DatabaseConnection;
use serde::Serialize;

pub async fn list_tags(
    State(db): State<DatabaseConnection>,
) -> Result<Json<TagsDto>, (StatusCode, String)> {
    let tags = get_tags(&db)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let tags_dto = TagsDto { tags };
    Ok(Json(tags_dto))
}

#[derive(Debug, Serialize)]
pub struct TagsDto {
    tags: Vec<String>,
}
