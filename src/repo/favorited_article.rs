use entity::entities::{favorited_article, prelude::FavoritedArticle};
use sea_orm::{DatabaseConnection, DbErr, DeleteResult, EntityTrait, InsertResult};

/// Insert `favorite article` for the provided `ActiveModel`.
/// Returns `InsertResult` with last inserted id on success, otherwise
/// returns an `database error`.
/// Empty input produce error as not allowed on database level.
/// See [`InsertResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.InsertResult.html)
/// documentation for more details.
pub async fn favorite_article(
    db: &DatabaseConnection,
    favorite_article: favorited_article::ActiveModel,
) -> Result<InsertResult<favorited_article::ActiveModel>, DbErr> {
    FavoritedArticle::insert(favorite_article).exec(db).await
}

/// Delete `favorite article` for the provided `ActiveModel`.
/// Returns `DeleteResult` with affected rows count on success, otherwise
/// returns an `database error`.
/// See [`DeleteResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.DeleteResult.html)
/// documentation for more details.
pub async fn unfavorite_article(
    db: &DatabaseConnection,
    favorite_article: favorited_article::ActiveModel,
) -> Result<DeleteResult, DbErr> {
    FavoritedArticle::delete(favorite_article).exec(db).await
}

/// Delete all existing `favorited article` records from database.
/// Returns `DeleteResult` with affected rows count on success, otherwise
/// returns an `database error`.
/// See [`DeleteResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.DeleteResult.html)
/// documentation for more details.
pub async fn empty_favorited_article_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    FavoritedArticle::delete_many().exec(db).await
}

#[cfg(test)]
mod test_favorite_article {
    use super::favorite_article;
    use crate::tests::{Operation::Insert, TestData, TestDataBuilder, TestErr};
    use entity::entities::{favorited_article, prelude::FavoritedArticle};
    use sea_orm::Set;
    use std::vec;
    use uuid::Uuid;

    #[tokio::test]
    async fn insert_not_exist_data() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                articles, users, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Insert(vec![(1, 1), (2, 2)]))
            .build()
            .await?;

        let article_id = articles.as_ref().unwrap()[1].id;
        let user_id = users.as_ref().unwrap()[2].id;

        let model = favorited_article::ActiveModel {
            article_id: Set(article_id),
            user_id: Set(user_id),
        };

        let last_id = (article_id, user_id);
        let insert_result = favorite_article(&connection, model).await?;
        assert_eq!(insert_result.last_insert_id, last_id);

        Ok(())
    }

    #[tokio::test]
    async fn insert_not_existing_article() -> Result<(), TestErr> {
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Insert(vec![(1, 1), (2, 2)]))
            .build()
            .await?;

        let user_id = users.as_ref().unwrap()[2].id;

        let model = favorited_article::ActiveModel {
            article_id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
        };

        let insert_result = favorite_article(&connection, model).await;
        assert!(insert_result
            .is_err_and(|err| err.to_string().ends_with("FOREIGN KEY constraint failed")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_not_existing_user() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Insert(vec![(1, 1), (2, 2)]))
            .build()
            .await?;

        let article_id = articles.as_ref().unwrap()[1].id;

        let model = favorited_article::ActiveModel {
            article_id: Set(article_id),
            user_id: Set(Uuid::new_v4()),
        };

        let insert_result = favorite_article(&connection, model).await;
        assert!(insert_result
            .is_err_and(|err| err.to_string().ends_with("FOREIGN KEY constraint failed")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_data() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                favorited_articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Insert(vec![(1, 1)]))
            .build()
            .await?;

        let actives = TestDataBuilder::activate_models::<
            FavoritedArticle,
            favorited_article::ActiveModel,
        >(&favorited_articles);
        let model = actives.into_iter().next().unwrap();

        let insert_result = favorite_article(&connection, model).await;
        assert!(insert_result.is_err_and(|err| err.to_string().ends_with(
            "UNIQUE constraint failed: favorited_article.article_id, favorited_article.user_id"
        )));

        Ok(())
    }
}

#[cfg(test)]
mod test_unfavorite_article {
    use super::unfavorite_article;
    use crate::tests::{Operation::Insert, TestData, TestDataBuilder, TestErr};
    use entity::entities::{favorited_article, prelude::FavoritedArticle};

    #[tokio::test]
    async fn delete_existing_data() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                favorited_articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Insert(vec![(1, 1)]))
            .build()
            .await?;
        let actives = TestDataBuilder::activate_models::<
            FavoritedArticle,
            favorited_article::ActiveModel,
        >(&favorited_articles);
        let model = actives.into_iter().next().unwrap();

        let delete_result = unfavorite_article(&connection, model).await?;
        assert_eq!(delete_result.rows_affected, 1_u64);

        Ok(())
    }
}

#[cfg(test)]
mod test_empty_favorited_article_table {
    use super::empty_favorited_article_table;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestDataBuilder, TestErr,
    };
    use entity::entities::{favorited_article, prelude::FavoritedArticle};
    use sea_orm::EntityTrait;

    #[tokio::test]
    async fn delete_existing_article_tags() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Insert(vec![(1, 1), (2, 2)]))
            .build()
            .await?;

        let delete_result = empty_favorited_article_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 2_u64);

        let expected: Vec<favorited_article::Model> = Vec::new();
        let result = FavoritedArticle::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn delete_empty_table() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Migration)
            .articles(Migration)
            .tags(Migration)
            .favorited_articles(Migration)
            .build()
            .await?;

        let delete_result = empty_favorited_article_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 0_u64);

        let expected: Vec<favorited_article::Model> = Vec::new();
        let result = FavoritedArticle::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }
}
