use crate::middleware::auth::create_token;
use entity::entities::{
    follower,
    prelude::{Follower, User},
    user,
};
use migration::SimpleExpr;
use sea_orm::{
    prelude::Uuid, query::*, ColumnTrait, DatabaseConnection, DbErr, DeleteResult, EntityTrait,
    FromQueryResult, InsertResult, QueryFilter,
};
use serde::Serialize;

/// Fetch `user` for the provided `email`.
/// Returns optional `user` on success, otherwise returns an `database error`.
pub async fn get_user_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<user::Model>, DbErr> {
    User::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await
}

/// Fetch `user` for the provided `username`.
/// Returns optional `user` on success, otherwise returns an `database error`.
pub async fn get_user_by_username(
    db: &DatabaseConnection,
    username: &str,
) -> Result<Option<user::Model>, DbErr> {
    User::find()
        .filter(user::Column::Username.eq(username))
        .one(db)
        .await
}

/// Fetch `user` for the provided `id`.
/// Returns optional `user` on success, otherwise returns an `database error`.
pub async fn get_user_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<user::Model>, DbErr> {
    User::find_by_id(id).one(db).await
}

/// Fetch `user` with token for the provided `id`.
/// Returns optional `user` on success, otherwise returns an `database error`.
pub async fn get_user_with_token_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<UserWithToken>, DbErr> {
    User::find_by_id(id)
        .into_model::<UserWithToken>()
        .one(db)
        .await
}

/// Insert `user` for the provided `ActiveModel`. Reject models with existing username or email.
/// Returns `InsertResult` with last inserted id on success, otherwise
/// returns an `database error`.
/// Empty username, empty email produces error as not allowed on database level.
/// See [`InsertResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.InsertResult.html)
/// documentation for more details.
pub async fn create_user(
    db: &DatabaseConnection,
    user: user::ActiveModel,
) -> Result<InsertResult<user::ActiveModel>, DbErr> {
    User::insert(user).exec(db).await
}

/// Update `user` for the provided `ActiveModel`.
/// Returns `user` on success, otherwise returns an `database error`.
/// Reject models with non existing username or email.
pub async fn update_user(
    db: &DatabaseConnection,
    user: user::ActiveModel,
) -> Result<user::Model, DbErr> {
    User::update(user).exec(db).await
}

/// Fetch `profile` for the provided `username`. Optional identifier used
/// to determine whether the logged in user is a follower of the profile.
/// Returns optional `profile` on success, otherwise returns an `database error`.
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

/// Returns expression for determine whether the logged in
/// user is a follower of the profile.
pub fn author_followed_by_current_user(user_id: Option<Uuid>) -> SimpleExpr {
    match user_id {
        Some(id) => user::Column::Id.in_subquery(
            // find users followed by current user
            Follower::find()
                .select_only()
                .column(follower::Column::UserId)
                .filter(follower::Column::FollowerId.eq(id))
                .into_query(),
        ),
        None => false.into(),
    }
}

/// Delete all existing `user` records from database.
/// Returns `DeleteResult` with affected rows count on success, otherwise
/// returns an `database error`.
/// See [`DeleteResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.DeleteResult.html)
/// documentation for more details.
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
    pub username: String,
    pub bio: Option<String>,
    pub image: Option<String>,
    pub following: bool,
}

impl FromQueryResult for UserWithToken {
    fn from_query_result(res: &sea_orm::QueryResult, pre: &str) -> Result<Self, sea_orm::DbErr> {
        let id: Uuid = res.try_get(pre, "id")?;

        Ok(Self {
            token: create_token(&id).unwrap(),
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
            token: create_token(&model.id).unwrap(),
            email: model.email,
            username: model.username,
            bio: model.bio,
            image: model.image,
        }
    }
}

#[cfg(test)]
mod test_get_user_by_email {
    use super::{create_user, get_user_by_email};
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::user;
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;
    const USER_TABLE_MIGRATION: &str = "m20231030_000001_create_user_table";
    const USER_TABLE_ALTER: &str = "m20231112_000008_add_user_password";

    #[tokio::test]
    async fn get_existing_user() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let models: Vec<user::ActiveModel> = (0..5)
            .map(|x| user::ActiveModel {
                id: Set(Uuid::new_v4()),
                email: Set(format!("email{x}")),
                username: Set(format!("username{x}")),
                bio: Set(Some("bio".to_owned())),
                image: Set(Some("image".to_owned())),
                password: Set("password".to_owned()),
            })
            .collect();

        let expected_user_id = models[3].id.clone().unwrap();
        let expected = Some(user::Model {
            id: expected_user_id,
            email: "email3".to_owned(),
            username: "username3".to_owned(),
            bio: Some("bio".to_owned()),
            image: Some("image".to_owned()),
            password: "password".to_owned(),
        });

        for mdl in models {
            create_user(&connection, mdl).await?;
        }

        let result = get_user_by_email(&connection, "email3").await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn get_non_existing_user() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let result = get_user_by_email(&connection, "email3").await?;
        assert_eq!(result, None);

        Ok(())
    }
}

#[cfg(test)]
mod test_get_user_by_username {
    use super::{create_user, get_user_by_username};
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::user;
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;
    const USER_TABLE_MIGRATION: &str = "m20231030_000001_create_user_table";
    const USER_TABLE_ALTER: &str = "m20231112_000008_add_user_password";

    #[tokio::test]
    async fn get_existing_user() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let models: Vec<user::ActiveModel> = (0..5)
            .map(|x| user::ActiveModel {
                id: Set(Uuid::new_v4()),
                email: Set(format!("email{x}")),
                username: Set(format!("username{x}")),
                bio: Set(Some("bio".to_owned())),
                image: Set(Some("image".to_owned())),
                password: Set("password".to_owned()),
            })
            .collect();

        let expected_user_id = models[3].id.clone().unwrap();
        let expected = Some(user::Model {
            id: expected_user_id,
            email: "email3".to_owned(),
            username: "username3".to_owned(),
            bio: Some("bio".to_owned()),
            image: Some("image".to_owned()),
            password: "password".to_owned(),
        });

        for mdl in models {
            create_user(&connection, mdl).await?;
        }

        let result = get_user_by_username(&connection, "username3").await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn get_non_existing_user() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let result = get_user_by_username(&connection, "username3").await?;
        assert_eq!(result, None);

        Ok(())
    }
}

#[cfg(test)]
mod test_get_user_by_id {
    use super::{create_user, get_user_by_id};
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::user;
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;
    const USER_TABLE_MIGRATION: &str = "m20231030_000001_create_user_table";
    const USER_TABLE_ALTER: &str = "m20231112_000008_add_user_password";

    #[tokio::test]
    async fn get_existing_user() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let models: Vec<user::ActiveModel> = (0..5)
            .map(|x| user::ActiveModel {
                id: Set(Uuid::new_v4()),
                email: Set(format!("email{x}")),
                username: Set(format!("username{x}")),
                bio: Set(Some("bio".to_owned())),
                image: Set(Some("image".to_owned())),
                password: Set("password".to_owned()),
            })
            .collect();

        let expected_user_id = models[3].id.clone().unwrap();
        let expected = Some(user::Model {
            id: expected_user_id,
            email: "email3".to_owned(),
            username: "username3".to_owned(),
            bio: Some("bio".to_owned()),
            image: Some("image".to_owned()),
            password: "password".to_owned(),
        });

        for mdl in models {
            create_user(&connection, mdl).await?;
        }

        let result = get_user_by_id(&connection, expected_user_id).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn get_non_existing_user() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let result = get_user_by_id(&connection, Uuid::new_v4()).await?;
        assert_eq!(result, None);

        Ok(())
    }
}

#[cfg(test)]
mod test_get_user_with_token_by_id {
    use super::{create_user, get_user_with_token_by_id, UserWithToken};
    use crate::{
        middleware::auth::create_token,
        tests::{execute_migration, init_test_db_connection},
    };
    use dotenvy::dotenv;
    use entity::entities::user;
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;
    const USER_TABLE_MIGRATION: &str = "m20231030_000001_create_user_table";
    const USER_TABLE_ALTER: &str = "m20231112_000008_add_user_password";

    #[tokio::test]
    // Also test FromQueryResult implementation for UserWithToken
    async fn get_existing_user() -> Result<(), DbErr> {
        dotenv().expect(".env file not found");
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let models: Vec<user::ActiveModel> = (0..5)
            .map(|x| user::ActiveModel {
                id: Set(Uuid::new_v4()),
                email: Set(format!("email{x}")),
                username: Set(format!("username{x}")),
                bio: Set(Some("bio".to_owned())),
                image: Set(Some("image".to_owned())),
                password: Set("password".to_owned()),
            })
            .collect();

        let expected_user_id = models[3].id.clone().unwrap();
        let expected = Some(UserWithToken {
            token: create_token(&expected_user_id).unwrap(),
            email: "email3".to_owned(),
            username: "username3".to_owned(),
            bio: Some("bio".to_owned()),
            image: Some("image".to_owned()),
        });

        for mdl in models {
            create_user(&connection, mdl).await?;
        }

        let result = get_user_with_token_by_id(&connection, expected_user_id).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn get_non_existing_user() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let result = get_user_with_token_by_id(&connection, Uuid::new_v4()).await?;
        assert_eq!(result, None);

        Ok(())
    }
}

#[cfg(test)]
mod test_create_user {
    use super::create_user;
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::user;
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;
    const USER_TABLE_MIGRATION: &str = "m20231030_000001_create_user_table";
    const USER_TABLE_ALTER: &str = "m20231112_000008_add_user_password";

    #[tokio::test]
    async fn insert_not_exist_data() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let id = Uuid::new_v4();
        let model = user::ActiveModel {
            id: Set(id),
            email: Set("email".to_owned()),
            username: Set("username".to_owned()),
            bio: Set(Some("bio".to_owned())),
            image: Set(Some("image".to_owned())),
            password: Set("password".to_owned()),
        };

        let insert_result = create_user(&connection, model).await?;
        assert_eq!(insert_result.last_insert_id, id);

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_id() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let id = Uuid::new_v4();
        let model1 = user::ActiveModel {
            id: Set(id),
            email: Set("email1".to_owned()),
            username: Set("username1".to_owned()),
            bio: Set(Some("bio".to_owned())),
            image: Set(Some("image".to_owned())),
            password: Set("password".to_owned()),
        };

        let model2 = user::ActiveModel {
            id: Set(id),
            email: Set("email2".to_owned()),
            username: Set("username2".to_owned()),
            bio: Set(Some("bio".to_owned())),
            image: Set(Some("image".to_owned())),
            password: Set("password".to_owned()),
        };

        create_user(&connection, model1).await?;
        let insert_result = create_user(&connection, model2).await;

        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: user.id")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_email() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let email = Set("test_email".to_owned());
        let model1 = user::ActiveModel {
            id: Set(Uuid::new_v4()),
            email: email.clone(),
            username: Set("username1".to_owned()),
            bio: Set(Some("bio".to_owned())),
            image: Set(Some("image".to_owned())),
            password: Set("password".to_owned()),
        };

        let model2 = user::ActiveModel {
            id: Set(Uuid::new_v4()),
            email,
            username: Set("username2".to_owned()),
            bio: Set(Some("bio".to_owned())),
            image: Set(Some("image".to_owned())),
            password: Set("password".to_owned()),
        };

        create_user(&connection, model1).await?;
        let insert_result = create_user(&connection, model2).await;

        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: user.email")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_username() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let username = Set("test_username".to_owned());
        let model1 = user::ActiveModel {
            id: Set(Uuid::new_v4()),
            email: Set("email1".to_owned()),
            username: username.clone(),
            bio: Set(Some("bio".to_owned())),
            image: Set(Some("image".to_owned())),
            password: Set("password".to_owned()),
        };

        let model2 = user::ActiveModel {
            id: Set(Uuid::new_v4()),
            email: Set("email2".to_owned()),
            username,
            bio: Set(Some("bio".to_owned())),
            image: Set(Some("image".to_owned())),
            password: Set("password".to_owned()),
        };

        create_user(&connection, model1).await?;
        let insert_result = create_user(&connection, model2).await;

        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: user.username")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_empty_email() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let model = user::ActiveModel {
            id: Set(Uuid::new_v4()),
            email: Set("".to_owned()),
            username: Set("username".to_owned()),
            bio: Set(Some("bio".to_owned())),
            image: Set(Some("image".to_owned())),
            password: Set("password".to_owned()),
        };

        let insert_result = create_user(&connection, model).await;

        assert!(insert_result
            .is_err_and(|err| err.to_string().ends_with("CHECK constraint failed: email")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_empty_username() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let model = user::ActiveModel {
            id: Set(Uuid::new_v4()),
            email: Set("email".to_owned()),
            username: Set("".to_owned()),
            bio: Set(Some("bio".to_owned())),
            image: Set(Some("image".to_owned())),
            password: Set("password".to_owned()),
        };

        let insert_result = create_user(&connection, model).await;

        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("CHECK constraint failed: username")));

        Ok(())
    }
}

#[cfg(test)]
mod test_update_user {
    use super::{create_user, update_user};
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::user;
    use sea_orm::{ActiveModelTrait, DbErr, Set};
    use uuid::Uuid;
    const USER_TABLE_MIGRATION: &str = "m20231030_000001_create_user_table";
    const USER_TABLE_ALTER: &str = "m20231112_000008_add_user_password";

    #[tokio::test]
    async fn update_existing_data() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let models: Vec<user::ActiveModel> = (0..5)
            .map(|x| user::ActiveModel {
                id: Set(Uuid::new_v4()),
                email: Set(format!("email{x}")),
                username: Set(format!("username{x}")),
                bio: Set(Some("bio".to_owned())),
                image: Set(Some("image".to_owned())),
                password: Set("password".to_owned()),
            })
            .collect();

        let id = models[3].id.clone().unwrap();
        for mdl in models {
            create_user(&connection, mdl).await?;
        }

        let expected = user::Model {
            id,
            email: "updated email".to_owned(),
            username: "updated username".to_owned(),
            bio: Some("bio".to_owned()),
            image: Some("image".to_owned()),
            password: "password".to_owned(),
        };

        let update_model = user::ActiveModel::from(expected.clone()).reset_all();
        let updated = update_user(&connection, update_model).await?;
        assert_eq!(expected, updated);

        Ok(())
    }

    #[tokio::test]
    async fn update_not_existing_data() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let expected = user::Model {
            id: Uuid::new_v4(),
            email: "updated email".to_owned(),
            username: "updated username".to_owned(),
            bio: Some("bio".to_owned()),
            image: Some("image".to_owned()),
            password: "password".to_owned(),
        };

        let update_model = user::ActiveModel::from(expected).reset_all();
        let result = update_user(&connection, update_model).await;
        assert!(
            result.is_err_and(|err| err.to_string().ends_with("None of the records are updated"))
        );

        Ok(())
    }
}

#[cfg(test)]
mod test_get_profile_by_username {
    use super::{create_user, get_profile_by_username, Profile};
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::user;
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;
    const USER_TABLE_MIGRATION: &str = "m20231030_000001_create_user_table";
    const USER_TABLE_ALTER: &str = "m20231112_000008_add_user_password";
    const FOLLOWER_TABLE_MIGRATION: &str = "m20231101_000006_create_follower_table";

    #[tokio::test]
    async fn get_existing_profile() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;
        execute_migration(&connection, FOLLOWER_TABLE_MIGRATION).await?;

        let models: Vec<user::ActiveModel> = (0..5)
            .map(|x| user::ActiveModel {
                id: Set(Uuid::new_v4()),
                email: Set(format!("email{x}")),
                username: Set(format!("username{x}")),
                bio: Set(Some("bio".to_owned())),
                image: Set(Some("image".to_owned())),
                password: Set("password".to_owned()),
            })
            .collect();

        for mdl in models {
            create_user(&connection, mdl).await?;
        }

        let expected = Profile {
            username: "username3".to_owned(),
            bio: Some("bio".to_owned()),
            image: Some("image".to_owned()),
            following: false,
        };

        let result = get_profile_by_username(&connection, "username3", None).await?;
        assert_eq!(result, Some(expected));

        Ok(())
    }

    #[tokio::test]
    async fn get_non_existing_user() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let result = get_profile_by_username(&connection, "non existing username", None).await?;
        assert_eq!(result, None);

        Ok(())
    }
}

#[cfg(test)]
mod test_author_followed_by_current_user {
    use super::{create_user, get_profile_by_username, Profile};
    use crate::repo::follower::create_follower;
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::follower;

    use entity::entities::user;
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;
    const USER_TABLE_MIGRATION: &str = "m20231030_000001_create_user_table";
    const USER_TABLE_ALTER: &str = "m20231112_000008_add_user_password";
    const FOLLOWER_TABLE_MIGRATION: &str = "m20231101_000006_create_follower_table";

    #[tokio::test]
    async fn get_existing_profile_with_follower() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;
        execute_migration(&connection, FOLLOWER_TABLE_MIGRATION).await?;

        let user_id = Uuid::new_v4();
        let follower_id = Uuid::new_v4();

        let user1 = user::ActiveModel {
            id: Set(user_id),
            email: Set("email1".to_owned()),
            username: Set("username1".to_owned()),
            bio: Set(Some("bio".to_owned())),
            image: Set(Some("image".to_owned())),
            password: Set("password".to_owned()),
        };

        let user2 = user::ActiveModel {
            id: Set(follower_id),
            email: Set("email2".to_owned()),
            username: Set("username2".to_owned()),
            bio: Set(Some("bio".to_owned())),
            image: Set(Some("image".to_owned())),
            password: Set("password".to_owned()),
        };

        create_user(&connection, user1).await?;
        create_user(&connection, user2).await?;

        let model = follower::ActiveModel {
            user_id: Set(user_id),
            follower_id: Set(follower_id),
        };

        create_follower(&connection, model).await?;

        let expected = Profile {
            username: "username1".to_owned(),
            bio: Some("bio".to_owned()),
            image: Some("image".to_owned()),
            following: true,
        };
        // Used follower username
        let result = get_profile_by_username(&connection, "username1", Some(follower_id)).await?;
        assert_eq!(result, Some(expected));

        Ok(())
    }

    #[tokio::test]
    async fn get_existing_profile_wo_follower() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;
        execute_migration(&connection, FOLLOWER_TABLE_MIGRATION).await?;

        let user_id = Uuid::new_v4();
        let not_follower_id = Uuid::new_v4();

        let user1 = user::ActiveModel {
            id: Set(user_id),
            email: Set("email1".to_owned()),
            username: Set("username1".to_owned()),
            bio: Set(Some("bio".to_owned())),
            image: Set(Some("image".to_owned())),
            password: Set("password".to_owned()),
        };

        let user2 = user::ActiveModel {
            id: Set(not_follower_id),
            email: Set("email2".to_owned()),
            username: Set("username2".to_owned()),
            bio: Set(Some("bio".to_owned())),
            image: Set(Some("image".to_owned())),
            password: Set("password".to_owned()),
        };

        create_user(&connection, user1).await?;
        create_user(&connection, user2).await?;

        let expected = Profile {
            username: "username1".to_owned(),
            bio: Some("bio".to_owned()),
            image: Some("image".to_owned()),
            following: false,
        };
        // Used follower username
        let result =
            get_profile_by_username(&connection, "username1", Some(not_follower_id)).await?;
        assert_eq!(result, Some(expected));

        Ok(())
    }
}

#[cfg(test)]
mod test_empty_user_table {
    use super::{create_user, empty_user_table, User};
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::user;
    use sea_orm::EntityTrait;
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;
    const USER_TABLE_MIGRATION: &str = "m20231030_000001_create_user_table";
    const USER_TABLE_ALTER: &str = "m20231112_000008_add_user_password";

    #[tokio::test]
    async fn delete_existing_users() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let models: Vec<user::ActiveModel> = (0..5)
            .map(|x| user::ActiveModel {
                id: Set(Uuid::new_v4()),
                email: Set(format!("email{x}")),
                username: Set(format!("username{x}")),
                bio: Set(Some("bio".to_owned())),
                image: Set(Some("image".to_owned())),
                password: Set("password".to_owned()),
            })
            .collect();

        for mdl in models {
            create_user(&connection, mdl).await?;
        }

        let delete_result = empty_user_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 5_u64);

        let expected: Vec<user::Model> = Vec::new();
        let result = User::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn delete_empty_table() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;

        let delete_result = empty_user_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 0_u64);

        let expected: Vec<user::Model> = Vec::new();
        let result = User::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }
}

#[cfg(test)]
mod test_user_with_token_from_user {
    use super::UserWithToken;
    use crate::middleware::auth::create_token;
    use dotenvy::dotenv;
    use entity::entities::user;
    use sea_orm::prelude::Uuid;

    #[test]
    fn convert_from() {
        dotenv().expect(".env file not found");
        let id = Uuid::new_v4();
        let token = create_token(&id).unwrap();

        let user_with_token: UserWithToken = user::Model {
            id,
            email: "email".to_owned(),
            username: "username".to_owned(),
            bio: Some("bio".to_owned()),
            image: Some("image".to_owned()),
            password: "password".to_owned(),
        }
        .into();

        let expected = UserWithToken {
            token,
            email: "email".to_owned(),
            username: "username".to_owned(),
            bio: Some("bio".to_owned()),
            image: Some("image".to_owned()),
        };

        assert_eq!(user_with_token, expected);
    }

    #[test]
    fn convert_from_with_none() {
        dotenv().expect(".env file not found");
        let id = Uuid::new_v4();
        let token = create_token(&id).unwrap();

        let user_with_token: UserWithToken = user::Model {
            id,
            email: "email".to_owned(),
            username: "username".to_owned(),
            bio: None,
            image: None,
            password: "password".to_owned(),
        }
        .into();

        let expected = UserWithToken {
            token,
            email: "email".to_owned(),
            username: "username".to_owned(),
            bio: None,
            image: None,
        };

        assert_eq!(user_with_token, expected);
    }
}
