use crate::middleware::auth::Token;
use crate::repo::user::{
    create_user, get_user_by_email, get_user_by_id, get_user_with_token_by_id,
    update_user as repo_update_user, UserWithToken,
};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use axum::{extract::State, http::StatusCode, Extension, Json};
use entity::entities::*;
use rand_core::OsRng;
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub async fn login_user(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<LoginUserDto>,
) -> Result<Json<UserDto>, (StatusCode, String)> {
    let input = payload.user;

    let current_user = get_user_by_email(&db, &input.email)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "User not finded".to_string(),
        ))?;

    let _pass_matched = PasswordHash::new(&current_user.password)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
        .map(|parsed_hash| {
            Argon2::default().verify_password(input.password.as_bytes(), &parsed_hash)
        })
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.1))?;

    let user_dto = UserDto {
        user: current_user.into(),
    };

    Ok(Json(user_dto))
}

pub async fn register_user(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<RegisterUserDto>,
) -> Result<Json<UserDto>, (StatusCode, String)> {
    let input = payload.user;

    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(input.password.as_bytes(), &salt)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
        .map(|hash| hash.to_string())?;

    let user_model = user::ActiveModel {
        id: Set(Uuid::new_v4()),
        email: Set(input.email),
        username: Set(input.username),
        password: Set(hashed_password),
        ..Default::default()
    };

    let user_res = create_user(&db, user_model)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let current_user = get_user_with_token_by_id(&db, user_res.last_insert_id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "User not finded".to_string(),
        ))?;

    let user_dto = UserDto { user: current_user };
    Ok(Json(user_dto))
}

pub async fn get_current_user(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
) -> Result<Json<UserDto>, (StatusCode, String)> {
    let current_user = get_user_with_token_by_id(&db, token.id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "User not finded".to_string(),
        ))?;

    let user_dto = UserDto { user: current_user };
    Ok(Json(user_dto))
}

pub async fn update_user(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
    Json(payload): Json<UpdateUserDto>,
) -> Result<Json<UserDto>, (StatusCode, String)> {
    let input = payload.user;

    let user_before = get_user_by_id(&db, token.id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "User not finded".to_string(),
        ))?;

    let mut user_model: user::ActiveModel = user_before.into();

    // TODO add macro for easy set active model
    if input.email.is_some() {
        user_model.email = Set(input.email.to_owned().unwrap());
    }
    if input.username.is_some() {
        user_model.username = Set(input.username.to_owned().unwrap());
    }
    if input.bio.is_some() {
        user_model.bio = Set(input.bio.to_owned());
    }
    if input.image.is_some() {
        user_model.image = Set(input.image);
    }
    if input.password.is_some() {
        user_model.password = Set(input.password.to_owned().unwrap());
    }

    let current_user = repo_update_user(&db, user_model)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let user_dto = UserDto {
        user: current_user.into(),
    };
    Ok(Json(user_dto))
}

#[derive(Debug, Serialize)]
pub struct UserDto {
    user: UserWithToken,
}

#[derive(Debug, Deserialize)]
pub struct LoginUserDto {
    user: LoginUser,
}

#[derive(Debug, Deserialize)]
pub struct RegisterUserDto {
    user: RegisterUser,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserDto {
    user: UpdateUser,
}

#[derive(Clone, Debug, Deserialize)]
struct UpdateUser {
    email: Option<String>,
    username: Option<String>,
    bio: Option<String>,
    password: Option<String>,
    image: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct LoginUser {
    email: String,
    password: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RegisterUser {
    username: String,
    email: String,
    password: String,
}
