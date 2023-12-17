use crate::middleware::auth::Token;
use crate::repo::{
    follower::{create_follower, delete_follower},
    user::{get_profile_by_username, get_user_by_username, Profile},
};
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use entity::entities::{follower, user};
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use serde::Serialize;

use super::error::ApiErr;

/// Axum handler for retrieve information about user with provided username. Optional
/// token used to determine whether the logged in user is a follower of the profile.
/// Returns json object with profile on success, otherwise returns an `api error`.
pub async fn get_profile(
    State(db): State<DatabaseConnection>,
    maybe_token: Option<Extension<Token>>,
    Path(username): Path<String>,
) -> Result<Json<ProfileDto>, ApiErr> {
    let current_user_id = maybe_token.map(|tkn| tkn.id);

    let profile = get_profile_by_username(&db, &username, current_user_id)
        .await?
        .ok_or(ApiErr::UserNotExist)?;

    let profile_dto = ProfileDto { profile };
    Ok(Json(profile_dto))
}

/// Axum handler for setting logged user as follower of provided (by username) user.
/// Returns json object with profile on success, otherwise returns an `api error`.
pub async fn follow_user(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
    Path(username): Path<String>,
) -> Result<Json<ProfileDto>, ApiErr> {
    let current_user_id = token.id;

    let following_user: user::Model = get_user_by_username(&db, &username)
        .await?
        .ok_or(ApiErr::UserNotExist)?;

    let follower_model = follower::ActiveModel {
        user_id: Set(following_user.id),
        follower_id: Set(current_user_id),
    };

    create_follower(&db, follower_model).await?;

    let profile = get_profile_by_username(&db, &username, Some(current_user_id))
        .await?
        .ok_or(ApiErr::UserNotExist)?;

    let profile_dto = ProfileDto { profile };
    Ok(Json(profile_dto))
}

/// Axum handler for unfollow provided (by username) user.
/// Returns json object with profile on success, otherwise returns an `api error`.
pub async fn unfollow_user(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
    Path(username): Path<String>,
) -> Result<Json<ProfileDto>, ApiErr> {
    let current_user_id = token.id;

    let following_user: user::Model = get_user_by_username(&db, &username)
        .await?
        .ok_or(ApiErr::UserNotExist)?;

    let follower_model = follower::ActiveModel {
        user_id: Set(following_user.id),
        follower_id: Set(current_user_id),
    };

    delete_follower(&db, follower_model).await?;

    let profile = get_profile_by_username(&db, &username, Some(current_user_id))
        .await?
        .ok_or(ApiErr::UserNotExist)?;

    let profile_dto = ProfileDto { profile };
    Ok(Json(profile_dto))
}

/// Struct describing JSON object for profile routes requests. Contains user profile data.
#[derive(Debug, PartialEq, Serialize)]
pub struct ProfileDto {
    profile: Profile,
}

#[cfg(test)]
mod test_get_current_user {
    use super::{get_profile, ProfileDto};
    use crate::api::error::ApiErr;
    use crate::middleware::auth::Token;
    use crate::repo::user::Profile;
    use crate::tests::{
        Operation::{Create, Insert},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::extract::Path;
    use axum::{extract::State, Extension, Json};
    use dotenvy::dotenv;
    use entity::entities::user;

    #[tokio::test]
    async fn get_existing_user() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(2))
            .followers(Insert(vec![(1, 2)]))
            .build()
            .await?;
        let profile: user::Model = users.as_ref().unwrap().iter().next().unwrap().clone();
        let current_user: user::Model = users.unwrap().iter().cloned().last().unwrap();
        let token = Token {
            exp: 35,
            id: current_user.id.clone(),
        };

        // Actual test start
        let expected = ProfileDto {
            profile: Profile {
                username: profile.username.clone(),
                bio: profile.bio,
                image: profile.image,
                following: true,
            },
        };
        let result = get_profile(
            State(connection),
            Some(Extension(token)),
            Path(profile.username),
        )
        .await?;
        let Json(result) = result;

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn get_non_existing_user() -> Result<(), TestErr> {
        // dotenv().expect(".env file not found");
        let (connection, _) = TestDataBuilder::new().users(Create(1)).build().await?;

        let result = get_profile(
            State(connection),
            None,
            Path("not exist username".to_owned()),
        )
        .await;

        assert!(match result {
            Err(ApiErr::UserNotExist) => true,
            _ => false,
        });

        Ok(())
    }
}

#[cfg(test)]
mod test_follow_user {
    use super::{follow_user, ProfileDto};
    use crate::api::error::ApiErr;
    use crate::middleware::auth::Token;
    use crate::repo::user::Profile;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::{
        extract::{Path, State},
        Extension, Json,
    };
    use dotenvy::dotenv;
    use entity::entities::user;

    #[tokio::test]
    async fn follow_existing_user() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(2))
            .followers(Migration)
            .build()
            .await?;
        let profile: user::Model = users.as_ref().unwrap().iter().next().unwrap().clone();
        let current_user: user::Model = users.unwrap().iter().cloned().last().unwrap();
        let token = Token {
            exp: 35,
            id: current_user.id.clone(),
        };

        // Actual test start
        let expected = ProfileDto {
            profile: Profile {
                username: profile.username.clone(),
                bio: profile.bio,
                image: profile.image,
                following: true,
            },
        };
        let result =
            follow_user(State(connection), Extension(token), Path(profile.username)).await?;
        let Json(result) = result;

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn follow_non_existing_user() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .followers(Migration)
            .build()
            .await?;

        let current_user: user::Model = users.unwrap().into_iter().next().unwrap();
        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let result = follow_user(
            State(connection),
            Extension(token),
            Path("not exist username".to_owned()),
        )
        .await;

        assert!(match result {
            Err(ApiErr::UserNotExist) => true,
            _ => false,
        });

        Ok(())
    }
}

#[cfg(test)]
mod test_unfollow_user {
    use super::{unfollow_user, ProfileDto};
    use crate::api::error::ApiErr;
    use crate::middleware::auth::Token;
    use crate::repo::user::Profile;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::{
        extract::{Path, State},
        Extension, Json,
    };
    use dotenvy::dotenv;
    use entity::entities::user;

    #[tokio::test]
    async fn unfollow_existing_user() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(2))
            .followers(Insert(vec![(1, 2)]))
            .build()
            .await?;
        let profile: user::Model = users.as_ref().unwrap().iter().next().unwrap().clone();
        let current_user: user::Model = users.unwrap().iter().cloned().last().unwrap();
        let token = Token {
            exp: 35,
            id: current_user.id.clone(),
        };

        // Actual test start
        let expected = ProfileDto {
            profile: Profile {
                username: profile.username.clone(),
                bio: profile.bio,
                image: profile.image,
                following: false,
            },
        };
        let result =
            unfollow_user(State(connection), Extension(token), Path(profile.username)).await?;
        let Json(result) = result;

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn follow_non_existing_user() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .followers(Migration)
            .build()
            .await?;

        let current_user: user::Model = users.unwrap().into_iter().next().unwrap();
        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let result = unfollow_user(
            State(connection),
            Extension(token),
            Path("not exist username".to_owned()),
        )
        .await;

        assert!(match result {
            Err(ApiErr::UserNotExist) => true,
            _ => false,
        });

        Ok(())
    }
}
