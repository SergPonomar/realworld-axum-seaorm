use entity::entities::{follower, prelude::Follower};
use sea_orm::{DatabaseConnection, DbErr, DeleteResult, EntityTrait, InsertResult};

/// Insert `follower` for the provided `ActiveModel`.
/// Returns `InsertResult` with last inserted id on success, otherwise
/// returns an `database error`.
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
#[cfg(feature = "seed")]
pub async fn empty_follower_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    Follower::delete_many().exec(db).await
}

#[cfg(test)]
mod test_create_follower {
    use super::create_follower;
    use crate::tests::{
        Operation::{Create, Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use entity::entities::{follower, prelude::Follower};
    use sea_orm::Set;
    use uuid::Uuid;

    #[tokio::test]
    async fn insert_not_exist_data() -> Result<(), TestErr> {
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(2))
            .followers(Migration)
            .build()
            .await?;

        let user_id = users.as_ref().unwrap()[0].id;
        let follower_id = users.as_ref().unwrap()[1].id;

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
    async fn insert_not_existing_follower() -> Result<(), TestErr> {
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Create(1))
            .followers(Migration)
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
    async fn insert_not_existing_user() -> Result<(), TestErr> {
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Create(1))
            .followers(Migration)
            .build()
            .await?;

        let follower_id = users.as_ref().unwrap()[0].id;

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
    async fn insert_existing_data() -> Result<(), TestErr> {
        let (connection, TestData { followers, .. }) = TestDataBuilder::new()
            .users(Insert(2))
            .followers(Insert(vec![(1, 2)]))
            .build()
            .await?;

        let actives =
            TestDataBuilder::activate_models::<Follower, follower::ActiveModel>(&followers);
        let model = actives.into_iter().next().unwrap();

        let insert_result = create_follower(&connection, model).await;
        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: follower.user_id, follower.follower_id")));

        Ok(())
    }
}

#[cfg(test)]
mod test_delete_follower {
    use super::delete_follower;
    use crate::tests::{Operation::Insert, TestData, TestDataBuilder, TestErr};
    use entity::entities::{follower, prelude::Follower};

    #[tokio::test]
    async fn delete_existing_data() -> Result<(), TestErr> {
        let (connection, TestData { followers, .. }) = TestDataBuilder::new()
            .users(Insert(2))
            .followers(Insert(vec![(1, 2)]))
            .build()
            .await?;
        let actives =
            TestDataBuilder::activate_models::<Follower, follower::ActiveModel>(&followers);
        let model = actives.into_iter().next().unwrap();

        let delete_result = delete_follower(&connection, model).await?;
        assert_eq!(delete_result.rows_affected, 1_u64);

        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "seed")]
mod test_empty_follower_table {
    use super::empty_follower_table;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestDataBuilder, TestErr,
    };
    use entity::entities::{follower, prelude::Follower};
    use sea_orm::EntityTrait;

    #[tokio::test]
    async fn delete_existing_followers() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(4))
            .followers(Insert(vec![(1, 2), (2, 3), (3, 4)]))
            .build()
            .await?;

        let delete_result = empty_follower_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 3_u64);

        let expected: Vec<follower::Model> = Vec::new();
        let result = Follower::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn delete_empty_table() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Migration)
            .followers(Migration)
            .build()
            .await?;

        let delete_result = empty_follower_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 0_u64);

        let expected: Vec<follower::Model> = Vec::new();
        let result = Follower::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }
}
