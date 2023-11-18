use anyhow::Result;
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use cder::DatabaseSeeder;
use entity::entities::{prelude::*, *};
use rand_core::OsRng;
use sea_orm::ActiveValue::Set;
use sea_orm::{sea_query::OnConflict, ActiveModelTrait, DatabaseConnection, EntityTrait, Iterable};

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

async fn seed_user(seeder: &mut DatabaseSeeder, db: &DatabaseConnection) -> Result<()> {
    seeder
        .populate_async("seed/fixtures/user.yml", |model: user::Model| async move {
            let mut active_model: user::ActiveModel = model.into();
            active_model.id.take();
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

            let res = User::insert::<user::ActiveModel>(active_model)
                .on_conflict(
                    OnConflict::column(user::Column::Username)
                        .do_nothing()
                        .to_owned(),
                )
                .on_conflict(
                    OnConflict::column(user::Column::Email)
                        .do_nothing()
                        .to_owned(),
                )
                .exec(db)
                .await;

            Ok(res.map_or(-1, |ir| ir.last_insert_id) as i64)
        })
        .await?;

    Ok(())
}

async fn seed_article(seeder: &mut DatabaseSeeder, db: &DatabaseConnection) -> Result<()> {
    seeder
        .populate_async(
            "seed/fixtures/article.yml",
            |model: article::Model| async move {
                let mut active_model: article::ActiveModel = model.into();
                active_model.id.take();
                if active_model.updated_at.as_ref().is_none() {
                    active_model.updated_at.take();
                }
                if active_model.created_at.as_ref().is_none() {
                    active_model.created_at.take();
                }
                active_model = active_model.reset_all();

                let res = Article::insert::<article::ActiveModel>(active_model)
                    .on_conflict(
                        OnConflict::column(article::Column::Slug)
                            .do_nothing()
                            .to_owned(),
                    )
                    .on_conflict(
                        OnConflict::columns([article::Column::AuthorId, article::Column::Title])
                            .do_nothing()
                            .to_owned(),
                    )
                    .exec(db)
                    .await;

                Ok(res.map_or(-1, |ir| ir.last_insert_id) as i64)
            },
        )
        .await?;

    Ok(())
}

async fn seed_comment(seeder: &mut DatabaseSeeder, db: &DatabaseConnection) -> Result<()> {
    seeder
        .populate_async(
            "seed/fixtures/comment.yml",
            |model: comment::Model| async move {
                let mut active_model: comment::ActiveModel = model.into();
                active_model.id.take();
                if active_model.updated_at.as_ref().is_none() {
                    active_model.updated_at.take();
                }
                if active_model.created_at.as_ref().is_none() {
                    active_model.created_at.take();
                }
                active_model = active_model.reset_all();

                let res = Comment::insert::<comment::ActiveModel>(active_model)
                    .exec(db)
                    .await;

                Ok(res.map_or(-1, |ir| ir.last_insert_id) as i64)
            },
        )
        .await?;

    Ok(())
}

async fn seed_tag(seeder: &mut DatabaseSeeder, db: &DatabaseConnection) -> Result<()> {
    seeder
        .populate_async("seed/fixtures/tag.yml", |model: tag::Model| async move {
            let mut active_model: tag::ActiveModel = model.into();
            active_model.id.take();
            active_model = active_model.reset_all();

            let res = Tag::insert::<tag::ActiveModel>(active_model)
                .on_conflict(
                    OnConflict::column(tag::Column::TagName)
                        .do_nothing()
                        .to_owned(),
                )
                .exec(db)
                .await;

            Ok(res.map_or(-1, |ir| ir.last_insert_id) as i64)
        })
        .await?;

    Ok(())
}

async fn seed_article_tag(seeder: &mut DatabaseSeeder, db: &DatabaseConnection) -> Result<()> {
    seeder
        .populate_async(
            "seed/fixtures/article_tag.yml",
            |model: article_tag::Model| async move {
                let mut active_model: article_tag::ActiveModel = model.into();
                active_model = active_model.reset_all();

                let _res = ArticleTag::insert::<article_tag::ActiveModel>(active_model)
                    .on_conflict(
                        OnConflict::columns(article_tag::Column::iter())
                            .do_nothing()
                            .to_owned(),
                    )
                    .exec(db)
                    .await;

                Ok(-1_i64)
            },
        )
        .await?;

    Ok(())
}

async fn seed_follower(seeder: &mut DatabaseSeeder, db: &DatabaseConnection) -> Result<()> {
    seeder
        .populate_async(
            "seed/fixtures/follower.yml",
            |model: follower::Model| async move {
                let mut active_model: follower::ActiveModel = model.into();
                active_model = active_model.reset_all();

                let _res = Follower::insert::<follower::ActiveModel>(active_model)
                    .on_conflict(
                        OnConflict::columns(follower::Column::iter())
                            .do_nothing()
                            .to_owned(),
                    )
                    .exec(db)
                    .await;

                Ok(-1_i64)
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
            "seed/fixtures/favorited_article.yml",
            |model: favorited_article::Model| async move {
                let mut active_model: favorited_article::ActiveModel = model.into();
                active_model = active_model.reset_all();

                let _res = FavoritedArticle::insert::<favorited_article::ActiveModel>(active_model)
                    .on_conflict(
                        OnConflict::columns(favorited_article::Column::iter())
                            .do_nothing()
                            .to_owned(),
                    )
                    .exec(db)
                    .await;

                Ok(-1_i64)
            },
        )
        .await?;

    Ok(())
}
