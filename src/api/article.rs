use crate::middleware::auth::Token;
use crate::repo::{
    article::{
        create_article as repo_create_article, delete_article as repo_delete_article,
        get_article_by_id, get_article_by_slug, get_article_model_by_slug, get_articles_feed,
        get_articles_with_filters, update_article as repo_update_article, ArticleWithAuthor,
    },
    article_tag::{create_article_tags, get_article_tags},
    favorited_article::{
        favorite_article as repo_favorite_article, unfavorite_article as repo_unfavorite_article,
    },
    tag::{create_tags, get_tags_ids},
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Local;
use entity::entities::{article, article_tag, favorited_article, tag};
use sea_orm::{prelude::DateTime, ActiveValue::Set, DatabaseConnection};
use serde::{Deserialize, Serialize};
use slug::slugify;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn list_articles(
    Query(params): Query<HashMap<String, String>>,
    maybe_token: Option<Extension<Token>>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<ArticlesDto>, (StatusCode, String)> {
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
        maybe_token.map(|tkn| tkn.id),
    )
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let articles_dto = ArticlesDto { articles };
    Ok(Json(articles_dto))
}

pub async fn feed_articles(
    Query(params): Query<HashMap<String, String>>,
    Extension(token): Extension<Token>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<ArticlesDto>, (StatusCode, String)> {
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

    let articles = get_articles_feed(&db, limit, offset, current_user_id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let articles_dto = ArticlesDto { articles };
    Ok(Json(articles_dto))
}

pub async fn get_article(
    State(db): State<DatabaseConnection>,
    maybe_token: Option<Extension<Token>>,
    Path(slug): Path<String>,
) -> Result<Json<ArticleDto>, (StatusCode, String)> {
    let article = get_article_by_slug(&db, &slug, maybe_token.map(|tkn| tkn.id))
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let article_dto = ArticleDto { article };
    Ok(Json(article_dto))
}

pub async fn create_article(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
    Json(payload): Json<CreateArticleDto>,
) -> Result<Json<ArticleDto>, (StatusCode, String)> {
    let current_user_id = token.id;
    let input = payload.article;

    let article_model = article::ActiveModel {
        id: Set(Uuid::new_v4()),
        slug: Set(slugify(&input.title)),
        title: Set(input.title),
        description: Set(input.description),
        body: Set(input.body),
        author_id: Set(current_user_id),
        ..Default::default()
    };

    let art_res = repo_create_article(&db, article_model)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    if let Some(tgs) = &input.tag_list {
        let tag_models = tgs
            .iter()
            .map(|tg| tag::ActiveModel {
                id: Set(Uuid::new_v4()),
                tag_name: Set(tg.to_owned()),
            })
            .collect();

        let _tag_res = create_tags(&db, tag_models)
            .await
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    };

    // Find tags ids
    // TODO зачем ids если это uuid, ты их сам и генерируешь. Может быит апдейт а не инсерт
    let tags_ids = get_tags_ids(&db, input.tag_list.clone().unwrap_or_default())
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let article_tag_models = tags_ids
        .iter()
        .map(|&id| article_tag::ActiveModel {
            tag_id: Set(id),
            article_id: Set(art_res.last_insert_id),
        })
        .collect::<Vec<article_tag::ActiveModel>>();

    let _article_tag_res = create_article_tags(&db, article_tag_models)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let mut article = get_article_by_id(&db, art_res.last_insert_id, current_user_id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    article.as_mut().unwrap().tag_list = input.tag_list;

    let article_dto = ArticleDto { article };
    Ok(Json(article_dto))
}

pub async fn update_article(
    Path(slug): Path<String>,
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<Token>,
    Json(payload): Json<UpdateArticleDto>,
) -> Result<Json<ArticleDto>, (StatusCode, String)> {
    let current_user_id = token.id;
    let input = payload.article;

    let updated_article = get_article_model_by_slug(&db, &slug)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Article not finded".to_string(),
        ))?;

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

    if vec![&input.title, &input.description, &input.body]
        .iter()
        .any(|fld| fld.is_some())
    {
        let time = DateTime::from_timestamp_millis(Local::now().timestamp_millis()).unwrap();
        article_model.updated_at = Set(Some(time));
    }

    let art_res = repo_update_article(&db, article_model)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let mut article = get_article_by_id(&db, art_res.id, current_user_id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let tags = get_article_tags(&db, art_res.id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    article.as_mut().unwrap().tag_list = Some(tags);

    let article_dto = ArticleDto { article };
    Ok(Json(article_dto))
}

pub async fn delete_article(
    Path(slug): Path<String>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<()>, (StatusCode, String)> {
    let deleted_article = get_article_model_by_slug(&db, &slug)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Article not finded".to_string(),
        ))?;

    let article_model: article::ActiveModel = deleted_article.into();

    let _art_res = repo_delete_article(&db, article_model)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(Json(()))
}

pub async fn favorite_article(
    Path(slug): Path<String>,
    Extension(token): Extension<Token>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<ArticleDto>, (StatusCode, String)> {
    let current_user_id = token.id;

    let finded = get_article_model_by_slug(&db, &slug)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Article not finded".to_string(),
        ))?;

    let favorite_article_model = favorited_article::ActiveModel {
        article_id: Set(finded.id),
        user_id: Set(current_user_id),
    };

    let _art_res = repo_favorite_article(&db, favorite_article_model)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let mut article = get_article_by_id(&db, finded.id, current_user_id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let tags = get_article_tags(&db, finded.id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    article.as_mut().unwrap().tag_list = Some(tags);

    let article_dto = ArticleDto { article };
    Ok(Json(article_dto))
}

pub async fn unfavorite_article(
    Path(slug): Path<String>,
    Extension(token): Extension<Token>,
    State(db): State<DatabaseConnection>,
) -> Result<Json<ArticleDto>, (StatusCode, String)> {
    let current_user_id = token.id;

    let finded = get_article_model_by_slug(&db, &slug)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Article not finded".to_string(),
        ))?;

    let favorite_article_model = favorited_article::ActiveModel {
        article_id: Set(finded.id),
        user_id: Set(current_user_id),
    };

    let _art_res = repo_unfavorite_article(&db, favorite_article_model)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let mut article = get_article_by_id(&db, finded.id, current_user_id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let tags = get_article_tags(&db, finded.id)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    article.as_mut().unwrap().tag_list = Some(tags);

    let article_dto = ArticleDto { article };
    Ok(Json(article_dto))
}

#[derive(Debug, Serialize)]
pub struct ArticlesDto {
    articles: Vec<ArticleWithAuthor>,
}

#[derive(Debug, Serialize)]
pub struct ArticleDto {
    article: Option<ArticleWithAuthor>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateArticle {
    title: String,
    description: String,
    body: String,
    tag_list: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateArticleDto {
    article: CreateArticle,
}

#[derive(Clone, Debug, Deserialize)]
struct UpdateArticle {
    title: Option<String>,
    description: Option<String>,
    body: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateArticleDto {
    article: UpdateArticle,
}
