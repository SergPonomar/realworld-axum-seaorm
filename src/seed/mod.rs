use crate::repo::article::{create_article, empty_article_table};
use crate::repo::article_tag::{empty_article_tag_table, insert_article_tag};
use crate::repo::comment::{empty_comment_table, insert_comment};
use crate::repo::favorited_article::{empty_favorited_article_table, favorite_article};
use crate::repo::follower::{create_follower, empty_follower_table};
use crate::repo::tag::{empty_tag_table, insert_tag};
use crate::repo::user::{create_user, empty_user_table};
use anyhow::Result;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use cder::DatabaseSeeder;
use entity::entities::*;
use rand_core::OsRng;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, DbErr, DeleteResult};
use uuid::Uuid;

pub async fn populate_seeds(db: &DatabaseConnection) -> Result<()> {
    let mut seeder = DatabaseSeeder::new();

    seed_user(&mut seeder, db).await?;
    seed_article(&mut seeder, db).await?;
    seed_comment(&mut seeder, db).await?;
    seed_tag(&mut seeder, db).await?;
    seed_article_tag(&mut seeder, db).await?;
    seed_follower(&mut seeder, db).await?;
    seed_favorited_article(&mut seeder, db).await
}

pub async fn empty_all_tables(db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
    empty_article_table(db).await?;
    empty_article_tag_table(db).await?;
    empty_comment_table(db).await?;
    empty_favorited_article_table(db).await?;
    empty_follower_table(db).await?;
    empty_tag_table(db).await?;
    empty_user_table(db).await
}

async fn seed_user(seeder: &mut DatabaseSeeder, db: &DatabaseConnection) -> Result<()> {
    seeder
        .populate_async(
            "src/seed/fixtures/user.yml",
            |model: user::Model| async move {
                let mut active_model: user::ActiveModel = model.into();
                active_model.id = Set(Uuid::new_v4());
                if active_model.image.as_ref().is_none() {
                    active_model.image.take();
                }

                let salt = SaltString::generate(&mut OsRng);
                let hashed_password = Argon2::default()
                    .hash_password(active_model.password.as_ref().as_bytes(), &salt)
                    .map(|hash| hash.to_string())
                    .unwrap();

                active_model.password = Set(hashed_password);
                active_model = active_model.reset_all();

                let res = create_user(db, active_model).await.unwrap();

                Ok(res.last_insert_id)
            },
        )
        .await?;

    Ok(())
}

async fn seed_article(seeder: &mut DatabaseSeeder, db: &DatabaseConnection) -> Result<()> {
    seeder
        .populate_async(
            "src/seed/fixtures/article.yml",
            |model: article::Model| async move {
                let mut active_model: article::ActiveModel = model.into();
                active_model.id = Set(Uuid::new_v4());
                if active_model.updated_at.as_ref().is_none() {
                    active_model.updated_at.take();
                }
                if active_model.created_at.as_ref().is_none() {
                    active_model.created_at.take();
                }
                active_model = active_model.reset_all();

                let res = create_article(db, active_model).await.unwrap();

                Ok(res.last_insert_id)
            },
        )
        .await?;

    Ok(())
}

async fn seed_comment(seeder: &mut DatabaseSeeder, db: &DatabaseConnection) -> Result<()> {
    seeder
        .populate_async(
            "src/seed/fixtures/comment.yml",
            |model: comment::Model| async move {
                println!("{:#?}", model);
                let mut active_model: comment::ActiveModel = model.into();
                active_model.id = Set(Uuid::new_v4());
                if active_model.updated_at.as_ref().is_none() {
                    active_model.updated_at.take();
                }
                if active_model.created_at.as_ref().is_none() {
                    active_model.created_at.take();
                }
                active_model = active_model.reset_all();

                let res = insert_comment(db, active_model).await.unwrap();

                Ok(res.last_insert_id)
            },
        )
        .await?;

    Ok(())
}

async fn seed_tag(seeder: &mut DatabaseSeeder, db: &DatabaseConnection) -> Result<()> {
    seeder
        .populate_async(
            "src/seed/fixtures/tag.yml",
            |model: tag::Model| async move {
                let mut active_model: tag::ActiveModel = model.into();
                active_model.id = Set(Uuid::new_v4());
                active_model = active_model.reset_all();

                let res = insert_tag(db, active_model).await.unwrap();
                Ok(res.last_insert_id)
            },
        )
        .await?;

    Ok(())
}

async fn seed_article_tag(seeder: &mut DatabaseSeeder, db: &DatabaseConnection) -> Result<()> {
    seeder
        .populate_async(
            "src/seed/fixtures/article_tag.yml",
            |model: article_tag::Model| async move {
                println!("{:#?}", model);
                let mut active_model: article_tag::ActiveModel = model.into();
                active_model = active_model.reset_all();

                let res = insert_article_tag(db, active_model).await.unwrap();

                Ok(format!("{:?}", res.last_insert_id))
            },
        )
        .await?;

    Ok(())
}

async fn seed_follower(seeder: &mut DatabaseSeeder, db: &DatabaseConnection) -> Result<()> {
    seeder
        .populate_async(
            "src/seed/fixtures/follower.yml",
            |model: follower::Model| async move {
                let mut active_model: follower::ActiveModel = model.into();
                active_model = active_model.reset_all();

                let res = create_follower(db, active_model).await.unwrap();

                Ok(format!("{:?}", res.last_insert_id))
            },
        )
        .await?;

    Ok(())
}

async fn seed_favorited_article(
    seeder: &mut DatabaseSeeder,
    db: &DatabaseConnection,
) -> Result<()> {
    seeder
        .populate_async(
            "src/seed/fixtures/favorited_article.yml",
            |model: favorited_article::Model| async move {
                let mut active_model: favorited_article::ActiveModel = model.into();
                active_model = active_model.reset_all();

                let res = favorite_article(db, active_model).await.unwrap();

                Ok(format!("{:?}", res.last_insert_id))
            },
        )
        .await?;

    Ok(())
}
