//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.4

use sea_orm::entity::prelude::*;
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Deserialize)]
// TODO Add Postgres feature only
// #[sea_orm(schema_name = "realworld_schema", table_name = "comment")]
#[sea_orm(table_name = "comment")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[serde(skip_deserializing)]
    pub id: Uuid,
    #[sea_orm(column_type = "Text")]
    pub body: String,
    pub author_id: Uuid,
    pub article_id: Uuid,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::article::Entity",
        from = "Column::ArticleId",
        to = "super::article::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Article,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::AuthorId",
        to = "super::user::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::article::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Article.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}