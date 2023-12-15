use super::user::{author_followed_by_current_user, Profile};
use entity::entities::{
    article, article_tag, favorited_article,
    prelude::{Article, ArticleTag, FavoritedArticle, Tag},
    tag, user,
};
use migration::{Alias, SimpleExpr};
use sea_orm::{
    entity::prelude::DateTime, prelude::Expr, query::*, ColumnTrait, DatabaseConnection, DbErr,
    DeleteResult, EntityTrait, FromQueryResult, ModelTrait, QueryFilter, RelationTrait,
};
use serde::Serialize;
use std::vec;
use uuid::Uuid;

const DEFAULT_PAGE_LIMIT: u64 = 20;
const DEFAULT_PAGE_OFFSET: u64 = 0;

/// Fetch `articles` with additional info (see ArticleWithAuthor for details). Optional parameters
/// used for filter records by tag name, author name, user who liked aticle. Limit response by
/// limit and offset parameters. Ordered by most recent first.
/// Returns vec of `articles` on success, otherwise returns an `database error`.
pub async fn get_articles_with_filters(
    db: &DatabaseConnection,
    tag_name: Option<&String>,
    author_name: Option<&String>,
    user_who_liked_it: Option<&String>,
    limit: Option<u64>,
    offset: Option<u64>,
    current_user_id: Option<Uuid>,
) -> Result<Vec<ArticleWithAuthor>, DbErr> {
    let art_extended = Article::find()
        .join(JoinType::LeftJoin, article::Relation::User.def())
        .column(user::Column::Username)
        .column(user::Column::Bio)
        .column(user::Column::Image)
        .filter(article_author(author_name))
        .filter(article_has_tag(tag_name))
        .filter(article_liked_by_user(user_who_liked_it))
        .column_as(
            author_followed_by_current_user(current_user_id),
            "following",
        )
        .column_as(article_liked_by_current_user(current_user_id), "favorited")
        .join(
            JoinType::LeftJoin,
            favorited_article::Relation::Article.def().rev(),
        )
        .column_as(article_favorites_count(), "favorites_count")
        .group_by(favorited_article::Column::ArticleId)
        .group_by(article::Column::Id)
        .group_by(user::Column::Username)
        .group_by(user::Column::Id)
        .limit(limit.or(Some(DEFAULT_PAGE_LIMIT)))
        .offset(offset.or(Some(DEFAULT_PAGE_OFFSET)))
        .order_by_desc(article::Column::UpdatedAt)
        .into_model::<ModelExtended>()
        .all(db)
        .await?;

    let art_models: Vec<article::Model> = art_extended
        .clone()
        .into_iter()
        .map(|mde| mde.into())
        .collect();

    let tags = art_models.load_many_to_many(Tag, ArticleTag, db).await?;

    let res: Vec<ArticleWithAuthor> = art_extended
        .into_iter()
        .zip(tags.into_iter())
        .map(|inf| inf.into())
        .collect();

    Ok(res)
}

/// Fetch `articles` created by followed users. Optional parameters used for filter records by
/// tag name, author name, user who liked aticle. Limit response by limit and offset parameters.
/// Ordered by most recent first. Returns vec of `articles` on success, otherwise returns an `database error`.
pub async fn get_articles_feed(
    db: &DatabaseConnection,
    limit: Option<u64>,
    offset: Option<u64>,
    current_user_id: Uuid,
) -> Result<Vec<ArticleWithAuthor>, DbErr> {
    let art_extended = Article::find()
        .join(JoinType::LeftJoin, article::Relation::User.def())
        .column(user::Column::Username)
        .column(user::Column::Bio)
        .column(user::Column::Image)
        .filter(author_followed_by_current_user(Some(current_user_id)))
        .column_as(Expr::val(true), "following")
        .column_as(
            article_liked_by_current_user(Some(current_user_id)),
            "favorited",
        )
        .join(
            JoinType::LeftJoin,
            favorited_article::Relation::Article.def().rev(),
        )
        .column_as(article_favorites_count(), "favorites_count")
        .group_by(favorited_article::Column::ArticleId)
        .group_by(article::Column::Id)
        .group_by(user::Column::Username)
        .group_by(user::Column::Id)
        .limit(limit.or(Some(DEFAULT_PAGE_LIMIT)))
        .offset(offset.or(Some(DEFAULT_PAGE_OFFSET)))
        .order_by_desc(article::Column::UpdatedAt)
        .into_model::<ModelExtended>()
        .all(db)
        .await?;

    let art_models: Vec<article::Model> = art_extended
        .clone()
        .into_iter()
        .map(|mde| mde.into())
        .collect();

    let tags = art_models.load_many_to_many(Tag, ArticleTag, db).await?;

    let res: Vec<ArticleWithAuthor> = art_extended
        .into_iter()
        .zip(tags.into_iter())
        .map(|inf| inf.into())
        .collect();

    Ok(res)
}

/// Count `articles` with additional info (see ArticleWithAuthor for details). Optional parameters used
/// for filter records by tag name, author name, user who liked aticle. Useful for limit/offset pagination.
/// Returns quantity of `articles` on success, otherwise returns an `database error`.
pub async fn get_articles_count(
    db: &DatabaseConnection,
    tag_name: Option<&String>,
    author_name: Option<&String>,
    user_who_liked_it: Option<&String>,
    current_user_id: Option<Uuid>,
) -> Result<u64, DbErr> {
    Article::find()
        .join(JoinType::LeftJoin, article::Relation::User.def())
        .filter(article_author(author_name))
        .filter(article_has_tag(tag_name))
        .filter(article_liked_by_user(user_who_liked_it))
        .filter(if current_user_id.is_some() {
            author_followed_by_current_user(current_user_id)
        } else {
            true.into()
        })
        .count(db)
        .await
}

/// Fetch `article` with additional info (see ArticleWithAuthor for details) for the provided `slug`.
/// Optional identifier used to determine whether the logged in user is a follower of the profile.
/// Returns optional `article` on success, otherwise returns an `database error`.
pub async fn get_article_by_slug(
    db: &DatabaseConnection,
    slug: &str,
    current_user_id: Option<Uuid>,
) -> Result<Option<ArticleWithAuthor>, DbErr> {
    let art_extended = Article::find()
        .filter(article::Column::Slug.eq(slug))
        .join(JoinType::LeftJoin, article::Relation::User.def())
        .column(user::Column::Username)
        .column(user::Column::Bio)
        .column(user::Column::Image)
        .column_as(
            author_followed_by_current_user(current_user_id),
            "following",
        )
        .column_as(article_liked_by_current_user(current_user_id), "favorited")
        .join(
            JoinType::LeftJoin,
            favorited_article::Relation::Article.def().rev(),
        )
        .column_as(article_favorites_count(), "favorites_count")
        .group_by(favorited_article::Column::ArticleId)
        .group_by(article::Column::Id)
        .group_by(user::Column::Username)
        .group_by(user::Column::Id)
        .into_model::<ModelExtended>()
        .one(db)
        .await?;

    if art_extended.is_none() {
        return Ok(None);
    }

    let model: article::Model = art_extended.clone().unwrap().into();
    let tags = model.find_related(Tag).all(db).await?;
    let res: ArticleWithAuthor = (art_extended.unwrap(), tags).into();

    Ok(Some(res))
}

/// Fetch `article` with additional info (see ArticleWithAuthor for details) for the provided `id`.
/// Optional identifier used to determine whether the logged in user is a follower of the profile.
/// Returns optional `article` on success, otherwise returns an `database error`.
pub async fn get_article_by_id(
    db: &DatabaseConnection,
    id: Uuid,
    current_user_id: Option<Uuid>,
) -> Result<Option<ArticleWithAuthor>, DbErr> {
    let art_extended = Article::find_by_id(id)
        .join(JoinType::LeftJoin, article::Relation::User.def())
        .column(user::Column::Username)
        .column(user::Column::Bio)
        .column(user::Column::Image)
        .column_as(
            author_followed_by_current_user(current_user_id),
            "following",
        )
        .column_as(article_liked_by_current_user(current_user_id), "favorited")
        .join(
            JoinType::LeftJoin,
            favorited_article::Relation::Article.def().rev(),
        )
        .column_as(article_favorites_count(), "favorites_count")
        .group_by(favorited_article::Column::ArticleId)
        .group_by(article::Column::Id)
        .group_by(user::Column::Username)
        .group_by(user::Column::Id)
        .into_model::<ModelExtended>()
        .one(db)
        .await?;

    if art_extended.is_none() {
        return Ok(None);
    }

    let model: article::Model = art_extended.clone().unwrap().into();
    let tags = model.find_related(Tag).all(db).await?;
    let res: ArticleWithAuthor = (art_extended.unwrap(), tags).into();

    Ok(Some(res))
}

/// Fetch `article` for the provided `slug`.
/// Returns optional `article` on success, otherwise returns an `database error`.
pub async fn get_article_model_by_slug(
    db: &DatabaseConnection,
    slug: &str,
) -> Result<Option<article::Model>, DbErr> {
    Article::find()
        .filter(article::Column::Slug.eq(slug))
        .one(db)
        .await
}

/// Insert `article` for the provided `ActiveModel`. Reject models with existing slug.
/// Returns `InsertResult` with last inserted id on success, otherwise
/// returns an `database error`.
/// Empty slug(or title, or description, or body), produces error as not allowed on database level.
/// See [`InsertResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.InsertResult.html)
/// documentation for more details.
pub async fn create_article(
    db: &DatabaseConnection,
    article: article::ActiveModel,
) -> Result<InsertResult<article::ActiveModel>, DbErr> {
    Article::insert(article).exec(db).await
}

/// Update `article` for the provided `ActiveModel`.
/// Returns `article` on success, otherwise returns an `database error`.
/// Reject models with non existing username or email.
pub async fn update_article(
    db: &DatabaseConnection,
    article: article::ActiveModel,
) -> Result<article::Model, DbErr> {
    Article::update(article).exec(db).await
}

/// Delete `article` for the provided `ActiveModel`.
/// Returns `DeleteResult` with affected rows count on success, otherwise
/// returns an `database error`.
/// See [`DeleteResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.DeleteResult.html)
/// documentation for more details.
pub async fn delete_article(
    db: &DatabaseConnection,
    article: article::ActiveModel,
) -> Result<DeleteResult, DbErr> {
    Article::delete(article).exec(db).await
}

/// Delete all existing `follower records` from database.
/// Returns `DeleteResult` with affected rows count on success, otherwise
/// returns an `database error`.
/// See [`DeleteResult`](https://docs.rs/sea-orm/latest/sea_orm/struct.DeleteResult.html)
/// documentation for more details.
pub async fn empty_article_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    Article::delete_many().exec(db).await
}

/// Returns expression for determine whether the user is a author of the article.
/// Return `true` if the author name is not specified since used as a filter.
fn article_author(author_name: Option<&String>) -> SimpleExpr {
    match author_name {
        Some(name) => user::Column::Username.like(name),
        None => true.into(),
    }
}

/// Returns expression for determine whether the article is tagged by provided tag.
/// Return `true` if the tag name is not specified since used as a filter.
fn article_has_tag(tag_name: Option<&String>) -> SimpleExpr {
    match tag_name {
        Some(name) => article::Column::Id.in_subquery(
            Article::find()
                .join(
                    JoinType::LeftJoin,
                    article_tag::Relation::Article.def().rev(),
                )
                .join(JoinType::LeftJoin, article_tag::Relation::Tag.def())
                .filter(tag::Column::TagName.like(name))
                .select_only()
                .column(article::Column::Id)
                .into_query(),
        ),
        None => true.into(),
    }
}

/// Returns expression for determine whether the article is liked by provided user.
/// Return `true` if the user name is not specified since used as a filter.
fn article_liked_by_user(user_name: Option<&String>) -> SimpleExpr {
    match user_name {
        Some(name) => article::Column::Id.in_subquery(
            Article::find()
                .join(
                    JoinType::LeftJoin,
                    favorited_article::Relation::Article.def().rev(),
                )
                .join(JoinType::LeftJoin, favorited_article::Relation::User.def())
                .filter(user::Column::Username.like(name))
                .select_only()
                .column(article::Column::Id)
                .into_query(),
        ),
        None => true.into(),
    }
}

/// Returns expression for determine whether the article liked by logged in user.
/// Return `false` if user id is not specified.
fn article_liked_by_current_user(user_id: Option<Uuid>) -> SimpleExpr {
    match user_id {
        Some(id) => article::Column::Id.in_subquery(
            FavoritedArticle::find()
                .select_only()
                .column(favorited_article::Column::ArticleId)
                .filter(favorited_article::Column::UserId.eq(id))
                .into_query(),
        ),
        None => false.into(),
    }
}

fn article_favorites_count() -> SimpleExpr {
    Expr::count(Expr::col(favorited_article::Column::ArticleId)).cast_as(Alias::new("Integer"))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelExtended {
    id: Uuid,
    slug: String,
    title: String,
    description: String,
    body: String,
    favorited: bool,
    favorites_count: i32,
    created_at: Option<DateTime>,
    updated_at: Option<DateTime>,
    author_id: Uuid,
    author: Profile,
}

impl FromQueryResult for ModelExtended {
    fn from_query_result(res: &sea_orm::QueryResult, pre: &str) -> Result<Self, sea_orm::DbErr> {
        Ok(Self {
            id: res.try_get(pre, "id")?,
            slug: res.try_get(pre, "slug")?,
            title: res.try_get(pre, "title")?,
            description: res.try_get(pre, "description")?,
            body: res.try_get(pre, "body")?,
            favorited: res.try_get(pre, "favorited")?,
            favorites_count: res.try_get(pre, "favorites_count")?,
            created_at: res.try_get(pre, "created_at")?,
            updated_at: res.try_get(pre, "updated_at")?,
            author_id: res.try_get(pre, "author_id")?,
            author: Profile::from_query_result(res, pre)?,
        })
    }
}

impl Into<article::Model> for ModelExtended {
    fn into(self) -> article::Model {
        article::Model {
            id: self.id,
            slug: self.slug,
            title: self.title,
            description: self.description,
            body: self.body,
            author_id: self.author_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleWithAuthor {
    slug: String,
    title: String,
    description: String,
    body: String,
    favorited: bool,
    favorites_count: i32,
    created_at: Option<DateTime>,
    updated_at: Option<DateTime>,
    author: Profile,
    tag_list: Vec<String>,
}

impl FromQueryResult for ArticleWithAuthor {
    fn from_query_result(res: &sea_orm::QueryResult, pre: &str) -> Result<Self, sea_orm::DbErr> {
        Ok(Self {
            slug: res.try_get(pre, "slug")?,
            title: res.try_get(pre, "title")?,
            description: res.try_get(pre, "description")?,
            body: res.try_get(pre, "body")?,
            favorited: res.try_get(pre, "favorited")?,
            favorites_count: res.try_get(pre, "favorites_count")?,
            created_at: res.try_get(pre, "created_at")?,
            updated_at: res.try_get(pre, "updated_at")?,
            tag_list: vec![],
            author: Profile::from_query_result(res, pre)?,
        })
    }
}

impl From<(ModelExtended, Vec<tag::Model>)> for ArticleWithAuthor {
    fn from((article, tags): (ModelExtended, Vec<tag::Model>)) -> Self {
        Self {
            slug: article.slug,
            title: article.title,
            description: article.description,
            body: article.body,
            favorited: article.favorited,
            favorites_count: article.favorites_count,
            created_at: article.created_at,
            updated_at: article.updated_at,
            author: article.author,
            tag_list: tags.into_iter().map(|tg| tg.tag_name).collect(),
        }
    }
}

#[cfg(test)]
mod test_get_articles_with_filters {
    use super::get_articles_with_filters;
    use crate::repo::{article::ArticleWithAuthor, user::Profile};
    use crate::tests::{
        Operation::{Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use std::vec;

    #[tokio::test]
    async fn get_existing_articles() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let author = users.unwrap().into_iter().next().unwrap();
        let expected: Vec<ArticleWithAuthor> = articles
            .unwrap()
            .into_iter()
            .rev()
            .map(|artcl| ArticleWithAuthor {
                slug: artcl.slug,
                title: artcl.title,
                description: artcl.description,
                body: artcl.body,
                favorited: false,
                favorites_count: 0,
                author: Profile {
                    username: author.username.clone(),
                    bio: author.bio.clone(),
                    image: author.image.clone(),
                    following: false,
                },
                created_at: artcl.created_at,
                updated_at: artcl.updated_at,
                tag_list: vec![],
            })
            .collect();

        let result =
            get_articles_with_filters(&connection, None, None, None, None, None, None).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn get_empty_list() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Migration)
            .articles(Migration)
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let result =
            get_articles_with_filters(&connection, None, None, None, None, None, None).await?;
        let expected = vec![];
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn filter_article_tag_pos() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Insert(5))
            .article_tags(Insert(vec![(1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]))
            .build()
            .await?;

        let author = users.unwrap().into_iter().nth(0).unwrap();
        let article = articles.unwrap().into_iter().nth(2).unwrap();
        let expected: Vec<ArticleWithAuthor> = [article]
            .into_iter()
            .map(|artcl| ArticleWithAuthor {
                slug: artcl.slug,
                title: artcl.title,
                description: artcl.description,
                body: artcl.body,
                favorited: false,
                favorites_count: 0,
                author: Profile {
                    username: author.username.clone(),
                    bio: author.bio.clone(),
                    image: author.image.clone(),
                    following: false,
                },
                created_at: artcl.created_at,
                updated_at: artcl.updated_at,
                tag_list: vec!["tag_name3".to_owned()],
            })
            .collect();

        let result = get_articles_with_filters(
            &connection,
            Some(&"tag_name3".to_owned()),
            None,
            None,
            None,
            None,
            None,
        )
        .await?;

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn filter_article_tag_neg() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1]))
            .favorited_articles(Migration)
            .tags(Insert(2))
            .article_tags(Insert(vec![(1, 1)]))
            .build()
            .await?;

        let result = get_articles_with_filters(
            &connection,
            Some(&"tag_name2".to_owned()),
            None,
            None,
            None,
            None,
            None,
        )
        .await?;

        let expected = vec![];
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn filter_article_tag_empty() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1]))
            .favorited_articles(Migration)
            .tags(Insert(2))
            .article_tags(Insert(vec![(1, 1)]))
            .build()
            .await?;

        let result = get_articles_with_filters(
            &connection,
            Some(&"".to_owned()),
            None,
            None,
            None,
            None,
            None,
        )
        .await?;

        let expected = vec![];
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn filter_article_author_pos() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 2, 1, 1]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let author = users.unwrap().into_iter().nth(1).unwrap();
        let article = articles.unwrap().into_iter().nth(2).unwrap();
        let expected: Vec<ArticleWithAuthor> = [article]
            .into_iter()
            .map(|artcl| ArticleWithAuthor {
                slug: artcl.slug,
                title: artcl.title,
                description: artcl.description,
                body: artcl.body,
                favorited: false,
                favorites_count: 0,
                author: Profile {
                    username: "username2".to_owned(),
                    bio: author.bio.clone(),
                    image: author.image.clone(),
                    following: false,
                },
                created_at: artcl.created_at,
                updated_at: artcl.updated_at,
                tag_list: vec![],
            })
            .collect();

        let result = get_articles_with_filters(
            &connection,
            None,
            Some(&"username2".to_owned()),
            None,
            None,
            None,
            None,
        )
        .await?;

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn filter_article_author_neg() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let result = get_articles_with_filters(
            &connection,
            None,
            Some(&"username2".to_owned()),
            None,
            None,
            None,
            None,
        )
        .await?;

        let expected = vec![];
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn filter_article_author_empty() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let result = get_articles_with_filters(
            &connection,
            None,
            Some(&"".to_owned()),
            None,
            None,
            None,
            None,
        )
        .await?;

        let expected = vec![];
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn filter_article_user_who_liked_it_pos() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Insert(vec![(3, 2)]))
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let author = users.unwrap().into_iter().nth(0).unwrap();
        let article = articles.unwrap().into_iter().nth(2).unwrap();
        let expected: Vec<ArticleWithAuthor> = [article]
            .into_iter()
            .map(|artcl| ArticleWithAuthor {
                slug: artcl.slug,
                title: artcl.title,
                description: artcl.description,
                body: artcl.body,
                favorited: false,
                favorites_count: 1,
                author: Profile {
                    username: author.username.clone(),
                    bio: author.bio.clone(),
                    image: author.image.clone(),
                    following: false,
                },
                created_at: artcl.created_at,
                updated_at: artcl.updated_at,
                tag_list: vec![],
            })
            .collect();

        let result = get_articles_with_filters(
            &connection,
            None,
            None,
            Some(&"username2".to_owned()),
            None,
            None,
            None,
        )
        .await?;

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn filter_article_user_who_liked_it_neg() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Insert(vec![(3, 2)]))
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let result = get_articles_with_filters(
            &connection,
            None,
            None,
            Some(&"username1".to_owned()),
            None,
            None,
            None,
        )
        .await?;

        let expected = vec![];
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn filter_article_user_who_liked_it_empty() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Insert(vec![(3, 2)]))
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let result = get_articles_with_filters(
            &connection,
            None,
            None,
            Some(&"".to_owned()),
            None,
            None,
            None,
        )
        .await?;

        let expected = vec![];
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn limit_articles_pos() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let author = users.unwrap().into_iter().next().unwrap();
        let expected: Vec<ArticleWithAuthor> = articles
            .unwrap()
            .into_iter()
            .rev()
            .take(2)
            .map(|artcl| ArticleWithAuthor {
                slug: artcl.slug,
                title: artcl.title,
                description: artcl.description,
                body: artcl.body,
                favorited: false,
                favorites_count: 0,
                author: Profile {
                    username: author.username.clone(),
                    bio: author.bio.clone(),
                    image: author.image.clone(),
                    following: false,
                },
                created_at: artcl.created_at,
                updated_at: artcl.updated_at,
                tag_list: vec![],
            })
            .collect();

        let result =
            get_articles_with_filters(&connection, None, None, None, Some(2), None, None).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn limit_articles_zero_val() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let expected = vec![];
        let result =
            get_articles_with_filters(&connection, None, None, None, Some(0), None, None).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn offset_articles_pos() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let author = users.unwrap().into_iter().next().unwrap();
        let expected: Vec<ArticleWithAuthor> = articles
            .unwrap()
            .into_iter()
            .take(3)
            .rev()
            .map(|artcl| ArticleWithAuthor {
                slug: artcl.slug,
                title: artcl.title,
                description: artcl.description,
                body: artcl.body,
                favorited: false,
                favorites_count: 0,
                author: Profile {
                    username: author.username.clone(),
                    bio: author.bio.clone(),
                    image: author.image.clone(),
                    following: false,
                },
                created_at: artcl.created_at,
                updated_at: artcl.updated_at,
                tag_list: vec![],
            })
            .collect();

        let result =
            get_articles_with_filters(&connection, None, None, None, None, Some(2), None).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn offset_articles_zero_val() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let author = users.unwrap().into_iter().next().unwrap();
        let expected: Vec<ArticleWithAuthor> = articles
            .unwrap()
            .into_iter()
            .rev()
            .map(|artcl| ArticleWithAuthor {
                slug: artcl.slug,
                title: artcl.title,
                description: artcl.description,
                body: artcl.body,
                favorited: false,
                favorites_count: 0,
                author: Profile {
                    username: author.username.clone(),
                    bio: author.bio.clone(),
                    image: author.image.clone(),
                    following: false,
                },
                created_at: artcl.created_at,
                updated_at: artcl.updated_at,
                tag_list: vec![],
            })
            .collect();

        let result =
            get_articles_with_filters(&connection, None, None, None, None, Some(0), None).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn articles_author_followed_by_current_user() -> Result<(), TestErr> {
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(3))
            .articles(Insert(vec![1, 2]))
            .favorited_articles(Migration)
            .followers(Insert(vec![(1, 3)]))
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let current_user = users.unwrap().into_iter().last().unwrap();

        let mut result = get_articles_with_filters(
            &connection,
            None,
            None,
            None,
            None,
            None,
            Some(current_user.id),
        )
        .await?;
        result.reverse();

        assert_eq!(result[0].author.following, true);
        assert_eq!(result[1].author.following, false);

        Ok(())
    }

    #[tokio::test]
    async fn articles_favorited_by_current_user() -> Result<(), TestErr> {
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 1]))
            .favorited_articles(Insert(vec![(2, 2)]))
            .followers(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let current_user = users.unwrap().into_iter().last().unwrap();

        let mut result = get_articles_with_filters(
            &connection,
            None,
            None,
            None,
            None,
            None,
            Some(current_user.id),
        )
        .await?;
        result.reverse();

        assert_eq!(result[0].favorited, false);
        assert_eq!(result[1].favorited, true);

        Ok(())
    }

    #[tokio::test]
    async fn articles_favorited_count() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1]))
            .favorited_articles(Insert(vec![(1, 1), (1, 2), (1, 3), (1, 4), (1, 5)]))
            .followers(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let mut result =
            get_articles_with_filters(&connection, None, None, None, None, None, None).await?;
        result.reverse();

        assert_eq!(result[0].favorites_count, 5);
        assert_eq!(result[1].favorites_count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn articles_tag_list() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1]))
            .favorited_articles(Migration)
            .followers(Migration)
            .tags(Insert(2))
            .article_tags(Insert(vec![(1, 1), (1, 2)]))
            .build()
            .await?;

        let mut result =
            get_articles_with_filters(&connection, None, None, None, None, None, None).await?;
        result.reverse();

        let tags = &mut result[0].tag_list;
        tags.sort();

        assert_eq!(result[0].tag_list, vec!["tag_name1", "tag_name2"]);
        assert_eq!(result[1].favorites_count, 0);

        Ok(())
    }
}

#[cfg(test)]
mod test_get_articles_feed {
    use super::get_articles_feed;
    use crate::repo::article::ArticleWithAuthor;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use std::vec;

    #[tokio::test]
    async fn get_followed_authors_articles() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 2, 2, 3, 4]))
            .favorited_articles(Migration)
            .followers(Insert(vec![(1, 5), (2, 5), (3, 5)]))
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let current_user = users.unwrap().into_iter().last().unwrap();
        let expected: Vec<String> = articles
            .unwrap()
            .into_iter()
            .take(4)
            .map(|mdl| mdl.title)
            .collect();

        let result = get_articles_feed(&connection, None, None, current_user.id).await?;
        let result: Vec<String> = result.into_iter().rev().map(|mdl| mdl.title).collect();

        assert_eq!(expected, result);

        Ok(())
    }

    #[tokio::test]
    async fn user_not_follows_any_other() -> Result<(), TestErr> {
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(3))
            .articles(Insert(vec![1, 2, 2]))
            .favorited_articles(Migration)
            .followers(Insert(vec![(1, 2), (2, 1)]))
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let current_user = users.unwrap().into_iter().last().unwrap();
        let expected: Vec<ArticleWithAuthor> = vec![];

        let result = get_articles_feed(&connection, None, None, current_user.id).await?;

        assert_eq!(expected, result);

        Ok(())
    }

    #[tokio::test]
    async fn limit_articles_pos() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 2, 2, 3, 4]))
            .favorited_articles(Migration)
            .followers(Insert(vec![(1, 5), (2, 5), (3, 5)]))
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let current_user = users.unwrap().into_iter().last().unwrap();
        let expected: Vec<String> = articles.unwrap()[2..4]
            .iter()
            .rev()
            .map(|mdl| &mdl.title)
            .cloned()
            .collect();

        let result = get_articles_feed(&connection, Some(2), None, current_user.id).await?;
        let result: Vec<String> = result.iter().map(|mdl| &mdl.title).cloned().collect();

        assert_eq!(expected, result);

        Ok(())
    }

    #[tokio::test]
    async fn limit_articles_zero_val() -> Result<(), TestErr> {
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 2, 2, 3, 4]))
            .favorited_articles(Migration)
            .followers(Insert(vec![(1, 5), (2, 5), (3, 5)]))
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let expected = vec![];
        let current_user = users.unwrap().into_iter().last().unwrap();
        let result = get_articles_feed(&connection, Some(0), None, current_user.id).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn offset_articles_pos() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 2, 2, 3, 4]))
            .favorited_articles(Migration)
            .followers(Insert(vec![(1, 5), (2, 5), (3, 5)]))
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let current_user = users.unwrap().into_iter().last().unwrap();
        let expected: Vec<String> = articles
            .unwrap()
            .iter()
            .take(2)
            .rev()
            .map(|mdl| &mdl.title)
            .cloned()
            .collect();

        let result = get_articles_feed(&connection, None, Some(2), current_user.id).await?;
        let result: Vec<String> = result.iter().map(|mdl| &mdl.title).cloned().collect();
        assert_eq!(expected, result);

        Ok(())
    }

    #[tokio::test]
    async fn offset_articles_zero_val() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 2, 2, 3, 4]))
            .favorited_articles(Migration)
            .followers(Insert(vec![(1, 5), (2, 5), (3, 5)]))
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let expected: Vec<String> = articles
            .unwrap()
            .iter()
            .take(4)
            .rev()
            .map(|mdl| &mdl.title)
            .cloned()
            .collect();
        let current_user = users.unwrap().into_iter().last().unwrap();
        let result = get_articles_feed(&connection, None, Some(0), current_user.id).await?;
        let result: Vec<String> = result.iter().map(|mdl| &mdl.title).cloned().collect();
        assert_eq!(result, expected);

        Ok(())
    }
}

#[cfg(test)]
mod test_get_articles_count {
    use super::get_articles_count;
    use crate::tests::{Operation::Insert, TestData, TestDataBuilder, TestErr};
    use std::vec;
    use uuid::Uuid;

    #[tokio::test]
    async fn count_articles() -> Result<(), TestErr> {
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 2, 2, 3, 4]))
            .favorited_articles(Insert(vec![(1, 1), (2, 2), (3, 2)]))
            .followers(Insert(vec![(1, 5), (2, 5), (3, 5)]))
            .tags(Insert(3))
            .article_tags(Insert(vec![(1, 1), (1, 2), (2, 2)]))
            .build()
            .await?;
        let current_user = users.unwrap().into_iter().last().unwrap();

        let result = get_articles_count(&connection, None, None, None, None).await?;
        assert_eq!(result, 5);
        let result =
            get_articles_count(&connection, Some(&"tag_name2".to_owned()), None, None, None)
                .await?;
        assert_eq!(result, 2);
        let result =
            get_articles_count(&connection, Some(&"not_exist".to_owned()), None, None, None)
                .await?;
        assert_eq!(result, 0);
        let result =
            get_articles_count(&connection, None, Some(&"username2".to_owned()), None, None)
                .await?;
        assert_eq!(result, 2);
        let result =
            get_articles_count(&connection, None, Some(&"not_exist".to_owned()), None, None)
                .await?;
        assert_eq!(result, 0);
        let result =
            get_articles_count(&connection, None, None, Some(&"username2".to_owned()), None)
                .await?;
        assert_eq!(result, 2);
        let result =
            get_articles_count(&connection, None, None, Some(&"not_exist".to_owned()), None)
                .await?;
        assert_eq!(result, 0);
        let result =
            get_articles_count(&connection, None, None, None, Some(current_user.id)).await?;
        assert_eq!(result, 4);
        let result =
            get_articles_count(&connection, None, None, None, Some(Uuid::new_v4())).await?;
        assert_eq!(result, 0);

        Ok(())
    }
}

#[cfg(test)]
mod test_get_article_by_slug {
    use super::get_article_by_slug;
    use crate::repo::{article::ArticleWithAuthor, user::Profile};
    use crate::tests::{
        Operation::{Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use std::vec;

    #[tokio::test]
    async fn get_existing_article() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Insert(1))
            .article_tags(Insert(vec![(3, 1)]))
            .build()
            .await?;

        let author = users.unwrap().into_iter().next().unwrap();
        let article = articles.unwrap().into_iter().nth(2).unwrap();
        let expected = ArticleWithAuthor {
            slug: article.slug,
            title: article.title,
            description: article.description,
            body: article.body,
            favorited: false,
            favorites_count: 0,
            author: Profile {
                username: author.username.clone(),
                bio: author.bio.clone(),
                image: author.image.clone(),
                following: false,
            },
            created_at: article.created_at,
            updated_at: article.updated_at,
            tag_list: vec!["tag_name1".to_owned()],
        };

        let result = get_article_by_slug(&connection, "title3", None).await?;
        assert_eq!(result, Some(expected));

        Ok(())
    }

    #[tokio::test]
    async fn none_existing_slug() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let result = get_article_by_slug(&connection, "not_exist", None).await?;
        let expected = None;
        assert_eq!(result, expected);

        Ok(())
    }
}

#[cfg(test)]
mod test_get_article_by_id {
    use super::get_article_by_id;
    use crate::repo::{article::ArticleWithAuthor, user::Profile};
    use crate::tests::{
        Operation::{Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use std::vec;
    use uuid::Uuid;

    #[tokio::test]
    async fn get_existing_article() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Insert(1))
            .article_tags(Insert(vec![(3, 1)]))
            .build()
            .await?;

        let author = users.unwrap().into_iter().next().unwrap();
        let article = articles.unwrap().into_iter().nth(2).unwrap();
        let expected = ArticleWithAuthor {
            slug: article.slug,
            title: article.title,
            description: article.description,
            body: article.body,
            favorited: false,
            favorites_count: 0,
            author: Profile {
                username: author.username.clone(),
                bio: author.bio.clone(),
                image: author.image.clone(),
                following: false,
            },
            created_at: article.created_at,
            updated_at: article.updated_at,
            tag_list: vec!["tag_name1".to_owned()],
        };

        let result = get_article_by_id(&connection, article.id, None).await?;
        assert_eq!(result, Some(expected));

        Ok(())
    }

    #[tokio::test]
    async fn none_existing_id() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let result = get_article_by_id(&connection, Uuid::new_v4(), None).await?;
        let expected = None;
        assert_eq!(result, expected);

        Ok(())
    }
}

#[cfg(test)]
mod test_get_article_model_by_slug {
    use super::get_article_model_by_slug;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use std::vec;

    #[tokio::test]
    async fn get_existing_article() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let expected = articles.unwrap().into_iter().nth(2).unwrap();
        let result = get_article_model_by_slug(&connection, "title3").await?;
        assert_eq!(result, Some(expected));

        Ok(())
    }

    #[tokio::test]
    async fn none_existing_slug() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let result = get_article_model_by_slug(&connection, "not_exist").await?;
        let expected = None;
        assert_eq!(result, expected);

        Ok(())
    }
}

#[cfg(test)]
mod test_create_article {
    use super::create_article;
    use crate::tests::{
        Operation::{Create, Insert},
        TestData, TestDataBuilder, TestErr,
    };
    use entity::entities::{article, prelude::Article};
    use sea_orm::Set;

    #[tokio::test]
    async fn insert_not_exist_data() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Create(vec![1]))
            .build()
            .await?;
        let id = articles.as_ref().unwrap().iter().next().unwrap().id;
        let actives = TestDataBuilder::activate_models::<Article, article::ActiveModel>(&articles);
        let model = actives.into_iter().next().unwrap();

        let insert_result = create_article(&connection, model).await?;
        assert_eq!(insert_result.last_insert_id, id);

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_id() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                articles: inserted, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1]))
            .build()
            .await?;
        let (_, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Create(2))
            .articles(Create(vec![2, 2]))
            .build()
            .await?;

        let inserted_id = inserted.unwrap().into_iter().next().unwrap().id;
        let second_article = articles.unwrap().into_iter().nth(1).unwrap();
        let model2 = article::ActiveModel {
            id: Set(inserted_id),
            ..second_article.into()
        };

        let insert_result = create_article(&connection, model2).await;

        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: article.id")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_existing_slug() -> Result<(), TestErr> {
        let (
            connection,
            TestData {
                articles: inserted, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1]))
            .build()
            .await?;
        let (_, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Create(2))
            .articles(Create(vec![2, 2]))
            .build()
            .await?;

        let inserted_slug = inserted.unwrap().into_iter().next().unwrap().slug;
        let second_article = articles.unwrap().into_iter().nth(1).unwrap();
        let model2 = article::ActiveModel {
            slug: Set(inserted_slug),
            ..second_article.into()
        };

        let insert_result = create_article(&connection, model2).await;

        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("UNIQUE constraint failed: article.slug")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_empty_slug() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Create(vec![1]))
            .build()
            .await?;
        let created = articles.unwrap().into_iter().next().unwrap();

        let model = article::ActiveModel {
            slug: Set("".to_owned()),
            ..created.into()
        };

        let insert_result = create_article(&connection, model).await;

        assert!(insert_result
            .is_err_and(|err| err.to_string().ends_with("CHECK constraint failed: slug")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_empty_title() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Create(vec![1]))
            .build()
            .await?;
        let created = articles.unwrap().into_iter().next().unwrap();

        let model = article::ActiveModel {
            title: Set("".to_owned()),
            ..created.into()
        };

        let insert_result = create_article(&connection, model).await;

        assert!(insert_result
            .is_err_and(|err| err.to_string().ends_with("CHECK constraint failed: title")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_empty_description() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Create(vec![1]))
            .build()
            .await?;
        let created = articles.unwrap().into_iter().next().unwrap();

        let model = article::ActiveModel {
            description: Set("".to_owned()),
            ..created.into()
        };

        let insert_result = create_article(&connection, model).await;

        assert!(insert_result.is_err_and(|err| err
            .to_string()
            .ends_with("CHECK constraint failed: description")));

        Ok(())
    }

    #[tokio::test]
    async fn insert_empty_body() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Create(vec![1]))
            .build()
            .await?;
        let created = articles.unwrap().into_iter().next().unwrap();

        let model = article::ActiveModel {
            body: Set("".to_owned()),
            ..created.into()
        };

        let insert_result = create_article(&connection, model).await;

        assert!(insert_result
            .is_err_and(|err| err.to_string().ends_with("CHECK constraint failed: body")));

        Ok(())
    }
}

#[cfg(test)]
mod test_update_article {
    use super::update_article;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use chrono::Local;
    use entity::entities::article;
    use sea_orm::ActiveModelTrait;
    use uuid::Uuid;

    #[tokio::test]
    async fn update_existing_data() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .build()
            .await?;
        let model = articles.unwrap().into_iter().nth(3).unwrap();

        let expected = article::Model {
            body: "body".to_owned(),
            description: "description".to_owned(),
            ..model
        };

        let update_model = article::ActiveModel::from(expected.clone()).reset_all();
        let updated = update_article(&connection, update_model).await?;
        assert_eq!(expected, updated);

        Ok(())
    }

    #[tokio::test]
    async fn update_not_existing_data() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Migration)
            .articles(Migration)
            .build()
            .await?;

        let expected = article::Model {
            id: Uuid::new_v4(),
            slug: "slug".to_owned(),
            title: "slug".to_owned(),
            description: "slug".to_owned(),
            body: "slug".to_owned(),
            author_id: Uuid::new_v4(),
            created_at: Some(Local::now().naive_local()),
            updated_at: Some(Local::now().naive_local()),
        };

        let update_model = article::ActiveModel::from(expected).reset_all();
        let result = update_article(&connection, update_model).await;
        assert!(
            result.is_err_and(|err| err.to_string().ends_with("None of the records are updated"))
        );

        Ok(())
    }
}

#[cfg(test)]
mod test_delete_article {
    use super::delete_article;
    use crate::tests::{Operation::Insert, TestData, TestDataBuilder, TestErr};
    use entity::entities::{article, prelude::Article};

    #[tokio::test]
    async fn delete_existing_data() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 2]))
            .build()
            .await?;
        let actives = TestDataBuilder::activate_models::<Article, article::ActiveModel>(&articles);
        let model = actives.into_iter().next().unwrap();

        let delete_result = delete_article(&connection, model).await?;
        assert_eq!(delete_result.rows_affected, 1_u64);

        Ok(())
    }
}

#[cfg(test)]
mod test_empty_article_table {
    use super::empty_article_table;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestDataBuilder, TestErr,
    };
    use entity::entities::{article, prelude::Article};
    use sea_orm::EntityTrait;

    #[tokio::test]
    async fn delete_existing_articles() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1, 1, 1, 1, 1]))
            .build()
            .await?;

        let delete_result = empty_article_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 5_u64);

        let expected: Vec<article::Model> = Vec::new();
        let result = Article::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn delete_empty_table() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Migration)
            .articles(Migration)
            .build()
            .await?;

        let delete_result = empty_article_table(&connection).await?;
        assert_eq!(delete_result.rows_affected, 0_u64);

        let expected: Vec<article::Model> = Vec::new();
        let result = Article::find().all(&connection).await?;
        assert_eq!(result, expected);

        Ok(())
    }
}
