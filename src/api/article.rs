use crate::middleware::auth::Token;
use crate::repo::{
    article::{
        create_article as repo_create_article, delete_article as repo_delete_article,
        get_article_by_id, get_article_by_slug, get_article_model_by_slug, get_articles_count,
        get_articles_feed, get_articles_with_filters, update_article as repo_update_article,
        ArticleWithAuthor,
    },
    article_tag::create_article_tags,
    favorited_article::{
        favorite_article as repo_favorite_article, unfavorite_article as repo_unfavorite_article,
    },
    tag::{create_tags, get_tags_ids},
};
use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use chrono::Local;
use entity::entities::{article, article_tag, favorited_article, tag};
use sea_orm::{prelude::DateTime, ActiveValue::Set, DatabaseConnection};
use serde::{Deserialize, Serialize};
use slug::slugify;
use std::collections::HashMap;
use uuid::Uuid;

use super::error::ApiErr;

/// Axum handler for Fetch `articles` with additional info (see ArticleWithAuthor for details).
/// Query parameters used for filter records by tag name, author name, user who liked aticle.
/// Limit response by limit and offset parameters. Ordered by most recent first.
/// Returns `articles` object on success, otherwise returns an `database error`.
pub async fn list_articles(
    Query(params): Query<HashMap<String, String>>,
    maybe_token: Option<Extension<Token>>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<ArticlesDto>, ApiErr> {
    // Filter by tag:
    let tag_name = params.get(&"tag".to_string()).filter(|str| !str.is_empty());

    // Filter by author:
    let author_name = params
        .get(&"author".to_string())
        .filter(|str| !str.is_empty());

    // Favorited by user:
    let user_who_liked_it = params
        .get(&"favorited".to_string())
        .filter(|str| !str.is_empty());

    // Limit number of articles (default is 20):
    let limit = params
        .get(&"limit".to_string())
        .map(|lm| lm.parse::<u64>())
        .filter(|res| res.is_ok())
        .map(|res| res.unwrap());

    // Offset/skip number of articles (default is 0):
    let offset = params
        .get(&"offset".to_string())
        .map(|lm| lm.parse::<u64>())
        .filter(|res| res.is_ok())
        .map(|res| res.unwrap());

    let articles = get_articles_with_filters(
        &db,
        tag_name,
        author_name,
        user_who_liked_it,
        limit,
        offset,
        maybe_token.clone().map(|tkn| tkn.id),
    )
    .await?;

    let articles_count =
        get_articles_count(&db, tag_name, author_name, user_who_liked_it, None).await?;

    let articles_dto = ArticlesDto {
        articles,
        articles_count,
    };

    Ok(Json(articles_dto))
}

/// Axum handler for fetch `articles` created by followed users. Limit response by limit and offset parameters.
/// Returns `articles` object on success, otherwise returns an `database error`.
pub async fn feed_articles(
    Query(params): Query<HashMap<String, String>>,
    Extension(token): Extension<Token>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<ArticlesDto>, ApiErr> {
    // Limit number of articles (default is 20):
    let limit = params
        .get(&"limit".to_string())
        .map(|lm| lm.parse::<u64>())
        .filter(|res| res.is_ok())
        .map(|res| res.unwrap());

    // Offset/skip number of articles (default is 0):
    let offset = params
        .get(&"offset".to_string())
        .map(|lm| lm.parse::<u64>())
        .filter(|res| res.is_ok())
        .map(|res| res.unwrap());

    let current_user_id = token.id;

    let articles = get_articles_feed(&db, limit, offset, current_user_id).await?;
    let articles_count = get_articles_count(&db, None, None, None, Some(current_user_id)).await?;

    let articles_dto = ArticlesDto {
        articles,
        articles_count,
    };

    Ok(Json(articles_dto))
}

/// Axum handler for retrieve information about article with provided title. Optional
/// token used to determine whether the logged in user is a follower of the article author.
/// Returns json object with article on success, otherwise returns an `api error`.
pub async fn get_article(
    State(db): State<DatabaseConnection>,
    maybe_token: Option<Extension<Token>>,
    Path(slug): Path<String>,
) -> Result<Json<ArticleDto>, ApiErr> {
    let article = get_article_by_slug(&db, &slug, maybe_token.map(|tkn| tkn.id)).await?;

    let article_dto = ArticleDto { article };
    Ok(Json(article_dto))
}

/// Axum handler for creating article. Only for authenticated users, thus token is required.
/// Returns json object with article on success, otherwise returns an `api error`.
pub async fn create_article(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
    Json(payload): Json<CreateArticleDto>,
) -> Result<Json<ArticleDto>, ApiErr> {
    let current_user_id = token.id;
    let input = payload.article;

    let article_model = article::ActiveModel {
        id: Set(Uuid::new_v4()),
        slug: Set(slugify(
            format! {"{}{}", input.title, current_user_id.simple()},
        )),
        title: Set(input.title),
        description: Set(input.description),
        body: Set(input.body),
        author_id: Set(current_user_id),
        ..Default::default()
    };

    let art_res = repo_create_article(&db, article_model).await?;

    // Insert new tags
    if let Some(tgs) = &input.tag_list {
        let tag_models = tgs
            .iter()
            .map(|tg| tag::ActiveModel {
                id: Set(Uuid::new_v4()),
                tag_name: Set(tg.to_owned()),
            })
            .collect();

        create_tags(&db, tag_models).await?;
    };

    // Find existing tag ids
    let tags_ids = get_tags_ids(&db, input.tag_list.clone().unwrap_or_default()).await?;

    let article_tag_models = tags_ids
        .iter()
        .map(|&id| article_tag::ActiveModel {
            tag_id: Set(id),
            article_id: Set(art_res.last_insert_id),
        })
        .collect::<Vec<article_tag::ActiveModel>>();

    create_article_tags(&db, article_tag_models).await?;

    let article = get_article_by_id(&db, art_res.last_insert_id, Some(current_user_id)).await?;

    let article_dto = ArticleDto { article };
    Ok(Json(article_dto))
}

/// Axum handler for updating article. Only for authenticated users, thus token is required.
/// Returns json object with article on success, otherwise returns an `api error`.
pub async fn update_article(
    Path(slug): Path<String>,
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
    Json(payload): Json<UpdateArticleDto>,
) -> Result<Json<ArticleDto>, ApiErr> {
    let current_user_id = token.id;
    let input = payload.article;

    let updated_article = get_article_model_by_slug(&db, &slug)
        .await?
        .ok_or(ApiErr::ArticleNotExist)?;

    let mut article_model: article::ActiveModel = updated_article.into();

    if input.title.is_some() {
        article_model.slug = Set(slugify(input.title.as_ref().unwrap()));
        article_model.title = Set(input.title.to_owned().unwrap());
    }
    if input.description.is_some() {
        article_model.description = Set(input.description.to_owned().unwrap());
    }
    if input.body.is_some() {
        article_model.body = Set(input.body.to_owned().unwrap());
    }

    if [&input.title, &input.description, &input.body]
        .iter()
        .any(|fld| fld.is_some())
    {
        let time = DateTime::from_timestamp_millis(Local::now().timestamp_millis()).unwrap();
        article_model.updated_at = Set(Some(time));
    }

    let art_res = repo_update_article(&db, article_model).await?;

    let article = get_article_by_id(&db, art_res.id, Some(current_user_id)).await?;

    let article_dto = ArticleDto { article };
    Ok(Json(article_dto))
}

/// Axum handler for delete article by provided article slug. Only for authenticated users,
/// thus token is required. Returns empty json object on success, otherwise returns an `api error`.
pub async fn delete_article(
    Path(slug): Path<String>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<()>, ApiErr> {
    let deleted_article = get_article_model_by_slug(&db, &slug)
        .await?
        .ok_or(ApiErr::ArticleNotExist)?;

    let article_model: article::ActiveModel = deleted_article.into();

    repo_delete_article(&db, article_model).await?;

    Ok(Json(()))
}

/// Axum handler for favorite article by logged user.
/// Returns json object with article on success, otherwise returns an `api error`.
pub async fn favorite_article(
    Path(slug): Path<String>,
    Extension(token): Extension<Token>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<ArticleDto>, ApiErr> {
    let current_user_id = token.id;

    let finded = get_article_model_by_slug(&db, &slug)
        .await?
        .ok_or(ApiErr::ArticleNotExist)?;

    let favorite_article_model = favorited_article::ActiveModel {
        article_id: Set(finded.id),
        user_id: Set(current_user_id),
    };

    repo_favorite_article(&db, favorite_article_model).await?;

    let article = get_article_by_id(&db, finded.id, Some(current_user_id)).await?;

    let article_dto = ArticleDto { article };
    Ok(Json(article_dto))
}

/// Axum handler for unfavorite article by logged user.
/// Returns json object with article on success, otherwise returns an `api error`.
pub async fn unfavorite_article(
    Path(slug): Path<String>,
    Extension(token): Extension<Token>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<ArticleDto>, ApiErr> {
    let current_user_id = token.id;

    let finded = get_article_model_by_slug(&db, &slug)
        .await?
        .ok_or(ApiErr::ArticleNotExist)?;

    let favorite_article_model = favorited_article::ActiveModel {
        article_id: Set(finded.id),
        user_id: Set(current_user_id),
    };

    repo_unfavorite_article(&db, favorite_article_model).await?;

    let article = get_article_by_id(&db, finded.id, Some(current_user_id)).await?;

    let article_dto = ArticleDto { article };
    Ok(Json(article_dto))
}

/// Struct describing JSON object, returned by handler. Contains list of articles.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticlesDto {
    articles: Vec<ArticleWithAuthor>,
    articles_count: u64,
}

/// Struct describing JSON object, returned by handler. Contains optional article.
#[derive(Debug, Serialize)]
pub struct ArticleDto {
    article: Option<ArticleWithAuthor>,
}

/// Struct describing JSON object from article creation request. Contains article.
#[derive(Debug, Deserialize)]
pub struct CreateArticleDto {
    article: CreateArticle,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateArticle {
    title: String,
    description: String,
    body: String,
    tag_list: Option<Vec<String>>,
}

/// Struct describing JSON object from change article data request. Contains article data.
#[derive(Debug, Deserialize)]
pub struct UpdateArticleDto {
    article: UpdateArticle,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct UpdateArticle {
    title: Option<String>,
    description: Option<String>,
    body: Option<String>,
}

#[cfg(test)]
mod test_list_articles {
    use super::list_articles;
    use crate::{
        middleware::auth::Token,
        tests::{
            Operation::{Insert, Migration},
            TestData, TestDataBuilder, TestErr,
        },
    };
    use axum::extract::Query;
    use axum::{extract::State, Extension, Json};
    use dotenvy::dotenv;
    use entity::entities::user;
    use std::collections::HashMap;
    use std::vec;

    #[tokio::test]
    async fn get_existing_articles() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(3))
            .articles(Insert(vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2])) // 13 articles
            .tags(Insert(2))
            .article_tags(Insert(vec![
                (1, 1),
                (3, 1),
                (5, 1),
                (6, 1),
                (7, 1),
                (10, 1),
                (11, 1),
                (13, 1),
                (1, 2),
            ])) // 8 article_tags
            .favorited_articles(Insert(vec![
                (1, 2),
                (3, 2),
                (6, 2),
                (10, 2),
                (11, 2),
                (13, 2),
                (1, 1),
            ]))
            .followers(Migration)
            .build()
            .await?;

        let current_user: user::Model = users.unwrap().into_iter().last().unwrap();

        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let params: HashMap<String, String> = [
            ("tag".to_owned(), "tag_name1".to_owned()),
            ("author".to_owned(), "username1".to_owned()),
            ("favorited".to_owned(), "username2".to_owned()),
            ("limit".to_owned(), "4".to_owned()),
            ("offset".to_owned(), "2".to_owned()),
        ]
        .into_iter()
        .collect();

        let result =
            list_articles(Query(params), Some(Extension(token)), State(connection)).await?;
        let Json(result) = result;

        assert_eq!(result.articles.len(), 3);
        assert_eq!(result.articles_count, 5);

        Ok(())
    }

    #[tokio::test]
    async fn get_no_articles() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Migration)
            .articles(Migration)
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .build()
            .await?;

        let params: HashMap<String, String> = HashMap::new();

        let result = list_articles(Query(params), None, State(connection)).await?;
        let Json(result) = result;

        assert_eq!(result.articles.len(), 0);
        assert_eq!(result.articles_count, 0);

        Ok(())
    }
}

#[cfg(test)]
mod test_feed_articles {
    use super::feed_articles;
    use crate::{
        middleware::auth::Token,
        tests::{
            Operation::{Insert, Migration},
            TestData, TestDataBuilder, TestErr,
        },
    };
    use axum::extract::Query;
    use axum::{extract::State, Extension, Json};
    use dotenvy::dotenv;
    use entity::entities::user;
    use std::collections::HashMap;
    use std::vec;

    #[tokio::test]
    async fn get_existing_articles() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(6))
            .articles(Insert(vec![1, 2, 3, 4, 5, 1, 2, 3, 4, 5]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .followers(Insert(vec![(1, 6), (2, 6), (3, 6), (4, 6), (3, 5)]))
            .build()
            .await?;

        let current_user: user::Model = users.unwrap().into_iter().last().unwrap();

        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let params: HashMap<String, String> = [
            ("limit".to_owned(), "5".to_owned()),
            ("offset".to_owned(), "5".to_owned()),
        ]
        .into_iter()
        .collect();

        let result = feed_articles(Query(params), Extension(token), State(connection)).await?;
        let Json(result) = result;

        assert_eq!(result.articles.len(), 3);
        assert_eq!(result.articles_count, 8);

        Ok(())
    }

    #[tokio::test]
    async fn get_no_articles() -> Result<(), TestErr> {
        let (connection, TestData { users, .. }) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Migration)
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .followers(Migration)
            .build()
            .await?;

        let current_user: user::Model = users.unwrap().into_iter().last().unwrap();
        let token = Token {
            exp: 35,
            id: current_user.id,
        };
        let params: HashMap<String, String> = HashMap::new();

        let result = feed_articles(Query(params), Extension(token), State(connection)).await?;
        let Json(result) = result;

        assert_eq!(result.articles.len(), 0);
        assert_eq!(result.articles_count, 0);

        Ok(())
    }
}

#[cfg(test)]
mod test_get_article {
    use super::get_article;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestDataBuilder, TestErr,
    };
    use axum::{
        extract::{Path, State},
        Json,
    };
    use dotenvy::dotenv;

    #[tokio::test]
    async fn get_existing_article() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1]))
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .followers(Migration)
            .build()
            .await?;

        // Actual test start
        let slug = "title1";
        let result = get_article(State(connection), None, Path(slug.to_owned())).await?;
        let Json(result) = result;

        assert_eq!(result.article.unwrap().title, slug.to_owned());

        Ok(())
    }

    #[tokio::test]
    async fn get_non_existing_article() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Migration)
            .articles(Migration)
            .favorited_articles(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .followers(Migration)
            .build()
            .await?;

        let slug = "not existing slug";
        let result = get_article(State(connection), None, Path(slug.to_owned())).await?;
        let Json(result) = result;

        assert_eq!(result.article, None);

        Ok(())
    }
}

#[cfg(test)]
mod test_create_article {
    use super::{create_article, CreateArticle, CreateArticleDto};
    use crate::middleware::auth::Token;
    use crate::tests::{
        Operation::{Create, Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::{extract::State, Extension, Json};
    use dotenvy::dotenv;
    use entity::entities::{article, user};

    #[tokio::test]
    async fn create_new_article() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Create(vec![1]))
            .comments(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .favorited_articles(Migration)
            .followers(Migration)
            .build()
            .await?;
        let current_user: user::Model = users.unwrap().into_iter().next().unwrap();
        let article: article::Model = articles.unwrap().into_iter().next().unwrap();

        let article_data = CreateArticleDto {
            article: CreateArticle {
                title: article.title.clone(),
                description: article.description,
                body: article.body,
                tag_list: Some(vec!["tag_name1".to_owned(), "tag_name2".to_owned()]),
            },
        };

        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let result =
            create_article(State(connection), Extension(token), Json(article_data)).await?;
        let Json(result) = result;

        assert_eq!(result.article.unwrap().title, article.title);

        Ok(())
    }
}

#[cfg(test)]
mod test_update_article {
    use super::{update_article, UpdateArticle, UpdateArticleDto};
    use crate::api::error::ApiErr;
    use crate::middleware::auth::Token;
    use crate::tests::{
        Operation::{Create, Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::{
        extract::{Path, State},
        Extension, Json,
    };
    use dotenvy::dotenv;
    use entity::entities::{article, user};

    #[tokio::test]
    async fn update_existing_article() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1]))
            .comments(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .favorited_articles(Migration)
            .followers(Migration)
            .build()
            .await?;

        let new_article_title = "updated_title";
        let user: user::Model = users.unwrap().into_iter().next().unwrap();
        let mut article: article::Model = articles.unwrap().into_iter().next().unwrap();
        article.title = new_article_title.to_owned();

        let payload = UpdateArticleDto {
            article: UpdateArticle {
                title: Some(new_article_title.to_owned()),
                ..Default::default()
            },
        };

        let token = Token {
            exp: 35,
            id: user.id,
        };

        // Actual test start
        let result = update_article(
            Path(article.slug),
            State(connection),
            Extension(token),
            Json(payload),
        )
        .await?;
        let Json(result) = result;

        assert_eq!(result.article.unwrap().title, new_article_title);

        Ok(())
    }

    #[tokio::test]
    async fn update_non_existing_article() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Create(vec![1]))
            .comments(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .favorited_articles(Migration)
            .followers(Migration)
            .build()
            .await?;

        let user: user::Model = users.unwrap().into_iter().next().unwrap();
        let article: article::Model = articles.unwrap().into_iter().next().unwrap();

        let payload = UpdateArticleDto {
            article: UpdateArticle {
                title: Some("updated_title".to_owned()),
                ..Default::default()
            },
        };

        let token = Token {
            exp: 35,
            id: user.id,
        };

        // Actual test start
        let result = update_article(
            Path(article.slug),
            State(connection),
            Extension(token),
            Json(payload),
        )
        .await;

        matches!(result, Err(ApiErr::ArticleNotExist));

        Ok(())
    }
}

#[cfg(test)]
mod test_delete_article {
    use super::delete_article;
    use crate::api::error::ApiErr;
    use crate::tests::{
        Operation::{Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::extract::{Path, State};
    use entity::entities::article;
    use std::vec;

    #[tokio::test]
    async fn delete_existing_article() -> Result<(), TestErr> {
        let (connection, TestData { articles, .. }) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1]))
            .followers(Migration)
            .build()
            .await?;

        let article: article::Model = articles.unwrap().into_iter().next().unwrap();

        let _result = delete_article(Path(article.slug), State(connection)).await?;

        Ok(())
    }

    #[tokio::test]
    async fn delete_non_existing_article() -> Result<(), TestErr> {
        let (connection, _) = TestDataBuilder::new()
            .users(Insert(5))
            .articles(Insert(vec![1, 1]))
            .followers(Migration)
            .build()
            .await?;

        let result = delete_article(Path("slug".to_owned()), State(connection)).await;

        matches!(result, Err(ApiErr::ArticleNotExist));

        Ok(())
    }
}

#[cfg(test)]
mod test_favorite_article {
    use super::favorite_article;
    use crate::api::error::ApiErr;
    use crate::middleware::auth::Token;
    use crate::tests::{
        Operation::{Create, Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::{
        extract::{Path, State},
        Extension, Json,
    };
    use dotenvy::dotenv;
    use entity::entities::{article, user};

    #[tokio::test]
    async fn favorite_existing_article() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1]))
            .comments(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .favorited_articles(Migration)
            .followers(Migration)
            .build()
            .await?;

        let current_user: user::Model = users.unwrap().into_iter().next().unwrap();
        let article: article::Model = articles.unwrap().into_iter().next().unwrap();

        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let result = favorite_article(
            Path(article.slug.clone()),
            Extension(token),
            State(connection),
        )
        .await?;
        let Json(result) = result;

        assert_eq!(result.article.unwrap().slug, article.slug);

        Ok(())
    }

    #[tokio::test]
    async fn favorite_non_existing_user() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Create(vec![1]))
            .comments(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .favorited_articles(Migration)
            .followers(Migration)
            .build()
            .await?;

        let current_user: user::Model = users.unwrap().into_iter().next().unwrap();
        let article: article::Model = articles.unwrap().into_iter().next().unwrap();

        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let result =
            favorite_article(Path(article.slug), Extension(token), State(connection)).await;

        matches!(result, Err(ApiErr::ArticleNotExist));

        Ok(())
    }
}

#[cfg(test)]
mod test_unfavorite_article {
    use super::unfavorite_article;
    use crate::api::error::ApiErr;
    use crate::middleware::auth::Token;
    use crate::tests::{
        Operation::{Create, Insert, Migration},
        TestData, TestDataBuilder, TestErr,
    };
    use axum::{
        extract::{Path, State},
        Extension, Json,
    };
    use dotenvy::dotenv;
    use entity::entities::{article, user};

    #[tokio::test]
    async fn unfavorite_existing_article() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Insert(vec![1]))
            .comments(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .favorited_articles(Migration)
            .followers(Migration)
            .build()
            .await?;

        let current_user: user::Model = users.unwrap().into_iter().next().unwrap();
        let article: article::Model = articles.unwrap().into_iter().next().unwrap();

        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let result = unfavorite_article(
            Path(article.slug.clone()),
            Extension(token),
            State(connection),
        )
        .await?;
        let Json(result) = result;

        assert_eq!(result.article.unwrap().slug, article.slug);

        Ok(())
    }

    #[tokio::test]
    async fn unfavorite_non_existing_user() -> Result<(), TestErr> {
        dotenv().expect(".env file not found");
        let (
            connection,
            TestData {
                users, articles, ..
            },
        ) = TestDataBuilder::new()
            .users(Insert(1))
            .articles(Create(vec![1]))
            .comments(Migration)
            .tags(Migration)
            .article_tags(Migration)
            .favorited_articles(Migration)
            .followers(Migration)
            .build()
            .await?;

        let current_user: user::Model = users.unwrap().into_iter().next().unwrap();
        let article: article::Model = articles.unwrap().into_iter().next().unwrap();

        let token = Token {
            exp: 35,
            id: current_user.id,
        };

        let result =
            unfavorite_article(Path(article.slug), Extension(token), State(connection)).await;

        matches!(result, Err(ApiErr::ArticleNotExist));

        Ok(())
    }
}
