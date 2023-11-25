use crate::middleware::auth::Token;
use crate::repo::{
    follower::{create_follower, delete_follower},
    user::{get_profile_by_username, get_user_by_username, Profile},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use entity::entities::{follower, user};
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use serde::Serialize;

pub async fn get_profile(
    State(db): State<DatabaseConnection>,
    maybe_token: Option<Extension<Token>>,
    Path(username): Path<String>,
) -> Result<Json<ProfileDto>, (StatusCode, String)> {
    let current_user_id = maybe_token.map(|tkn| tkn.id);

    let profile = get_profile_by_username(&db, &username, current_user_id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "User not finded".to_string(),
        ))?;

    let profile_dto = ProfileDto { profile };
    Ok(Json(profile_dto))
}

pub async fn follow_user(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
    Path(username): Path<String>,
) -> Result<Json<ProfileDto>, (StatusCode, String)> {
    let current_user_id = token.id;

    let following_user: user::Model = get_user_by_username(&db, &username)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "User not finded".to_string(),
        ))?;

    let follower_model = follower::ActiveModel {
        user_id: Set(following_user.id),
        follower_id: Set(current_user_id),
    };

    let _flw_res = create_follower(&db, follower_model)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let profile = get_profile_by_username(&db, &username, Some(current_user_id))
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Profile not finded".to_string(),
        ))?;

    let profile_dto = ProfileDto { profile };
    Ok(Json(profile_dto))
}

pub async fn unfollow_user(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
    Path(username): Path<String>,
) -> Result<Json<ProfileDto>, (StatusCode, String)> {
    let current_user_id = token.id;

    let following_user: user::Model = get_user_by_username(&db, &username)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "User not finded".to_string(),
        ))?;

    let follower_model = follower::ActiveModel {
        user_id: Set(following_user.id),
        follower_id: Set(current_user_id),
    };

    let _flw_res = delete_follower(&db, follower_model)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let profile = get_profile_by_username(&db, &username, Some(current_user_id))
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Profile not finded".to_string(),
        ))?;

    let profile_dto = ProfileDto { profile };
    Ok(Json(profile_dto))
}

#[derive(Debug, Serialize)]
pub struct ProfileDto {
    profile: Profile,
}
