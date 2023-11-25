//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.4

use sea_orm::entity::prelude::*;
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Deserialize)]
#[sea_orm(schema_name = "realworld_schema", table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[serde(skip_deserializing)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub email: String,
    #[sea_orm(unique)]
    pub username: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub bio: Option<String>,
    pub image: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub password: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::article::Entity")]
    Article,
    #[sea_orm(has_many = "super::comment::Entity")]
    Comment,
    #[sea_orm(has_many = "super::favorited_article::Entity")]
    FavoritedArticle,
}

impl Related<super::comment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Comment.def()
    }
}

impl Related<super::favorited_article::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FavoritedArticle.def()
    }
}

impl Related<super::article::Entity> for Entity {
    fn to() -> RelationDef {
        super::favorited_article::Relation::Article.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::favorited_article::Relation::User.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
