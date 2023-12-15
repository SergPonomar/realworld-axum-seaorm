use entity::entities::{article_tag, prelude::ArticleTag, tag};
use sea_orm::{
    query::*, ColumnTrait, DatabaseConnection, DbErr, DeleteResult, EntityTrait, InsertResult,
    RelationTrait, TryInsertResult,
};
use uuid::Uuid;

/// Insert `article tags` for the provided `ActiveModel`.
/// Returns `TryInsertResult` on success, otherwise returns an `database error`.
/// See [`TryInsertResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.TryInsertResult.html)
/// documentation for more details.
pub async fn create_article_tags(
    db: &DatabaseConnection,
    article_tags: Vec<article_tag::ActiveModel>,
) -> Result<TryInsertResult<InsertResult<article_tag::ActiveModel>>, DbErr> {
    ArticleTag::insert_many(article_tags)
        .on_empty_do_nothing()
        .exec(db)
        .await
}

/// Insert `article tag` for the provided `ActiveModel`.
/// Returns `InsertResult` with last inserted id on success, otherwise
/// returns an `database error`.
/// Empty input produce error as not allowed on database level.
/// See [`InsertResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.InsertResult.html)
/// documentation for more details.
pub async fn insert_article_tag(
    db: &DatabaseConnection,
    article_tag: article_tag::ActiveModel,
) -> Result<InsertResult<article_tag::ActiveModel>, DbErr> {
    ArticleTag::insert(article_tag).exec(db).await
}

/// Fetch `tag names` for the provided article.
/// Returns `list of tag names` on success, otherwise returns an `database error`.
pub async fn get_article_tags(
    db: &DatabaseConnection,
    article_id: Uuid,
) -> Result<Vec<String>, DbErr> {
    ArticleTag::find()
        .join(JoinType::LeftJoin, article_tag::Relation::Tag.def())
        .filter(article_tag::Column::ArticleId.eq(article_id))
        .select_only()
        .column(tag::Column::TagName)
        .group_by(tag::Column::TagName)
        .into_tuple::<String>()
        .all(db)
        .await
}

/// Delete all existing `article tag records` from database.
/// Returns `DeleteResult` with affected rows count on success, otherwise
/// returns an `database error`.
/// See [`DeleteResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.DeleteResult.html)
/// documentation for more details.
pub async fn empty_article_tag_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    ArticleTag::delete_many().exec(db).await
}

#[cfg(test)]
mod test_create_article_tags {
    use super::{create_article_tags, insert_article_tag};
    use crate::tests::{
        Operation::{Create, Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use entity::entities::{article_tag, prelude::ArticleTag};
    use sea_orm::{
        Set,
        TryInsertResult::{Conflicted, Empty, Inserted},
    };
    use std::vec;

    #[tokio::test]
    async fn insert_not_exist_data() -> Result<(), TestErr> {
        let (connection, TestData { article_tags, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .tags(Insert(3))
            .article_tags(Create(vec![(1, 1), (2, 1), (2, 2), (3, 3)]))
            .build()
            .await?;
        let last_article_tag = article_tags.as_ref().unwrap().iter().last().unwrap();
        let last_id = (last_article_tag.article_id, last_article_tag.tag_id);
        let actives =
            TestDataBuilder::activate_models::<ArticleTag, article_tag::ActiveModel>(&article_tags);
        let insert_result = create_article_tags(&connection, actives).await?;

        assert!(match insert_result {
            Inserted(res) => res.last_insert_id == last_id,
            Conflicted => true,
            _ => false,
        });

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_article_tag() -> Result<(), TestErr> {
        let (connection, TestData { article_tags, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .tags(Insert(3))
            .article_tags(Create(vec![(1, 1), (2, 1), (2, 2), (3, 3)]))
            .build()
            .await?;
        let actives =
            TestDataBuilder::activate_models::<ArticleTag, article_tag::ActiveModel>(&article_tags);

        let existing = article_tags.as_ref().unwrap().iter().nth(1).unwrap();
        let model = article_tag::ActiveModel {
            article_id: Set(existing.article_id),
            tag_id: Set(existing.tag_id),
        };

        insert_article_tag(&connection, model).await?;
        let try_insert = create_article_tags(&connection, actives).await;

        assert!(try_insert.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: article_tag.article_id, article_tag.tag_id")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_empty_collection() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Migration)
            .articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;
        let actives = vec![];
        let insert_result = create_article_tags(&connection, actives).await?;

        assert!(match insert_result {
            Empty => true,
            _ => false,
        });

        Ok(())
    }
}

#[cfg(test)]
mod test_insert_article_tag {
    use super::insert_article_tag;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use entity::entities::{article_tag, prelude::ArticleTag};
    use sea_orm::Set;
    use std::vec;
    use uuid::Uuid;

    #[tokio::test]
    async fn insert_not_exist_data() -> Result<(), TestErr> {
        let (connection, TestData { articles, tags, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .tags(Insert(3))
            .article_tags(Insert(vec![(1, 1), (2, 1), (2, 2), (3, 3)]))
            .build()
            .await?;

        let article_id = articles.as_ref().unwrap()[1].id;
        let tag_id = tags.as_ref().unwrap()[2].id;

        let model = article_tag::ActiveModel {
            article_id: Set(article_id),
            tag_id: Set(tag_id),
        };

        let last_id = (article_id, tag_id);
        let insert_result = insert_article_tag(&connection, model).await?;
        assert_eq!(insert_result.last_insert_id, last_id);

        Ok(())
    }

    #[tokio::test]
    async fn insert_not_existing_article() -> Result<(), TestErr> {
        let (connection, TestData { tags, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .tags(Insert(3))
            .article_tags(Migration)
            .build()
            .await?;

        let tag_id = tags.as_ref().unwrap()[2].id;

        let model = article_tag::ActiveModel {
            article_id: Set(Uuid::new_v4()),
            tag_id: Set(tag_id),
        };

        let insert_result = insert_article_tag(&connection, model).await;
        assert!(insert_result
            .is_err_and(|err| err.to_string().ends_with("FOREIGN KEY constraint failed")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_not_existing_tag() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .tags(Insert(3))
            .article_tags(Migration)
            .build()
            .await?;

        let article_id = articles.as_ref().unwrap()[1].id;

        let model = article_tag::ActiveModel {
            article_id: Set(article_id),
            tag_id: Set(Uuid::new_v4()),
        };

        let insert_result = insert_article_tag(&connection, model).await;
        assert!(insert_result
            .is_err_and(|err| err.to_string().ends_with("FOREIGN KEY constraint failed")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_data() -> Result<(), TestErr> {
        let (connection, TestData { article_tags, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .tags(Insert(1))
            .article_tags(Insert(vec![(1, 1)]))
            .build()
            .await?;

        let actives =
            TestDataBuilder::activate_models::<ArticleTag, article_tag::ActiveModel>(&article_tags);
        let model = actives.into_iter().next().unwrap();

        let insert_result = insert_article_tag(&connection, model).await;
        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: article_tag.article_id, article_tag.tag_id")));

        Ok(())
    }
}

#[cfg(test)]
mod test_get_article_tags {
    use super::get_article_tags;
    use crate::tests::{Operation::Insert, TestData, TestDataBuilder, TestErr};
    use std::vec;
    use uuid::Uuid;

    #[tokio::test]
    async fn tags_of_existing_article() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .tags(Insert(3))
            .article_tags(Insert(vec![(1, 1), (2, 1), (2, 2), (3, 3)]))
            .build()
            .await?;

        let article_id = articles.as_ref().unwrap()[1].id;
        let result = get_article_tags(&connection, article_id).await?;
        let expected = vec!["tag_name1".to_owned(), "tag_name2".to_owned()];
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn article_with_no_tags() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .tags(Insert(3))
            .article_tags(Insert(vec![(1, 1), (2, 1), (2, 2), (3, 3)]))
            .build()
            .await?;

        let article_id = articles.as_ref().unwrap()[4].id;
        let result = get_article_tags(&connection, article_id).await?;
        let expected: Vec<String> = vec![];
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn tags_of_non_existing_article() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .tags(Insert(3))
            .article_tags(Insert(vec![(1, 1), (2, 1), (2, 2), (3, 3)]))
            .build()
            .await?;

        let article_id = Uuid::new_v4();
        let result = get_article_tags(&connection, article_id).await?;
        let expected: Vec<String> = vec![];
        assert_eq!(result, expected);

        Ok(())
    }
}

#[cfg(test)]
mod test_empty_article_tag_table {
    use super::empty_article_tag_table;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestDataBuilder, TestErr,
    };
    use entity::entities::{article_tag, prelude::ArticleTag};
    use sea_orm::EntityTrait;

    #[tokio::test]
    async fn delete_existing_article_tags() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .tags(Insert(3))
            .article_tags(Insert(vec![(1, 1), (2, 1), (2, 2), (3, 3)]))
            .build()
            .await?;

        let delete_result = empty_article_tag_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 4_u64);

        let expected: Vec<article_tag::Model> = Vec::new();
        let result = ArticleTag::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn delete_empty_table() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Migration)
            .articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let delete_result = empty_article_tag_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 0_u64);

        let expected: Vec<article_tag::Model> = Vec::new();
        let result = ArticleTag::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }
}
