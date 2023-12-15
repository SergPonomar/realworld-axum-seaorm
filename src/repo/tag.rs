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

/// Insert `tag` for the provided `ActiveModel`. Reject models with existing tag names.
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
    use crate::tests::{Operation::Create, TestData, TestDataBuilder, TestErr};
    use entity::entities::{prelude::Tag, tag};
    use sea_orm::{
        Set,
        TryInsertResult::{Conflicted, Empty, Inserted},
    };
    use std::vec;
    use uuid::Uuid;

    #[tokio::test]
    async fn insert_not_exist_data() -> Result<(), TestErr> {
        let (connection, TestData { tags, .. }) =
            TestDataBuilder::new().tags(Create(5)).build().await?;
        let last_id = tags.as_ref().unwrap().iter().last().unwrap().id;
        let actives = TestDataBuilder::activate_models::<Tag, tag::ActiveModel>(&tags);
        let insert_result = create_tags(&connection, actives).await?;

        assert!(match insert_result {
            Inserted(res) => res.last_insert_id == last_id,
            Conflicted => true,
            _ => false,
        });

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_id() -> Result<(), TestErr> {
        let (connection, TestData { tags, .. }) =
            TestDataBuilder::new().tags(Create(5)).build().await?;
        let actives = TestDataBuilder::activate_models::<Tag, tag::ActiveModel>(&tags);

        let id = tags.as_ref().unwrap().iter().nth(1).unwrap().id;
        let model = tag::ActiveModel {
            id: Set(id),
            tag_name: Set("tag_name99".to_owned()),
        };

        insert_tag(&connection, model).await?;
        let try_insert = create_tags(&connection, actives).await;
        assert!(try_insert.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: tag.id")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_tag_name() -> Result<(), TestErr> {
        let (connection, TestData { tags, .. }) =
            TestDataBuilder::new().tags(Create(5)).build().await?;
        let actives = TestDataBuilder::activate_models::<Tag, tag::ActiveModel>(&tags);

        let model = tag::ActiveModel {
            id: Set(Uuid::new_v4()),
            tag_name: Set("tag_name2".to_owned()),
        };

        let last_id = tags.as_ref().unwrap().iter().last().unwrap().id;
        insert_tag(&connection, model).await?;

        let insert_result = create_tags(&connection, actives).await?;

        assert!(match insert_result {
            Inserted(res) => res.last_insert_id == last_id,
            Conflicted => true,
            _ => false,
        });

        Ok(())
    }

    #[tokio::test]
    async fn insert_empty_collection() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new().tags(Create(5)).build().await?;
        let actives = vec![];
        let insert_result = create_tags(&connection, actives).await?;

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
    use crate::tests::{
        Operation::{Create, Insert},
        TestData, TestDataBuilder, TestErr,
    };
    use entity::entities::tag;
    use sea_orm::Set;
    use uuid::Uuid;

    #[tokio::test]
    async fn insert_not_exist_data() -> Result<(), TestErr> {
        let (connection, TestData { tags, .. }) =
            TestDataBuilder::new().tags(Create(1)).build().await?;
        let tag = tags.unwrap().into_iter().next().unwrap();
        let id = (&tag.id).clone();

        let insert_result = insert_tag(&connection, tag.into()).await?;
        assert_eq!(insert_result.last_insert_id, id);

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_id() -> Result<(), TestErr> {
        let (connection, TestData { tags, .. }) =
            TestDataBuilder::new().tags(Insert(1)).build().await?;
        let id = tags.unwrap()[0].id;

        let model2 = tag::ActiveModel {
            id: Set(id),
            tag_name: Set("test_insert_tag2".to_owned()),
        };

        let insert_result = insert_tag(&connection, model2).await;

        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: tag.id")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_tag_name() -> Result<(), TestErr> {
        let (connection, TestData { tags, .. }) =
            TestDataBuilder::new().tags(Insert(1)).build().await?;
        let tag_name = &tags.unwrap()[0].tag_name;

        let model2 = tag::ActiveModel {
            id: Set(Uuid::new_v4()),
            tag_name: Set(tag_name.into()),
        };

        let insert_result = insert_tag(&connection, model2).await;

        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: tag.tag_name")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_empty_tag_name() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new().tags(Create(1)).build().await?;
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
    use super::{create_tags, get_tags_ids, Tag};
    use crate::tests::{
        Operation::{Create, Insert},
        TestData, TestDataBuilder, TestErr,
    };
    use entity::entities::tag;
    use uuid::Uuid;

    #[tokio::test]
    async fn get_ids_of_existing_tags() -> Result<(), TestErr> {
        let (connection, TestData { tags, .. }) =
            TestDataBuilder::new().tags(Insert(5)).build().await?;

        let input: Vec<String> = tags
            .as_ref()
            .unwrap()
            .iter()
            .cloned()
            .map(|model| model.tag_name)
            .collect();

        let expected: Vec<Uuid> = tags
            .as_ref()
            .unwrap()
            .iter()
            .cloned()
            .map(|model| model.id)
            .collect();

        let actives = TestDataBuilder::activate_models::<Tag, tag::ActiveModel>(&tags);
        create_tags(&connection, actives).await?;

        let result = get_tags_ids(&connection, input).await?;

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn get_ids_of_non_existing_tags() -> Result<(), TestErr> {
        let (connection, TestData { tags, .. }) =
            TestDataBuilder::new().tags(Create(5)).build().await?;
        let input: Vec<String> = tags
            .unwrap()
            .into_iter()
            .map(|model| model.tag_name)
            .collect();

        let expected: Vec<Uuid> = Vec::new();
        let result = get_tags_ids(&connection, input).await?;

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn get_ids_of_empty_list() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new().tags(Create(1)).build().await?;
        let input: Vec<String> = Vec::new();
        let expected: Vec<Uuid> = Vec::new();
        let result = get_tags_ids(&connection, input).await?;

        assert_eq!(result, expected);

        Ok(())
    }
}

#[cfg(test)]
mod test_get_tags {
    use super::get_tags;
    use crate::tests::{
        Operation::{Create, Insert},
        TestData, TestDataBuilder, TestErr,
    };

    #[tokio::test]
    async fn get_existing_tags() -> Result<(), TestErr> {
        let (connection, TestData { tags, .. }) =
            TestDataBuilder::new().tags(Insert(5)).build().await?;
        let expected: Vec<String> = tags
            .unwrap()
            .into_iter()
            .map(|model| model.tag_name)
            .collect();

        let result = get_tags(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn get_empty_list() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new().tags(Create(1)).build().await?;
        let expected: Vec<String> = Vec::new();
        let result = get_tags(&connection).await?;

        assert_eq!(result, expected);

        Ok(())
    }
}

#[cfg(test)]
mod test_empty_tag_table {
    use super::{empty_tag_table, get_tags};
    use crate::tests::{
        Operation::{Create, Insert},
        TestData, TestDataBuilder, TestErr,
    };

    #[tokio::test]
    async fn delete_existing_tags() -> Result<(), TestErr> {
        let (connection, TestData { tags, .. }) =
            TestDataBuilder::new().tags(Insert(5)).build().await?;
        let expected: Vec<String> = Vec::new();

        let delete_result = empty_tag_table(&connection).await?;
        let result = get_tags(&connection).await?;
        assert_eq!(delete_result.rows_affected, tags.unwrap().len() as u64);
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn delete_empty_table() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new().tags(Create(1)).build().await?;
        let expected: Vec<String> = Vec::new();

        let delete_result = empty_tag_table(&connection).await?;
        let result = get_tags(&connection).await?;
        assert_eq!(delete_result.rows_affected, expected.len() as u64);
        assert_eq!(result, expected);

        Ok(())
    }
}
