use super::error::ApiErr;
use crate::middleware::auth::{check_passwords, hash_password, Token};
use crate::repo::user::{
    create_user, get_user_by_email, get_user_by_id, get_user_with_token_by_id,
    update_user as repo_update_user, UserWithToken,
};
use axum::{extract::State, Extension, Json};
use entity::entities::*;
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Axum handler for login user.
/// Returns json object with user on success, otherwise returns an `api error`.
pub async fn login_user(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<LoginUserDto>,
) -> Result<Json<UserDto>, ApiErr> {
    let input = payload.user;

    let current_user = get_user_by_email(&db, &input.email)
        .await?
        .ok_or(ApiErr::UserNotExist)?;

    check_passwords(&input.password, &current_user.password).map_err(|_err| ApiErr::WrongPass)?;

    let user_dto = UserDto {
        user: current_user.into(),
    };

    Ok(Json(user_dto))
}

/// Axum handler for register user.
/// Returns json object with user on success, otherwise returns an `api error`.
pub async fn register_user(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<RegisterUserDto>,
) -> Result<Json<UserDto>, ApiErr> {
    let input = payload.user;
    let hashed_password = hash_password(&input.password).map_err(|_err| ApiErr::WrongPass)?;

    let user_model = user::ActiveModel {
        id: Set(Uuid::new_v4()),
        email: Set(input.email),
        username: Set(input.username),
        password: Set(hashed_password),
        ..Default::default()
    };

    let user_res = create_user(&db, user_model).await?;
    let current_user = get_user_with_token_by_id(&db, user_res.last_insert_id)
        .await?
        .ok_or(ApiErr::UserNotExist)?;

    let user_dto = UserDto { user: current_user };
    Ok(Json(user_dto))
}

/// Axum handler for retrieve information about logged user.
/// Returns json object with user on success, otherwise returns an `api error`.
pub async fn get_current_user(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
) -> Result<Json<UserDto>, ApiErr> {
    let current_user = get_user_with_token_by_id(&db, token.id)
        .await?
        .ok_or(ApiErr::UserNotExist)?;

    let user_dto = UserDto { user: current_user };
    Ok(Json(user_dto))
}

/// Axum handler for update information about logged user.
/// Returns json object with user on success, otherwise returns an `api error`.
pub async fn update_user(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
    Json(payload): Json<UpdateUserDto>,
) -> Result<Json<UserDto>, ApiErr> {
    let input = payload.user;

    let user_before = get_user_by_id(&db, token.id)
        .await?
        .ok_or(ApiErr::UserNotExist)?;

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

    let current_user = repo_update_user(&db, user_model).await?;

    let user_dto = UserDto {
        user: current_user.into(),
    };
    Ok(Json(user_dto))
}

/// Struct describing JSON object, returned by handler. Contains user info with authentication token.
#[derive(Debug, Serialize, PartialEq)]
pub struct UserDto {
    user: UserWithToken,
}

/// Struct describing JSON object from login request. Contains user loggin data.
#[derive(Debug, Deserialize)]
pub struct LoginUserDto {
    user: LoginUser,
}

#[derive(Clone, Debug, Deserialize)]
struct LoginUser {
    email: String,
    password: String,
}

/// Struct describing JSON object from registration request. Contains user loggin data.
#[derive(Debug, Deserialize)]
pub struct RegisterUserDto {
    user: RegisterUser,
}

#[derive(Clone, Debug, Deserialize)]
struct RegisterUser {
    username: String,
    email: String,
    password: String,
}

/// Struct describing JSON object from change user data request. Contains user profile data.
#[derive(Debug, Deserialize)]
pub struct UpdateUserDto {
    user: UpdateUser,
}

#[derive(Clone, Default, Debug, Deserialize)]
struct UpdateUser {
    email: Option<String>,
    username: Option<String>,
    bio: Option<String>,
    password: Option<String>,
    image: Option<String>,
}

#[cfg(test)]
mod test_login_user {
    use super::{login_user, LoginUser, LoginUserDto, UserDto};
    use crate::api::error::ApiErr;
    use crate::middleware::auth::hash_password;
    use crate::repo::user::create_user;
    use crate::tests::{
        Operation::{Create, Insert},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::{extract::State, Json};
    use dotenvy::dotenv;
    use entity::entities::user;
    use sea_orm::ActiveModelTrait;

    #[tokio::test]
    async fn login_existing_user() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) =
            TestDataBuilder::new().users(Create(1)).build().await?;
        let user: user::Model = users.unwrap().into_iter().next().unwrap();
        let user_hashed: user::ActiveModel = user::Model {
            password: hash_password("password").unwrap(),
            ..user.clone()
        }
        .into();
        let user_hashed = user_hashed.reset_all();
        create_user(&connection, user_hashed).await?;

        // Actual test start
        let expected = UserDto { user: user.into() };
        let login_data = LoginUserDto {
            user: LoginUser {
                email: "email1".to_owned(),
                password: "password".to_owned(),
            },
        };

        let result = login_user(State(connection), Json(login_data)).await?;
        let Json(result) = result;

        assert_eq!(result.user.email, expected.user.email);

        Ok(())
    }

    #[tokio::test]
    async fn wrong_email() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new().users(Insert(1)).build().await?;

        let login_data = LoginUserDto {
            user: LoginUser {
                email: "wrong email".to_owned(),
                password: "password".to_owned(),
            },
        };
        let result = login_user(State(connection), Json(login_data)).await;

        matches!(result, Err(ApiErr::UserNotExist));

        Ok(())
    }

    #[tokio::test]
    async fn wrong_password() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) =
            TestDataBuilder::new().users(Create(1)).build().await?;
        let user: user::Model = users.unwrap().into_iter().next().unwrap();
        let user_hashed: user::ActiveModel = user::Model {
            password: hash_password("password").unwrap(),
            ..user.clone()
        }
        .into();
        let user_hashed = user_hashed.reset_all();
        create_user(&connection, user_hashed).await?;

        // Actual test start
        let login_data = LoginUserDto {
            user: LoginUser {
                email: "email1".to_owned(),
                password: "wrong password".to_owned(),
            },
        };

        let result = login_user(State(connection), Json(login_data)).await;
        matches!(result, Err(ApiErr::WrongPass));

        Ok(())
    }
}

#[cfg(test)]
mod test_register_user {
    use super::{register_user, RegisterUser, RegisterUserDto};
    use crate::api::error::ApiErr;
    use crate::tests::{
        Operation::{Create, Insert},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::{extract::State, Json};
    use dotenvy::dotenv;
    use entity::entities::user;
    use sea_orm::DbErr;

    #[tokio::test]
    async fn register_new_user() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) =
            TestDataBuilder::new().users(Create(1)).build().await?;
        let user: user::Model = users.unwrap().into_iter().next().unwrap();

        let reg_data = RegisterUserDto {
            user: RegisterUser {
                email: user.email.clone(),
                password: user.password,
                username: user.username,
            },
        };

        let result = register_user(State(connection), Json(reg_data)).await?;
        let Json(result) = result;
        assert_eq!(result.user.email, user.email);

        Ok(())
    }

    #[tokio::test]
    async fn exist_user_with_email() -> Result<(), TestErr> {
        let (connection, TestData { users, .. }) =
            TestDataBuilder::new().users(Insert(1)).build().await?;
        let user: user::Model = users.unwrap().into_iter().next().unwrap();

        let reg_data = RegisterUserDto {
            user: RegisterUser {
                email: user.email,
                password: user.password,
                username: "other_username".to_owned(),
            },
        };

        let result = register_user(State(connection), Json(reg_data)).await;
        matches!(result, Err(ApiErr::DbErr(DbErr::Exec(_))));

        Ok(())
    }

    #[tokio::test]
    async fn exist_user_with_username() -> Result<(), TestErr> {
        let (connection, TestData { users, .. }) =
            TestDataBuilder::new().users(Insert(1)).build().await?;
        let user: user::Model = users.unwrap().into_iter().next().unwrap();

        let reg_data = RegisterUserDto {
            user: RegisterUser {
                email: "other_email".to_owned(),
                password: user.password,
                username: user.username,
            },
        };

        let result = register_user(State(connection), Json(reg_data)).await;
        matches!(result, Err(ApiErr::DbErr(DbErr::Exec(_))));

        Ok(())
    }
}

#[cfg(test)]
mod test_get_current_user {
    use super::{get_current_user, UserDto};
    use crate::api::error::ApiErr;
    use crate::middleware::auth::Token;
    use crate::tests::{
        Operation::{Create, Insert},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::{extract::State, Extension, Json};
    use dotenvy::dotenv;
    use entity::entities::user;

    #[tokio::test]
    async fn get_existing_user() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) =
            TestDataBuilder::new().users(Insert(1)).build().await?;
        let user: user::Model = users.unwrap().into_iter().next().unwrap();
        let token = Token {
            exp: 35,
            id: user.id,
        };

        // Actual test start
        let expected = UserDto { user: user.into() };
        let result = get_current_user(State(connection), Extension(token)).await?;
        let Json(result) = result;

        assert_eq!(result.user.email, expected.user.email);

        Ok(())
    }

    #[tokio::test]
    async fn get_non_existing_user() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) =
            TestDataBuilder::new().users(Create(1)).build().await?;
        let user: user::Model = users.unwrap().into_iter().next().unwrap();
        let token = Token {
            exp: 35,
            id: user.id,
        };

        let result = get_current_user(State(connection), Extension(token)).await;
        matches!(result, Err(ApiErr::UserNotExist));

        Ok(())
    }
}

#[cfg(test)]
mod test_update_user {
    use super::{update_user, UpdateUser, UpdateUserDto, UserDto};
    use crate::api::error::ApiErr;
    use crate::middleware::auth::Token;
    use crate::tests::{
        Operation::{Create, Insert},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::{extract::State, Extension, Json};
    use dotenvy::dotenv;
    use entity::entities::user;

    #[tokio::test]
    async fn update_existing_user() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) =
            TestDataBuilder::new().users(Insert(1)).build().await?;
        let new_user_name = "updated_username";
        let mut user: user::Model = users.unwrap().into_iter().next().unwrap();
        user.username = new_user_name.to_owned();

        let payload = UpdateUserDto {
            user: UpdateUser {
                username: Some(new_user_name.to_owned()),
                ..Default::default()
            },
        };

        let token = Token {
            exp: 35,
            id: user.id,
        };

        // Actual test start
        let expected = UserDto { user: user.into() };
        let result = update_user(State(connection), Extension(token), Json(payload)).await?;
        let Json(result) = result;

        assert_eq!(result.user.username, expected.user.username);

        Ok(())
    }

    #[tokio::test]
    async fn update_non_existing_user() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) =
            TestDataBuilder::new().users(Create(1)).build().await?;
        let user: user::Model = users.unwrap().into_iter().next().unwrap();

        let payload = UpdateUserDto {
            user: UpdateUser {
                username: Some("updated_username".to_owned()),
                ..Default::default()
            },
        };

        let token = Token {
            exp: 35,
            id: user.id,
        };

        // Actual test start
        let result = update_user(State(connection), Extension(token), Json(payload)).await;

        matches!(result, Err(ApiErr::UserNotExist));

        Ok(())
    }
}
