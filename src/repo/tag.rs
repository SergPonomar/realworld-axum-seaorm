use entity::entities::{prelude::Tag, tag};
use migration::{Alias, Expr, OnConflict};
use sea_orm::{
    DatabaseConnection, DbErr, DeleteResult, EntityTrait, InsertResult, QueryFilter, QuerySelect,
    TryInsertResult,
};
use uuid::Uuid;

/// Insert `tags` for the provided `ActiveModel`s. Ignore models with existing tag names.
/// Returns `Inserted(InsertResult)` with last inserted id on success, otherwise
/// returns an `database error`.
/// Empty input produce `Empty` result.
/// See [`TryInsertResult`](https://docs.rs/sea-orm/latest/sea_orm/enum.TryInsertResult.html)
/// documentation for more details.
pub async fn create_tags(
    db: &DatabaseConnection,
    tags: Vec<tag::ActiveModel>,
) -> Result<TryInsertResult<InsertResult<tag::ActiveModel>>, DbErr> {
    // Filter empty tag names
    let tags = tags.into_iter().filter(|model| !model.is_empty());
    Tag::insert_many(tags)
        .on_conflict(
            OnConflict::column(tag::Column::TagName)
                .do_nothing()
                .to_owned(),
        )
        .on_empty_do_nothing()
        .exec(db)
        .await
}

/// Insert `tag` for the provided `ActiveModel`. Ignore models with existing tag names.
/// Returns `InsertResult` with last inserted id on success, otherwise
/// returns an `database error`.
/// Empty tag name produce error as not allowed on database level.
/// See [`InsertResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.InsertResult.html)
/// documentation for more details.
pub async fn insert_tag(
    db: &DatabaseConnection,
    tag: tag::ActiveModel,
) -> Result<InsertResult<tag::ActiveModel>, DbErr> {
    // TODO all fields in activemodel should be Set
    Tag::insert(tag).exec(db).await
}

/// Fetch `tag ids` for the provided `tag names`. Ignore not existing tag names.
/// Returns `list of tag names` on success, otherwise returns an `database error`.
/// Empty input produce empty result.
pub async fn get_tags_ids(db: &DatabaseConnection, tags: Vec<String>) -> Result<Vec<Uuid>, DbErr> {
    // Filter empty tag names
    let tags: Vec<String> = tags.into_iter().filter(|tg| !tg.is_empty()).collect();
    if tags.len() == 0 {
        return Ok(Vec::new());
    };
    Tag::find()
        .filter(Expr::expr(Expr::col(tag::Column::TagName).cast_as(Alias::new("text"))).is_in(tags))
        .into_tuple::<Uuid>()
        .all(db)
        .await
}

/// Fetch all `tag names` from database.
/// Returns `list of tag names` on success, otherwise returns an `database error`.
pub async fn get_tags(db: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
    Tag::find()
        .select_only()
        .column(tag::Column::TagName)
        .into_tuple::<String>()
        .all(db)
        .await
}

/// Delete all existing `tag records` from database.
/// Returns `DeleteResult` with affected rows count on success, otherwise
/// returns an `database error`.
/// See [`DeleteResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.DeleteResult.html)
/// documentation for more details.
pub async fn empty_tag_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    Tag::delete_many().exec(db).await
}

#[cfg(test)]
mod test_create_tags {
    use super::{create_tags, insert_tag};
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::tag;
    use sea_orm::{
        DbErr, Set,
        TryInsertResult::{Conflicted, Empty, Inserted},
    };
    use std::vec;
    use uuid::Uuid;
    const MIGRATION_NAME: &str = "m20231030_000004_create_tag_table";

    #[tokio::test]
    async fn insert_not_exist_data() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let models: Vec<tag::ActiveModel> = (1..=5)
            .map(|x| tag::ActiveModel {
                id: Set(Uuid::new_v4()),
                tag_name: Set(format!("test_insert_tag{x}")),
            })
            .collect();

        let last_id = models[models.len() - 1].id.clone().unwrap();

        let insert_result = create_tags(&connection, models).await?;

        assert!(match insert_result {
            Inserted(res) => res.last_insert_id == last_id,
            Conflicted => true,
            _ => false,
        });

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_id() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let models: Vec<tag::ActiveModel> = (1..=5)
            .map(|x| tag::ActiveModel {
                id: Set(Uuid::new_v4()),
                tag_name: Set(format!("test_insert_tag{x}")),
            })
            .collect();

        let id = models[1].id.clone();
        let model = tag::ActiveModel {
            id,
            tag_name: Set("test_insert_tag99".to_owned()),
        };

        insert_tag(&connection, model).await?;

        let try_insert = create_tags(&connection, models).await;

        assert!(try_insert.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: tag.id")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_tag_name() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let models: Vec<tag::ActiveModel> = (1..=5)
            .map(|x| tag::ActiveModel {
                id: Set(Uuid::new_v4()),
                tag_name: Set(format!("test_insert_tag{x}")),
            })
            .collect();

        let model = tag::ActiveModel {
            id: Set(Uuid::new_v4()),
            tag_name: Set("test_insert_tag2".to_owned()),
        };

        let last_id = models[models.len() - 1].id.clone().unwrap();
        insert_tag(&connection, model).await?;

        let insert_result = create_tags(&connection, models).await?;

        assert!(match insert_result {
            Inserted(res) => res.last_insert_id == last_id,
            Conflicted => true,
            _ => false,
        });

        Ok(())
    }

    #[tokio::test]
    async fn insert_empty_collection() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let models = vec![];

        let insert_result = create_tags(&connection, models).await?;

        assert!(match insert_result {
            Empty => true,
            _ => false,
        });

        Ok(())
    }
}

#[cfg(test)]
mod test_insert_tag {
    use super::insert_tag;
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::tag;
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;
    const MIGRATION_NAME: &str = "m20231030_000004_create_tag_table";

    #[tokio::test]
    async fn insert_not_exist_data() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let id = Uuid::new_v4();
        let model = tag::ActiveModel {
            id: Set(id),
            tag_name: Set("test_insert_tag1".to_owned()),
        };

        let insert_result = insert_tag(&connection, model).await?;
        assert_eq!(insert_result.last_insert_id, id);

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_id() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let id = Uuid::new_v4();
        let model1 = tag::ActiveModel {
            id: Set(id),
            tag_name: Set("test_insert_tag1".to_owned()),
        };

        let model2 = tag::ActiveModel {
            id: Set(id),
            tag_name: Set("test_insert_tag2".to_owned()),
        };

        insert_tag(&connection, model1).await?;
        let insert_result = insert_tag(&connection, model2).await;

        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: tag.id")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_tag_name() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let tag_name = Set("test_insert_tag".to_owned());
        let model1 = tag::ActiveModel {
            id: Set(Uuid::new_v4()),
            tag_name: tag_name.clone(),
        };

        let model2 = tag::ActiveModel {
            id: Set(Uuid::new_v4()),
            tag_name,
        };

        insert_tag(&connection, model1).await?;
        let insert_result = insert_tag(&connection, model2).await;

        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: tag.tag_name")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_empty_tag_name() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let model = tag::ActiveModel {
            id: Set(Uuid::new_v4()),
            tag_name: Set("".to_owned()),
        };

        let insert_result = insert_tag(&connection, model).await;

        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("CHECK constraint failed: tag_name")));

        Ok(())
    }
}

#[cfg(test)]
mod test_get_tags_ids {
    use super::{create_tags, get_tags_ids};
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::tag;
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;
    const MIGRATION_NAME: &str = "m20231030_000004_create_tag_table";

    #[tokio::test]
    async fn get_ids_of_existing_tags() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let models: Vec<tag::ActiveModel> = (1..=5)
            .map(|x| tag::ActiveModel {
                id: Set(Uuid::new_v4()),
                tag_name: Set(format!("test_insert_tag{x}")),
            })
            .collect();

        let input: Vec<String> = models
            .clone()
            .into_iter()
            .map(|model| model.tag_name.unwrap())
            .collect();

        let expected: Vec<Uuid> = models
            .clone()
            .into_iter()
            .map(|model| model.id.unwrap())
            .collect();

        create_tags(&connection, models).await?;

        let result = get_tags_ids(&connection, input).await?;

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn get_ids_of_non_existing_tags() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let input: Vec<String> = (1..=5).map(|x| format!("test_insert_tag{x}")).collect();
        let expected: Vec<Uuid> = Vec::new();
        let result = get_tags_ids(&connection, input).await?;

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn get_ids_of_empty_list() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let input: Vec<String> = Vec::new();
        let expected: Vec<Uuid> = Vec::new();
        let result = get_tags_ids(&connection, input).await?;

        assert_eq!(result, expected);

        Ok(())
    }
}

#[cfg(test)]
mod test_get_tags {
    use super::{create_tags, get_tags};
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::tag;
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;
    const MIGRATION_NAME: &str = "m20231030_000004_create_tag_table";

    #[tokio::test]
    async fn get_existing_tags() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let models: Vec<tag::ActiveModel> = (1..=5)
            .map(|x| tag::ActiveModel {
                id: Set(Uuid::new_v4()),
                tag_name: Set(format!("test_insert_tag{x}")),
            })
            .collect();

        let expected: Vec<String> = models
            .clone()
            .into_iter()
            .map(|model| model.tag_name.unwrap())
            .collect();

        create_tags(&connection, models).await?;

        let result = get_tags(&connection).await?;

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn get_empty_list() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let expected: Vec<String> = Vec::new();
        let result = get_tags(&connection).await?;

        assert_eq!(result, expected);

        Ok(())
    }
}

#[cfg(test)]
mod test_empty_tag_table {
    use super::{create_tags, empty_tag_table, get_tags};
    use crate::tests::{execute_migration, init_test_db_connection};
    use entity::entities::tag;
    use sea_orm::{DbErr, Set};
    use uuid::Uuid;
    const MIGRATION_NAME: &str = "m20231030_000004_create_tag_table";

    #[tokio::test]
    async fn delete_existing_tags() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let models: Vec<tag::ActiveModel> = (1..=5)
            .map(|x| tag::ActiveModel {
                id: Set(Uuid::new_v4()),
                tag_name: Set(format!("test_insert_tag{x}")),
            })
            .collect();

        create_tags(&connection, models.clone()).await?;

        let expected: Vec<String> = Vec::new();

        let delete_result = empty_tag_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, models.len() as u64);

        let result = get_tags(&connection).await?;

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn delete_empty_table() -> Result<(), DbErr> {
        let connection = init_test_db_connection().await?;
        execute_migration(&connection, MIGRATION_NAME).await?;

        let expected: Vec<String> = Vec::new();

        let delete_result = empty_tag_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, expected.len() as u64);

        let result = get_tags(&connection).await?;

        assert_eq!(result, expected);

        Ok(())
    }
}
