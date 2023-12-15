use super::error::ApiErr;
use crate::repo::tag::get_tags;
use axum::{extract::State, Json};
use sea_orm::DatabaseConnection;
use serde::Serialize;

/// Axum handler for fetch all existing `tag names`.
/// Returns json object with list of tag names on success, otherwise returns an `api error`.
pub async fn list_tags(State(db): State<DatabaseConnection>) -> Result<Json<TagsDto>, ApiErr> {
    let tags = get_tags(&db).await?;

    let tags_dto = TagsDto { tags };
    Ok(Json(tags_dto))
}

/// Struct describing JSON object, returned by handler. Contains list of tag names.
#[derive(Debug, Serialize, PartialEq)]
pub struct TagsDto {
    tags: Vec<String>,
}

#[cfg(test)]
mod test_list_tags {
    use super::{list_tags, TagsDto};
    use crate::tests::{
        Operation::{Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::{extract::State, Json};
    use std::vec;

    #[tokio::test]
    async fn get_existing_tags() -> Result<(), TestErr> {
        let (connection, TestData { tags, .. }) =
            TestDataBuilder::new().tags(Insert(5)).build().await?;
        let tags: Vec<String> = tags.unwrap().into_iter().map(|mdl| mdl.tag_name).collect();
        let expected = TagsDto { tags };

        let result = list_tags(State(connection)).await?;
        let Json(result) = result;

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn get_no_tags() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new().tags(Migration).build().await?;
        let tags: Vec<String> = vec![];
        let expected = TagsDto { tags };

        let result = list_tags(State(connection)).await?;
        let Json(result) = result;

        assert_eq!(result, expected);

        Ok(())
    }
}

#[cfg(test)]
mod test_errors {
    use super::list_tags;
    use crate::{
        api::error::ApiErr,
        tests::{TestDataBuilder, TestErr},
    };
    use axum::extract::State;

    #[tokio::test]
    async fn stale_connection() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new().build().await?;
        connection.clone().close().await?;

        let result = list_tags(State(connection)).await;

        assert!(match result {
            Err(ApiErr::DbErr(_)) => true,
            _ => false,
        });

        Ok(())
    }

    #[tokio::test]
    async fn no_migration() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new().build().await?;

        let result = list_tags(State(connection)).await;

        assert!(match result {
            Err(ApiErr::DbErr(_)) => true,
            _ => false,
        });

        Ok(())
    }
}
