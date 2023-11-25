use super::user::{author_followed_by_current_user, Profile};
use entity::entities::{
    article, article_tag, favorited_article, follower,
    prelude::{Article, FavoritedArticle},
    tag, user,
};
use migration::{Alias, SimpleExpr};
use sea_orm::{
    entity::prelude::DateTime, prelude::Expr, query::*, ColumnTrait, DatabaseConnection, DbErr,
    DeleteResult, EntityTrait, FromQueryResult, QueryFilter, RelationTrait,
};
use serde::Serialize;
use uuid::Uuid;

const DEFAULT_PAGE_LIMIT: u64 = 20;
const DEFAULT_PAGE_OFFSET: u64 = 0;

pub async fn get_articles_with_filters(
    db: &DatabaseConnection,
    tag_name: Option<&String>,
    author_name: Option<&String>,
    user_who_liked_it: Option<&String>,
    limit: Option<u64>,
    offset: Option<u64>,
    current_user_id: Option<Uuid>,
) -> Result<Vec<ArticleWithAuthor>, DbErr> {
    Article::find()
        .join(JoinType::LeftJoin, article::Relation::User.def())
        .column(user::Column::Username)
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
        // .column_as(article::Column::Id.count(), "articles_count")
        .group_by(favorited_article::Column::ArticleId)
        .group_by(article::Column::Id)
        .group_by(user::Column::Username)
        .group_by(user::Column::Id)
        .limit(limit.or(Some(DEFAULT_PAGE_LIMIT)))
        .offset(offset.or(Some(DEFAULT_PAGE_OFFSET)))
        .order_by_desc(article::Column::UpdatedAt)
        .into_model::<ArticleWithAuthor>()
        .all(db)
        .await
}

pub async fn get_articles_feed(
    db: &DatabaseConnection,
    limit: Option<u64>,
    offset: Option<u64>,
    current_user_id: Uuid,
) -> Result<Vec<ArticleWithAuthor>, DbErr> {
    Article::find()
        .join(JoinType::LeftJoin, article::Relation::User.def())
        .column(user::Column::Username)
        // .filter(
        //     user::Column::Id.in_subquery(
        //         Follower::find()
        //             .filter(follower::Column::UserId.eq(current_user_id))
        //             .select_only()
        //             .column(follower::Column::FollowerId)
        //             .into_query(),
        //     ),
        // )
        .filter(author_followed_by_current_user(Some(current_user_id)))
        .column_as(
            author_followed_by_current_user(Some(current_user_id)),
            "following",
        )
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
        .order_by_desc(article::Column::UpdatedAt)
        .limit(limit.or(Some(DEFAULT_PAGE_LIMIT)))
        .offset(offset.or(Some(DEFAULT_PAGE_OFFSET)))
        .into_model::<ArticleWithAuthor>()
        .all(db)
        .await
}

pub async fn get_article_by_slug(
    db: &DatabaseConnection,
    slug: &str,
    current_user_id: Option<Uuid>,
) -> Result<Option<ArticleWithAuthor>, DbErr> {
    Article::find()
        .filter(article::Column::Slug.eq(slug))
        .join(JoinType::LeftJoin, article::Relation::User.def())
        .column(user::Column::Username)
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
        .into_model::<ArticleWithAuthor>()
        .one(db)
        .await
}

pub async fn get_article_by_id(
    db: &DatabaseConnection,
    id: Uuid,
    current_user_id: Uuid,
) -> Result<Option<ArticleWithAuthor>, DbErr> {
    Article::find_by_id(id)
        .join(JoinType::LeftJoin, article::Relation::User.def())
        .column(user::Column::Username)
        .column_as(
            author_followed_by_current_user(Some(current_user_id)),
            "following",
        )
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
        .into_model::<ArticleWithAuthor>()
        .one(db)
        .await
}

pub async fn get_article_model_by_slug(
    db: &DatabaseConnection,
    slug: &str,
) -> Result<Option<article::Model>, DbErr> {
    Article::find()
        .filter(article::Column::Slug.eq(slug))
        .one(db)
        .await
}

pub async fn create_article(
    db: &DatabaseConnection,
    article: article::ActiveModel,
) -> Result<InsertResult<article::ActiveModel>, DbErr> {
    Article::insert(article).exec(db).await
}

pub async fn update_article(
    db: &DatabaseConnection,
    article: article::ActiveModel,
) -> Result<article::Model, DbErr> {
    Article::update(article).exec(db).await
}

pub async fn delete_article(
    db: &DatabaseConnection,
    article: article::ActiveModel,
) -> Result<DeleteResult, DbErr> {
    Article::delete(article).exec(db).await
}

pub async fn empty_article_table(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    Article::delete_many().exec(db).await
}

fn article_author(author_name: Option<&String>) -> SimpleExpr {
    match author_name {
        Some(name) => user::Column::Username.like(name),
        None => false.into(),
    }
}

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
        None => false.into(),
    }
}

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
        None => false.into(),
    }
}

fn article_liked_by_current_user(user_id: Option<Uuid>) -> SimpleExpr {
    match user_id {
        Some(id) => article::Column::Id.in_subquery(
            FavoritedArticle::find()
                .select_only()
                .column(favorited_article::Column::ArticleId)
                .filter(follower::Column::UserId.eq(id))
                .into_query(),
        ),
        None => false.into(),
    }
}

fn article_favorites_count() -> SimpleExpr {
    Expr::count(Expr::col(favorited_article::Column::ArticleId)).cast_as(Alias::new("Integer"))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleWithAuthor {
    slug: String,
    title: String,
    description: Option<String>,
    body: Option<String>,
    favorited: Option<bool>,
    favorites_count: Option<i32>,
    created_at: Option<DateTime>,
    updated_at: Option<DateTime>,
    author: Profile,
    pub tag_list: Option<Vec<String>>,
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
            tag_list: None,
            author: Profile::from_query_result(res, pre)?,
        })
    }
}
