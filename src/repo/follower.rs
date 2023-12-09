use entity::entities::{follower, prelude::Follower};
use sea_orm::{DatabaseConnection, DbErr, DeleteResult, EntityTrait, InsertResult};

/// Insert `follower` for the provided `ActiveModel`.
/// Returns `InsertResult` with last inserted id on success, otherwise
/// returns an `database error`.
/// Empty input produce error as not allowed on database level.
/// See [`InsertResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.InsertResult.html)
/// documentation for more details.
pub async fn create_follower(
    db: &DatabaseConnection,
    follower: follower::ActiveModel,
) -> Result<InsertResult<follower::ActiveModel>, DbErr> {
    Follower::insert(follower).exec(db).await
}

/// Delete `follower` for the provided `ActiveModel`.
/// Returns `DeleteResult` with affected rows count on success, otherwise
/// returns an `database error`.
/// See [`DeleteResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.DeleteResult.html)
/// documentation for more details.
pub async fn delete_follower(
    db: &DatabaseConnection,
    follower: follower::ActiveModel,
) -> Result<DeleteResult, DbErr> {
    Follower::delete(follower).exec(db).await
}

/// Delete all existing `follower records` from database.
/// Returns `DeleteResult` with affected rows count on success, otherwise
/// returns an `database error`.
/// See [`DeleteResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.DeleteResult.html)
/// documentation for more details.
pub async fn empty_follower_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    Follower::delete_many().exec(db).await
}

#[cfg(test)]
mod test_create_follower {
    use super::create_follower;
    use crate::repo::user::create_user;
    use crate::tests::BldrErr;
    use crate::tests::TestData;
    use crate::tests::{execute_migration, init_test_db_connection};
    use crate::tests::{RelUserFollower, TestDataBuilder};
    use entity::entities::{follower, user};
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;

    const USER_TABLE_MIGRATION: &str = "m20231030_000001_create_user_table";
    const USER_TABLE_ALTER: &str = "m20231112_000008_add_user_password";
    const FOLLOWER_TABLE_MIGRATION: &str = "m20231101_000006_create_follower_table";

    #[tokio::test]
    async fn insert_not_exist_data() -> Result<(), BldrErr> {
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(2)
            .followers(RelUserFollower(vec![(1, 2)]))
            .build()
            .await?;

        let user_id = users.as_ref().unwrap()[1].id;
        let follower_id = users.as_ref().unwrap()[0].id;

        let model = follower::ActiveModel {
            user_id: Set(user_id),
            follower_id: Set(follower_id),
        };

        let last_id = (user_id, follower_id);
        let insert_result = create_follower(&connection, model).await?;
        assert_eq!(insert_result.last_insert_id, last_id);

        Ok(())
    }

    #[tokio::test]
    async fn insert_not_existing_follower() -> Result<(), BldrErr> {
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(2)
            .followers(RelUserFollower(vec![(1, 2)]))
            .build()
            .await?;

        let user_id = users.as_ref().unwrap()[0].id;

        let model = follower::ActiveModel {
            user_id: Set(user_id),
            follower_id: Set(Uuid::new_v4()),
        };

        let insert_result = create_follower(&connection, model).await;
        assert!(insert_result
            .is_err_and(|err| err.to_string().ends_with("FOREIGN KEY constraint failed")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_not_existing_user() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;
        execute_migration(&connection, FOLLOWER_TABLE_MIGRATION).await?;

        let follower_id = Uuid::new_v4();

        let user1 = user::ActiveModel {
            id: Set(follower_id),
            email: Set("email1".to_owned()),
            username: Set("username1".to_owned()),
            password: Set("password_hash".to_owned()),
            ..Default::default()
        };

        create_user(&connection, user1).await?;

        let model = follower::ActiveModel {
            user_id: Set(Uuid::new_v4()),
            follower_id: Set(follower_id),
        };

        let insert_result = create_follower(&connection, model).await;
        assert!(insert_result
            .is_err_and(|err| err.to_string().ends_with("FOREIGN KEY constraint failed")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_data() -> Result<(), DbErr> {
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
            password: Set("password_hash".to_owned()),
            ..Default::default()
        };

        let user2 = user::ActiveModel {
            id: Set(follower_id),
            email: Set("email2".to_owned()),
            username: Set("username2".to_owned()),
            password: Set("password_hash".to_owned()),
            ..Default::default()
        };

        create_user(&connection, user1).await?;
        create_user(&connection, user2).await?;

        let model = follower::ActiveModel {
            user_id: Set(user_id),
            follower_id: Set(follower_id),
        };

        create_follower(&connection, model.clone()).await?;
        let insert_result = create_follower(&connection, model).await;
        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: follower.user_id, follower.follower_id")));

        Ok(())
    }
}

#[cfg(test)]
mod test_delete_follower {
    use super::{create_follower, delete_follower};
    use crate::repo::user::create_user;
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::{follower, user};
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;

    const USER_TABLE_MIGRATION: &str = "m20231030_000001_create_user_table";
    const USER_TABLE_ALTER: &str = "m20231112_000008_add_user_password";
    const FOLLOWER_TABLE_MIGRATION: &str = "m20231101_000006_create_follower_table";

    #[tokio::test]
    async fn delete_existing_data() -> Result<(), DbErr> {
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
            password: Set("password_hash".to_owned()),
            ..Default::default()
        };

        let user2 = user::ActiveModel {
            id: Set(follower_id),
            email: Set("email2".to_owned()),
            username: Set("username2".to_owned()),
            password: Set("password_hash".to_owned()),
            ..Default::default()
        };

        let model = follower::ActiveModel {
            user_id: Set(user_id),
            follower_id: Set(follower_id),
        };

        create_user(&connection, user1).await?;
        create_user(&connection, user2).await?;
        create_follower(&connection, model.clone()).await?;

        let delete_result = delete_follower(&connection, model).await?;
        assert_eq!(delete_result.rows_affected, 1_u64);

        Ok(())
    }
}

#[cfg(test)]
mod test_empty_follower_table {
    use super::{create_follower, empty_follower_table};
    use crate::repo::user::create_user;
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::{follower, prelude::Follower, user};
    use sea_orm::EntityTrait;
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;

    const USER_TABLE_MIGRATION: &str = "m20231030_000001_create_user_table";
    const USER_TABLE_ALTER: &str = "m20231112_000008_add_user_password";
    const FOLLOWER_TABLE_MIGRATION: &str = "m20231101_000006_create_follower_table";

    #[tokio::test]
    async fn delete_existing_followers() -> Result<(), DbErr> {
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
            password: Set("password_hash".to_owned()),
            ..Default::default()
        };

        let user2 = user::ActiveModel {
            id: Set(follower_id),
            email: Set("email2".to_owned()),
            username: Set("username2".to_owned()),
            password: Set("password_hash".to_owned()),
            ..Default::default()
        };

        create_user(&connection, user1).await?;
        create_user(&connection, user2).await?;

        let model = follower::ActiveModel {
            user_id: Set(user_id),
            follower_id: Set(follower_id),
        };

        create_follower(&connection, model.clone()).await?;

        let delete_result = empty_follower_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 1_u64);

        let expected: Vec<follower::Model> = Vec::new();
        let result = Follower::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn delete_empty_table() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, USER_TABLE_MIGRATION).await?;
        execute_migration(&connection, USER_TABLE_ALTER).await?;
        execute_migration(&connection, FOLLOWER_TABLE_MIGRATION).await?;

        let delete_result = empty_follower_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 0_u64);

        let expected: Vec<follower::Model> = Vec::new();
        let result = Follower::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }
}
