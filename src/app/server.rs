use crate::api::{
    article::{
        create_article, delete_article, favorite_article, feed_articles, get_article,
        list_articles, unfavorite_article, update_article,
    },
    comment::{create_comment, delete_comment, list_comments},
    profile::{follow_user, get_profile, unfollow_user},
    tags::list_tags,
    user::{get_current_user, login_user, register_user, update_user},
};
use crate::middleware::auth::{auth, optional_auth};
use axum::{
    middleware::from_fn,
    routing::{delete, get, post, put},
    Router,
};
use sea_orm::DatabaseConnection;
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use tower::ServiceBuilder;

const DEFAULT_APP_PORT: u16 = 3000;
const DEFAULT_APP_HOST: &str = "127.0.0.1";
const APP_PORT: &str = "APP_PORT";
const APP_HOST: &str = "APP_HOST";
// const AUTHENTICATED_USER_ID: u64 = 28;

pub async fn start(connection: DatabaseConnection) {
    let optional_auth_routes = Router::new()
        .route("/api/users", post(register_user))
        .route("/api/users/login", post(login_user))
        .route("/api/profiles/:username", get(get_profile))
        .route("/api/articles", get(list_articles))
        .route("/api/articles/:slug", get(get_article))
        .route("/api/articles/:slug/comments", get(list_comments))
        .route("/api/tags", get(list_tags))
        .layer(ServiceBuilder::new().layer(from_fn(optional_auth)));

    let auth_routes = Router::new()
        .route("/api/user", put(update_user).get(get_current_user))
        .route(
            "/api/profiles/:username/follow",
            post(follow_user).delete(unfollow_user),
        )
        .route("/api/articles", post(create_article))
        .route("/api/articles/feed", get(feed_articles))
        .route(
            "/api/articles/:slug",
            put(update_article).delete(delete_article),
        )
        .route(
            "/api/articles/:slug/favorite",
            post(favorite_article).delete(unfavorite_article),
        )
        .route("/api/articles/:slug/comments", post(create_comment))
        .route("/api/articles/:slug/comments/:id", delete(delete_comment))
        .layer(ServiceBuilder::new().layer(from_fn(auth)));

    let app = Router::new()
        .merge(auth_routes)
        .merge(optional_auth_routes)
        .with_state(connection);

    let addr = get_socket_address();
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// #[derive(Debug, Serialize)]
// struct UserDto {
//     user: Option<UserWithToken>,
// }

// // #[derive(Clone, Debug, PartialEq, FromQueryResult, Eq, Serialize)]
// #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
// #[serde(rename_all = "camelCase")]
// struct UserWithToken {
//     token: String,
//     email: String,
//     username: String,
//     bio: Option<String>,
//     image: Option<String>,
// }

// impl FromQueryResult for UserWithToken {
//     fn from_query_result(res: &sea_orm::QueryResult, pre: &str) -> Result<Self, sea_orm::DbErr> {
//         let username: String = res.try_get(pre, "username")?;

//         Ok(Self {
//             token: create_token(&username).unwrap(),
//             email: res.try_get(pre, "email")?,
//             username,
//             bio: res.try_get(pre, "bio")?,
//             image: res.try_get(pre, "image")?,
//         })
//     }
// }

// #[derive(Clone, Debug, Deserialize)]
// #[serde(rename_all = "camelCase")]
// struct LoginUser {
//     email: String,
//     password: String,
// }

// #[derive(Debug, Deserialize)]
// struct LoginUserDto {
//     user: LoginUser,
// }

// async fn login_user(
//     State(db): State<DatabaseConnection>,
//     Json(payload): Json<LoginUserDto>,
// ) -> Result<Json<()>, (StatusCode, String)> {
//     let input = payload.user;

//     // let user_pass = User::find()
//     //     .filter(user::Column::Email.eq(&input.email))
//     //     .select_only()
//     //     .column(user::Column::Password)
//     //     .into_tuple::<String>()
//     //     .one(&db)
//     //     .await
//     //     .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let current_user = get_user_by_email(&db, &input.email)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
//         .ok_or((
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "User not finded".to_string(),
//         ))?;
//     // let current_user = User::find()
//     //     .filter(user::Column::Email.eq(input.email))
//     //     .one(&db)
//     //     .await
//     //     .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
//     //     .ok_or((
//     //         StatusCode::INTERNAL_SERVER_ERROR,
//     //         "User not finded".to_string(),
//     //     ))?;

//     let _pass_matched = PasswordHash::new(&current_user.password)
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
//         .map(|parsed_hash| {
//             Argon2::default().verify_password(input.password.as_bytes(), &parsed_hash)
//         })
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.1))?;

//     // let user_dto = UserDto {
//     //     user: Some(current_user),
//     // };
//     // Ok(Json(user_dto))
//     Ok(Json(()))
// }

// #[derive(Clone, Debug, Deserialize)]
// struct RegisterUser {
//     username: String,
//     email: String,
//     password: String,
// }

// #[derive(Debug, Deserialize)]
// struct RegisterUserDto {
//     user: RegisterUser,
// }

// async fn register_user(
//     State(db): State<DatabaseConnection>,
//     Json(payload): Json<RegisterUserDto>,
// ) -> Result<Json<UserDto>, (StatusCode, String)> {
//     let input = payload.user;

//     let salt = SaltString::generate(&mut OsRng);
//     let hashed_password = Argon2::default()
//         .hash_password(input.password.as_bytes(), &salt)
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
//         .map(|hash| hash.to_string())?;

//     let user_model = user::ActiveModel {
//         id: Set(Uuid::new_v4()),
//         email: Set(input.email),
//         username: Set(input.username),
//         password: Set(hashed_password),
//         ..Default::default()
//     };

//     let user_res = create_user(&db, user_model)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let current_user = User::find_by_id(user_res.last_insert_id)
//         .into_model::<UserWithToken>()
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let user_dto = UserDto { user: current_user };
//     Ok(Json(user_dto))
// }

// async fn get_current_user(
//     State(db): State<DatabaseConnection>,
//     Extension(token): Extension<Token>,
// ) -> Result<Json<UserDto>, (StatusCode, String)> {
//     let current_user = User::find()
//         .filter(user::Column::Username.eq(token.username))
//         .into_model::<UserWithToken>()
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let user_dto = UserDto { user: current_user };
//     Ok(Json(user_dto))
// }

// // TODO add password
// #[derive(Clone, Debug, Deserialize)]
// #[serde(rename_all = "camelCase")]
// struct UpdateUser {
//     email: Option<String>,
//     username: Option<String>,
//     bio: Option<String>,
//     image: Option<String>,
// }

// #[derive(Debug, Deserialize)]
// struct UpdateUserDto {
//     user: UpdateUser,
// }

// async fn update_user(
//     State(db): State<DatabaseConnection>,
//     Extension(token): Extension<Token>,
//     Json(payload): Json<UpdateUserDto>,
// ) -> Result<Json<UserDto>, (StatusCode, String)> {
//     let input = payload.user;

//     let finded: Option<user::Model> = User::find()
//         .filter(user::Column::Username.eq(&token.username))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let mut user_model: user::ActiveModel = finded.unwrap().into();

//     if input.email.is_some() {
//         user_model.email = Set(input.email.to_owned().unwrap());
//     }
//     if input.username.is_some() {
//         user_model.username = Set(input.username.to_owned().unwrap());
//     }
//     if input.bio.is_some() {
//         user_model.bio = Set(input.bio.to_owned());
//     }
//     if input.image.is_some() {
//         user_model.image = Set(input.image);
//     }

//     user_model
//         .update(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let current_user = User::find()
//         .filter(user::Column::Username.eq(token.username))
//         .into_model::<UserWithToken>()
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let user_dto = UserDto { user: current_user };
//     Ok(Json(user_dto))
// }

// #[derive(Clone, Debug, PartialEq, FromQueryResult, Eq, Serialize)]
// struct Profile {
//     username: String,
//     bio: Option<String>,
//     image: Option<String>,
//     following: bool,
// }

// #[derive(Debug, Serialize)]
// struct ProfileDto {
//     profile: Option<Profile>,
// }

// async fn get_profile(
//     State(db): State<DatabaseConnection>,
//     maybe_token: Option<Extension<Token>>,
//     Path(username): Path<String>,
// ) -> Result<Json<ProfileDto>, (StatusCode, String)> {
//     let current_user = if let Some(token) = maybe_token {
//         User::find()
//             .filter(user::Column::Username.eq(&token.username))
//             .one(&db)
//             .await
//             .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
//     } else {
//         None
//     };

//     let profile = User::find()
//         .filter(user::Column::Username.eq(username))
//         .column_as(
//             user::Column::Id.in_subquery(
//                 User::find()
//                     .join(JoinType::InnerJoin, follower::Relation::User1.def().rev())
//                     .select_only()
//                     .column(follower::Column::UserId)
//                     .filter(match current_user {
//                         Some(user) => follower::Column::FollowerId.eq(user.id),
//                         None => false.into(),
//                     })
//                     .into_query(),
//             ),
//             "following",
//         )
//         .into_model::<Profile>()
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let profile_dto = ProfileDto { profile: profile };
//     Ok(Json(profile_dto))
// }

// async fn follow_user(
//     State(db): State<DatabaseConnection>,
//     Extension(token): Extension<Token>,
//     Path(username): Path<String>,
// ) -> Result<Json<ProfileDto>, (StatusCode, String)> {
//     let finded: Option<user::Model> = User::find()
//         .filter(user::Column::Username.eq(&username))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let current_user = User::find()
//         .filter(user::Column::Username.eq(token.username))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let follower_model = follower::ActiveModel {
//         user_id: Set(finded.as_ref().unwrap().id),
//         follower_id: Set(current_user.as_ref().unwrap().id),
//     };

//     let _flw_res = follower_model
//         .insert(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let profile = User::find()
//         .filter(user::Column::Username.eq(username))
//         .column_as(
//             user::Column::Id.in_subquery(
//                 User::find()
//                     .join(JoinType::InnerJoin, follower::Relation::User1.def().rev())
//                     .select_only()
//                     .column(follower::Column::UserId)
//                     .filter(match current_user {
//                         Some(user) => follower::Column::FollowerId.eq(user.id),
//                         None => false.into(),
//                     })
//                     .into_query(),
//             ),
//             "following",
//         )
//         .into_model::<Profile>()
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let profile_dto = ProfileDto { profile: profile };
//     Ok(Json(profile_dto))
// }

// async fn unfollow_user(
//     State(db): State<DatabaseConnection>,
//     Extension(token): Extension<Token>,
//     Path(username): Path<String>,
// ) -> Result<Json<ProfileDto>, (StatusCode, String)> {
//     let finded: Option<user::Model> = User::find()
//         .filter(user::Column::Username.eq(&username))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let current_user = User::find()
//         .filter(user::Column::Username.eq(token.username))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let follower_model = follower::ActiveModel {
//         user_id: Set(finded.as_ref().unwrap().id),
//         follower_id: Set(current_user.as_ref().unwrap().id),
//     };

//     let _flw_res = follower_model
//         .delete(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let profile = User::find()
//         .filter(user::Column::Username.eq(username))
//         .column_as(
//             user::Column::Id.in_subquery(
//                 User::find()
//                     .join(JoinType::InnerJoin, follower::Relation::User1.def().rev())
//                     .select_only()
//                     .column(follower::Column::UserId)
//                     .filter(match current_user {
//                         Some(user) => follower::Column::FollowerId.eq(user.id),
//                         None => false.into(),
//                     })
//                     .into_query(),
//             ),
//             "following",
//         )
//         .into_model::<Profile>()
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let profile_dto = ProfileDto { profile: profile };
//     Ok(Json(profile_dto))
// }

// #[derive(Debug, Serialize)]
// struct ArticlesDto {
//     articles: Vec<ArticleWithAuthor>,
// }

// #[derive(Debug, Serialize)]
// struct ArticleDto {
//     article: Option<ArticleWithAuthor>,
// }

// #[derive(Debug, Serialize)]
// #[serde(rename_all = "camelCase")]
// struct ArticleWithAuthor {
//     slug: String,
//     title: String,
//     description: Option<String>,
//     body: Option<String>,
//     favorited: Option<bool>,
//     favorites_count: Option<i32>,
//     created_at: Option<DateTime>,
//     updated_at: Option<DateTime>,
//     author: Profile,
//     tag_list: Option<Vec<String>>,
// }

// impl FromQueryResult for ArticleWithAuthor {
//     fn from_query_result(res: &sea_orm::QueryResult, pre: &str) -> Result<Self, sea_orm::DbErr> {
//         Ok(Self {
//             slug: res.try_get(pre, "slug")?,
//             title: res.try_get(pre, "title")?,
//             description: res.try_get(pre, "description")?,
//             body: res.try_get(pre, "body")?,
//             favorited: res.try_get(pre, "favorited")?,
//             favorites_count: res.try_get(pre, "favorites_count")?,
//             created_at: res.try_get(pre, "created_at")?,
//             updated_at: res.try_get(pre, "updated_at")?,
//             tag_list: None,
//             author: Profile::from_query_result(res, pre)?,
//         })
//     }
// }

// async fn list_articles(
//     Query(params): Query<HashMap<String, String>>,
//     maybe_token: Option<Extension<Token>>,
//     State(db): State<DatabaseConnection>,
// ) -> Result<Json<ArticlesDto>, (StatusCode, String)> {
//     // Filter by tag:
//     let tag_name = params
//         .get(&"tag".to_string())
//         .unwrap_or(&String::new())
//         .to_owned();

//     // Filter by author:
//     let author_name = params
//         .get(&"author".to_string())
//         .unwrap_or(&String::new())
//         .to_owned();

//     // Favorited by user:
//     let user_who_liked_it = params
//         .get(&"favorited".to_string())
//         .unwrap_or(&String::new())
//         .to_owned();

//     // Limit number of articles (default is 20):
//     let limit = params
//         .get(&"limit".to_string())
//         .map_or(20, |lm| lm.parse().unwrap_or(20));

//     // Offset/skip number of articles (default is 0):
//     let offset = params
//         .get(&"offset".to_string())
//         .map_or(0, |off| off.parse().unwrap_or(0));

//     let current_user = if let Some(token) = maybe_token {
//         User::find()
//             .filter(user::Column::Username.eq(&token.username))
//             .one(&db)
//             .await
//             .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
//     } else {
//         None
//     };

//     let articles = Article::find()
//         .join(JoinType::LeftJoin, article::Relation::User.def())
//         .column(user::Column::Username)
//         .filter(if author_name.is_empty() {
//             Expr::value(true)
//         } else {
//             user::Column::Username.like(author_name)
//         })
//         .filter(
//             article::Column::Id.in_subquery(
//                 Article::find()
//                     .join(
//                         JoinType::LeftJoin,
//                         article_tag::Relation::Article.def().rev(),
//                     )
//                     .join(JoinType::LeftJoin, article_tag::Relation::Tag.def())
//                     .filter(if tag_name.is_empty() {
//                         Expr::value(true)
//                     } else {
//                         tag::Column::TagName.like(tag_name)
//                     })
//                     .select_only()
//                     .column(article::Column::Id)
//                     .into_query(),
//             ),
//         )
//         .filter(
//             article::Column::Id.in_subquery(
//                 Article::find()
//                     .join(
//                         JoinType::LeftJoin,
//                         favorited_article::Relation::Article.def().rev(),
//                     )
//                     .join(JoinType::LeftJoin, favorited_article::Relation::User.def())
//                     .filter(if user_who_liked_it.is_empty() {
//                         Expr::value(true)
//                     } else {
//                         user::Column::Username.like(user_who_liked_it)
//                     })
//                     .select_only()
//                     .column(article::Column::Id)
//                     .into_query(),
//             ),
//         )
//         .column_as(
//             user::Column::Id.in_subquery(
//                 User::find()
//                     .join(JoinType::InnerJoin, follower::Relation::User1.def().rev())
//                     .select_only()
//                     .column(follower::Column::UserId)
//                     .filter(match &current_user {
//                         Some(user) => follower::Column::FollowerId.eq(user.id),
//                         None => false.into(),
//                     })
//                     .into_query(),
//             ),
//             "following",
//         )
//         .column_as(
//             article::Column::Id.in_subquery(
//                 FavoritedArticle::find()
//                     .select_only()
//                     .column(favorited_article::Column::ArticleId)
//                     .filter(match &current_user {
//                         Some(user) => follower::Column::UserId.eq(user.id),
//                         None => false.into(),
//                     })
//                     .into_query(),
//             ),
//             "favorited",
//         )
//         .join(
//             JoinType::LeftJoin,
//             favorited_article::Relation::Article.def().rev(),
//         )
//         .column_as(
//             Expr::count(Expr::col(favorited_article::Column::ArticleId))
//                 .cast_as(Alias::new("Integer")),
//             "favorites_count",
//         )
//         .group_by(favorited_article::Column::ArticleId)
//         .group_by(article::Column::Id)
//         .group_by(user::Column::Username)
//         .group_by(user::Column::Id)
//         .limit(limit)
//         .offset(offset)
//         .order_by_desc(article::Column::UpdatedAt)
//         .into_model::<ArticleWithAuthor>()
//         .all(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let articles_dto = ArticlesDto { articles: articles };
//     Ok(Json(articles_dto))
// }

// async fn feed_articles(
//     Query(params): Query<HashMap<String, String>>,
//     Extension(token): Extension<Token>,
//     State(db): State<DatabaseConnection>,
// ) -> Result<Json<ArticlesDto>, (StatusCode, String)> {
//     // Limit number of articles (default is 20):
//     let limit = params
//         .get(&"limit".to_string())
//         .map_or(20, |lm| lm.parse().unwrap_or(20));

//     // Offset/skip number of articles (default is 0):
//     let offset = params
//         .get(&"offset".to_string())
//         .map_or(0, |off| off.parse().unwrap_or(0));

//     let current_user = User::find()
//         .filter(user::Column::Username.eq(token.username))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let current_user_id = current_user.unwrap().id;

//     let articles = Article::find()
//         .join(JoinType::LeftJoin, article::Relation::User.def())
//         .column(user::Column::Username)
//         .filter(
//             user::Column::Id.in_subquery(
//                 Follower::find()
//                     .filter(follower::Column::UserId.eq(current_user_id))
//                     .select_only()
//                     .column(follower::Column::FollowerId)
//                     .into_query(),
//             ),
//         )
//         .column_as(
//             user::Column::Id.in_subquery(
//                 User::find()
//                     .join(JoinType::InnerJoin, follower::Relation::User1.def().rev())
//                     .select_only()
//                     .column(follower::Column::UserId)
//                     .filter(follower::Column::FollowerId.eq(current_user_id))
//                     .into_query(),
//             ),
//             "following",
//         )
//         .column_as(
//             article::Column::Id.in_subquery(
//                 FavoritedArticle::find()
//                     .select_only()
//                     .column(favorited_article::Column::ArticleId)
//                     .filter(favorited_article::Column::UserId.eq(current_user_id))
//                     .into_query(),
//             ),
//             "favorited",
//         )
//         .join(
//             JoinType::LeftJoin,
//             favorited_article::Relation::Article.def().rev(),
//         )
//         .column_as(
//             Expr::count(Expr::col(favorited_article::Column::ArticleId))
//                 .cast_as(Alias::new("Integer")),
//             "favorites_count",
//         )
//         .group_by(favorited_article::Column::ArticleId)
//         .group_by(article::Column::Id)
//         .group_by(user::Column::Username)
//         .group_by(user::Column::Id)
//         .order_by_desc(article::Column::UpdatedAt)
//         .limit(limit)
//         .offset(offset)
//         .into_model::<ArticleWithAuthor>()
//         .all(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let articles_dto = ArticlesDto { articles: articles };
//     Ok(Json(articles_dto))
// }

// async fn get_article(
//     State(db): State<DatabaseConnection>,
//     maybe_token: Option<Extension<Token>>,
//     Path(slug): Path<String>,
// ) -> Result<Json<ArticleDto>, (StatusCode, String)> {
//     let current_user = if let Some(token) = maybe_token {
//         User::find()
//             .filter(user::Column::Username.eq(&token.username))
//             .one(&db)
//             .await
//             .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
//     } else {
//         None
//     };

//     let article = Article::find()
//         .filter(article::Column::Slug.eq(slug))
//         .join(JoinType::LeftJoin, article::Relation::User.def())
//         .column(user::Column::Username)
//         .column_as(
//             user::Column::Id.in_subquery(
//                 User::find()
//                     .join(JoinType::InnerJoin, follower::Relation::User1.def().rev())
//                     .select_only()
//                     .column(follower::Column::UserId)
//                     .filter(match &current_user {
//                         Some(user) => follower::Column::FollowerId.eq(user.id),
//                         None => false.into(),
//                     })
//                     .into_query(),
//             ),
//             "following",
//         )
//         .column_as(
//             article::Column::Id.in_subquery(
//                 FavoritedArticle::find()
//                     .select_only()
//                     .column(favorited_article::Column::ArticleId)
//                     .filter(match current_user {
//                         Some(user) => favorited_article::Column::UserId.eq(user.id),
//                         None => false.into(),
//                     })
//                     .into_query(),
//             ),
//             "favorited",
//         )
//         .join(
//             JoinType::LeftJoin,
//             favorited_article::Relation::Article.def().rev(),
//         )
//         .column_as(
//             Expr::count(Expr::col(favorited_article::Column::ArticleId))
//                 .cast_as(Alias::new("Integer")),
//             "favorites_count",
//         )
//         .group_by(favorited_article::Column::ArticleId)
//         .group_by(article::Column::Id)
//         .group_by(user::Column::Username)
//         .group_by(user::Column::Id)
//         .into_model::<ArticleWithAuthor>()
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let article_dto = ArticleDto { article: article };
//     Ok(Json(article_dto))
// }

// #[derive(Clone, Debug, Deserialize)]
// #[serde(rename_all = "camelCase")]
// struct CreateArticle {
//     title: String,
//     description: String,
//     body: String,
//     tag_list: Option<Vec<String>>,
// }

// #[derive(Debug, Deserialize)]
// struct CreateArticleDto {
//     article: CreateArticle,
// }

// async fn create_article(
//     State(db): State<DatabaseConnection>,
//     Extension(token): Extension<Token>,
//     Json(payload): Json<CreateArticleDto>,
// ) -> Result<Json<ArticleDto>, (StatusCode, String)> {
//     let current_user = User::find()
//         .filter(user::Column::Username.eq(token.username))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let current_user_id = current_user.unwrap().id;

//     let input = payload.article;

//     let article_model = article::ActiveModel {
//         slug: Set(slugify(&input.title)),
//         title: Set(input.title),
//         description: Set(input.description),
//         body: Set(input.body),
//         author_id: Set(current_user_id as i32),
//         ..Default::default()
//     };

//     let art_res = Article::insert::<article::ActiveModel>(article_model)
//         .exec(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let tag_models = if let Some(tgs) = &input.tag_list {
//         tgs.iter()
//             .map(|tg| tag::ActiveModel {
//                 tag_name: Set(tg.to_owned()),
//                 ..Default::default()
//             })
//             .collect()
//     } else {
//         vec![]
//     };

//     let _tag_res = Tag::insert_many(tag_models)
//         .on_empty_do_nothing()
//         .exec(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     // Find tags ids
//     let tags_ids = Tag::find()
//         .filter(
//             Expr::expr(Expr::col(tag::Column::TagName).cast_as(Alias::new("text")))
//                 .is_in(input.tag_list.clone().unwrap_or_default()),
//         )
//         .all(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let article_tag_models = tags_ids
//         .iter()
//         .map(|x| article_tag::ActiveModel {
//             tag_id: Set(x.id),
//             article_id: Set(art_res.last_insert_id),
//         })
//         .collect::<Vec<article_tag::ActiveModel>>();

//     let _article_tag_res = ArticleTag::insert_many(article_tag_models)
//         .on_empty_do_nothing()
//         .exec(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let mut article = Article::find_by_id(art_res.last_insert_id)
//         .join(JoinType::LeftJoin, article::Relation::User.def())
//         .column(user::Column::Username)
//         .column_as(
//             user::Column::Id.in_subquery(
//                 User::find()
//                     .join(JoinType::InnerJoin, follower::Relation::User1.def().rev())
//                     .select_only()
//                     .column(follower::Column::UserId)
//                     .filter(follower::Column::FollowerId.eq(current_user_id))
//                     .into_query(),
//             ),
//             "following",
//         )
//         .column_as(
//             article::Column::Id.in_subquery(
//                 FavoritedArticle::find()
//                     .select_only()
//                     .column(favorited_article::Column::ArticleId)
//                     .filter(favorited_article::Column::UserId.eq(current_user_id))
//                     .into_query(),
//             ),
//             "favorited",
//         )
//         .join(
//             JoinType::LeftJoin,
//             favorited_article::Relation::Article.def().rev(),
//         )
//         .column_as(
//             Expr::count(Expr::col(favorited_article::Column::ArticleId))
//                 .cast_as(Alias::new("Integer")),
//             "favorites_count",
//         )
//         .group_by(favorited_article::Column::ArticleId)
//         .group_by(article::Column::Id)
//         .group_by(user::Column::Username)
//         .group_by(user::Column::Id)
//         .into_model::<ArticleWithAuthor>()
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     article.as_mut().unwrap().tag_list = input.tag_list;

//     let article_dto = ArticleDto { article: article };
//     Ok(Json(article_dto))
// }

// #[derive(Clone, Debug, Deserialize)]
// #[serde(rename_all = "camelCase")]
// struct UpdateArticle {
//     title: Option<String>,
//     description: Option<String>,
//     body: Option<String>,
// }

// #[derive(Debug, Deserialize)]
// struct UpdateArticleDto {
//     article: UpdateArticle,
// }

// async fn update_article(
//     Path(slug): Path<String>,
//     State(db): State<DatabaseConnection>,
//     Extension(token): Extension<Token>,
//     Json(payload): Json<UpdateArticleDto>,
// ) -> Result<Json<ArticleDto>, (StatusCode, String)> {
//     let current_user = User::find()
//         .filter(user::Column::Username.eq(token.username))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let current_user_id = current_user.unwrap().id;

//     let input = payload.article;

//     let finded: Option<article::Model> = Article::find()
//         .filter(article::Column::Slug.eq(slug))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let mut article_model: article::ActiveModel = finded.unwrap().into();

//     if input.title.is_some() {
//         article_model.slug = Set(slugify(input.title.as_ref().unwrap()));
//         article_model.title = Set(input.title.to_owned().unwrap());
//     }
//     if input.description.is_some() {
//         article_model.description = Set(input.description.to_owned().unwrap());
//     }
//     if input.body.is_some() {
//         article_model.body = Set(input.body.to_owned().unwrap());
//     }

//     if vec![&input.title, &input.description, &input.body]
//         .iter()
//         .any(|fld| fld.is_some())
//     {
//         let time = DateTime::from_timestamp_millis(Local::now().timestamp_millis()).unwrap();
//         article_model.updated_at = Set(Some(time));
//     }

//     let art_res = article_model
//         .update(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let mut article = Article::find_by_id(art_res.id)
//         .join(JoinType::LeftJoin, article::Relation::User.def())
//         .column(user::Column::Username)
//         .column_as(
//             user::Column::Id.in_subquery(
//                 User::find()
//                     .join(JoinType::InnerJoin, follower::Relation::User1.def().rev())
//                     .select_only()
//                     .column(follower::Column::UserId)
//                     .filter(follower::Column::FollowerId.eq(current_user_id))
//                     .into_query(),
//             ),
//             "following",
//         )
//         .column_as(
//             article::Column::Id.in_subquery(
//                 FavoritedArticle::find()
//                     .select_only()
//                     .column(favorited_article::Column::ArticleId)
//                     .filter(favorited_article::Column::UserId.eq(current_user_id))
//                     .into_query(),
//             ),
//             "favorited",
//         )
//         .join(
//             JoinType::LeftJoin,
//             favorited_article::Relation::Article.def().rev(),
//         )
//         .column_as(
//             Expr::count(Expr::col(favorited_article::Column::ArticleId))
//                 .cast_as(Alias::new("Integer")),
//             "favorites_count",
//         )
//         .group_by(favorited_article::Column::ArticleId)
//         .group_by(article::Column::Id)
//         .group_by(user::Column::Username)
//         .group_by(user::Column::Id)
//         .into_model::<ArticleWithAuthor>()
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let tags = ArticleTag::find()
//         .join(JoinType::LeftJoin, article_tag::Relation::Tag.def())
//         .filter(article_tag::Column::ArticleId.eq(art_res.id))
//         .select_only()
//         .column(tag::Column::TagName)
//         .into_tuple::<String>()
//         .all(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     article.as_mut().unwrap().tag_list = Some(tags);

//     let article_dto = ArticleDto { article: article };
//     Ok(Json(article_dto))
// }

// async fn delete_article(
//     Path(slug): Path<String>,
//     State(db): State<DatabaseConnection>,
// ) -> Result<Json<()>, (StatusCode, String)> {
//     let finded: Option<article::Model> = Article::find()
//         .filter(article::Column::Slug.eq(slug))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let article_model: article::ActiveModel = finded.unwrap().into();

//     let _art_res = article_model
//         .delete(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     Ok(Json(()))
// }

// async fn favorite_article(
//     Path(slug): Path<String>,
//     Extension(token): Extension<Token>,
//     State(db): State<DatabaseConnection>,
// ) -> Result<Json<ArticleDto>, (StatusCode, String)> {
//     let current_user = User::find()
//         .filter(user::Column::Username.eq(token.username))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let current_user_id = current_user.unwrap().id;

//     let finded: Option<article::Model> = Article::find()
//         .filter(article::Column::Slug.eq(slug))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let favorite_article_model = favorited_article::ActiveModel {
//         article_id: Set(finded.as_ref().unwrap().id),
//         user_id: Set(current_user_id as i32),
//     };

//     let _art_res = favorite_article_model
//         .insert(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let mut article = Article::find_by_id(finded.as_ref().unwrap().id)
//         .join(JoinType::LeftJoin, article::Relation::User.def())
//         .column(user::Column::Username)
//         .column_as(
//             user::Column::Id.in_subquery(
//                 User::find()
//                     .join(JoinType::InnerJoin, follower::Relation::User1.def().rev())
//                     .select_only()
//                     .column(follower::Column::UserId)
//                     .filter(follower::Column::FollowerId.eq(current_user_id))
//                     .into_query(),
//             ),
//             "following",
//         )
//         .column_as(
//             article::Column::Id.in_subquery(
//                 FavoritedArticle::find()
//                     .select_only()
//                     .column(favorited_article::Column::ArticleId)
//                     .filter(favorited_article::Column::UserId.eq(current_user_id))
//                     .into_query(),
//             ),
//             "favorited",
//         )
//         .join(
//             JoinType::LeftJoin,
//             favorited_article::Relation::Article.def().rev(),
//         )
//         .column_as(
//             Expr::count(Expr::col(favorited_article::Column::ArticleId))
//                 .cast_as(Alias::new("Integer")),
//             "favorites_count",
//         )
//         .group_by(favorited_article::Column::ArticleId)
//         .group_by(article::Column::Id)
//         .group_by(user::Column::Username)
//         .group_by(user::Column::Id)
//         .into_model::<ArticleWithAuthor>()
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let tags = ArticleTag::find()
//         .join(JoinType::LeftJoin, article_tag::Relation::Tag.def())
//         .filter(article_tag::Column::ArticleId.eq(finded.as_ref().unwrap().id))
//         .select_only()
//         .column(tag::Column::TagName)
//         .into_tuple::<String>()
//         .all(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     article.as_mut().unwrap().tag_list = Some(tags);

//     let article_dto = ArticleDto { article: article };
//     Ok(Json(article_dto))
// }

// async fn unfavorite_article(
//     Path(slug): Path<String>,
//     Extension(token): Extension<Token>,
//     State(db): State<DatabaseConnection>,
// ) -> Result<Json<ArticleDto>, (StatusCode, String)> {
//     let current_user = User::find()
//         .filter(user::Column::Username.eq(token.username))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let current_user_id = current_user.unwrap().id;

//     let finded: Option<article::Model> = Article::find()
//         .filter(article::Column::Slug.eq(slug))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let favorite_article_model = favorited_article::ActiveModel {
//         article_id: Set(finded.as_ref().unwrap().id),
//         user_id: Set(current_user_id as i32),
//     };

//     let _art_res = favorite_article_model
//         .delete(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let mut article = Article::find_by_id(finded.as_ref().unwrap().id)
//         .join(JoinType::LeftJoin, article::Relation::User.def())
//         .column(user::Column::Username)
//         .column_as(
//             user::Column::Id.in_subquery(
//                 User::find()
//                     .join(JoinType::InnerJoin, follower::Relation::User1.def().rev())
//                     .select_only()
//                     .column(follower::Column::UserId)
//                     .filter(follower::Column::FollowerId.eq(current_user_id))
//                     .into_query(),
//             ),
//             "following",
//         )
//         .column_as(
//             article::Column::Id.in_subquery(
//                 FavoritedArticle::find()
//                     .select_only()
//                     .column(favorited_article::Column::ArticleId)
//                     .filter(favorited_article::Column::UserId.eq(current_user_id))
//                     .into_query(),
//             ),
//             "favorited",
//         )
//         .join(
//             JoinType::LeftJoin,
//             favorited_article::Relation::Article.def().rev(),
//         )
//         .column_as(
//             Expr::count(Expr::col(favorited_article::Column::ArticleId))
//                 .cast_as(Alias::new("Integer")),
//             "favorites_count",
//         )
//         .group_by(favorited_article::Column::ArticleId)
//         .group_by(article::Column::Id)
//         .group_by(user::Column::Username)
//         .group_by(user::Column::Id)
//         .into_model::<ArticleWithAuthor>()
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let tags = ArticleTag::find()
//         .join(JoinType::LeftJoin, article_tag::Relation::Tag.def())
//         .filter(article_tag::Column::ArticleId.eq(finded.as_ref().unwrap().id))
//         .select_only()
//         .column(tag::Column::TagName)
//         .into_tuple::<String>()
//         .all(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     article.as_mut().unwrap().tag_list = Some(tags);

//     let article_dto = ArticleDto { article: article };
//     Ok(Json(article_dto))
// }

// #[derive(Debug, Serialize)]
// struct CommentsDto {
//     comments: Vec<CommentWithAuthor>,
// }

// #[derive(Debug, Serialize)]
// struct CommentDto {
//     comment: Option<CommentWithAuthor>,
// }

// #[derive(Debug, Serialize)]
// #[serde(rename_all = "camelCase")]
// struct CommentWithAuthor {
//     id: i32,
//     body: String,
//     created_at: Option<DateTime>,
//     updated_at: Option<DateTime>,
//     author: Profile,
// }

// #[derive(Clone, Debug, Deserialize)]
// #[serde(rename_all = "camelCase")]
// struct CreateComment {
//     body: String,
// }

// #[derive(Debug, Deserialize)]
// struct CreateCommentDto {
//     comment: CreateComment,
// }

// impl FromQueryResult for CommentWithAuthor {
//     fn from_query_result(res: &sea_orm::QueryResult, pre: &str) -> Result<Self, sea_orm::DbErr> {
//         Ok(Self {
//             id: res.try_get(pre, "id")?,
//             body: res.try_get(pre, "body")?,
//             created_at: res.try_get(pre, "created_at")?,
//             updated_at: res.try_get(pre, "updated_at")?,
//             author: Profile::from_query_result(res, pre)?,
//         })
//     }
// }

// async fn create_comment(
//     Path(slug): Path<String>,
//     State(db): State<DatabaseConnection>,
//     Extension(token): Extension<Token>,
//     Json(payload): Json<CreateCommentDto>,
// ) -> Result<Json<CommentDto>, (StatusCode, String)> {
//     let current_user = User::find()
//         .filter(user::Column::Username.eq(token.username))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let current_user_id = current_user.unwrap().id;

//     let input = payload.comment;

//     // Find Article
//     let finded: Option<article::Model> = Article::find()
//         .filter(article::Column::Slug.eq(slug))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let comment_model = comment::ActiveModel {
//         body: Set(input.body),
//         author_id: Set(current_user_id as i32),
//         article_id: Set(finded.unwrap().id),
//         ..Default::default()
//     };

//     let cmnt_res = Comment::insert::<comment::ActiveModel>(comment_model)
//         .exec(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let comment = Comment::find_by_id(cmnt_res.last_insert_id)
//         .join(JoinType::LeftJoin, comment::Relation::User.def())
//         .column(user::Column::Username)
//         .column_as(
//             user::Column::Id.in_subquery(
//                 User::find()
//                     .join(JoinType::InnerJoin, follower::Relation::User1.def().rev())
//                     .select_only()
//                     .column(follower::Column::UserId)
//                     .filter(follower::Column::FollowerId.eq(current_user_id))
//                     .into_query(),
//             ),
//             "following",
//         )
//         .into_model::<CommentWithAuthor>()
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let comment_dto = CommentDto { comment: comment };
//     Ok(Json(comment_dto))
// }

// async fn list_comments(
//     Path(slug): Path<String>,
//     maybe_token: Option<Extension<Token>>,
//     State(db): State<DatabaseConnection>,
// ) -> Result<Json<CommentsDto>, (StatusCode, String)> {
//     let current_user = if let Some(token) = maybe_token {
//         User::find()
//             .filter(user::Column::Username.eq(&token.username))
//             .one(&db)
//             .await
//             .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
//     } else {
//         None
//     };

//     // Find Article
//     let finded: Option<article::Model> = Article::find()
//         .filter(article::Column::Slug.eq(slug))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let comments = Comment::find()
//         .join(JoinType::LeftJoin, comment::Relation::User.def())
//         .filter(comment::Column::ArticleId.eq(finded.unwrap().id))
//         .column(user::Column::Username)
//         .column_as(
//             user::Column::Id.in_subquery(
//                 User::find()
//                     .join(JoinType::InnerJoin, follower::Relation::User1.def().rev())
//                     .select_only()
//                     .column(follower::Column::UserId)
//                     .filter(match &current_user {
//                         Some(user) => follower::Column::FollowerId.eq(user.id),
//                         None => false.into(),
//                     })
//                     .into_query(),
//             ),
//             "following",
//         )
//         .into_model::<CommentWithAuthor>()
//         .all(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let comments_dto = CommentsDto { comments: comments };
//     Ok(Json(comments_dto))
// }

// async fn delete_comment(
//     Path((slug, comment_id)): Path<(String, i32)>,
//     State(db): State<DatabaseConnection>,
// ) -> Result<Json<()>, (StatusCode, String)> {
//     let _finded: Option<article::Model> = Article::find()
//         .filter(article::Column::Slug.eq(slug))
//         .one(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let _delete_res = Comment::delete_by_id(comment_id)
//         .exec(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     Ok(Json(()))
// }

// #[derive(Debug, Serialize)]
// struct TagsDto {
//     tags: Vec<String>,
// }

// TTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTT

// async fn list_tags(
//     State(db): State<DatabaseConnection>,
// ) -> Result<Json<TagsDto>, (StatusCode, String)> {
//     let tags = Tag::find()
//         .select_only()
//         .column(tag::Column::TagName)
//         .into_tuple::<String>()
//         .all(&db)
//         .await
//         .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

//     let tags_dto = TagsDto { tags: tags };
//     Ok(Json(tags_dto))
// }

/// Return APP_PORT from environment varibles or defalt port (3000)
fn get_app_port() -> u16 {
    env::var(APP_PORT).map_or(DEFAULT_APP_PORT, |port| {
        port.parse().unwrap_or(DEFAULT_APP_PORT)
    })
}

/// Return socket address from environment varibles or defalt port (3000)
fn get_socket_address() -> SocketAddr {
    let app_port = get_app_port();
    let host = env::var(APP_HOST).map_or(DEFAULT_APP_HOST.to_string(), |host| {
        if !host.is_empty() {
            host
        } else {
            DEFAULT_APP_HOST.to_string()
        }
    });

    SocketAddr::from((IpAddr::from_str(&host).unwrap(), app_port))
}

#[cfg(test)]
mod get_app_port_tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn when_env_set() {
        env::set_var(APP_PORT, "1234");
        assert_eq!(get_app_port(), 1234);
    }

    #[test]
    #[serial]
    fn when_env_set_empty() {
        env::set_var(APP_PORT, "");
        assert_eq!(get_app_port(), DEFAULT_APP_PORT);
    }

    #[test]
    #[serial]
    fn when_env_not_set() {
        env::remove_var(APP_PORT);
        assert_eq!(get_app_port(), DEFAULT_APP_PORT);
    }
}

#[cfg(test)]
mod get_socket_address_tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn when_env_set() {
        env::set_var(APP_HOST, "0.0.0.0");
        env::set_var(APP_PORT, "3000");
        assert_eq!(Ok(get_socket_address()), "0.0.0.0:3000".parse());
    }

    #[test]
    #[serial]
    fn when_env_set_empty() {
        env::set_var(APP_HOST, "");
        env::set_var(APP_PORT, "3000");
        let expected = format!("{DEFAULT_APP_HOST}:3000");
        assert_eq!(Ok(get_socket_address()), expected.parse());
    }

    #[test]
    #[serial]
    fn when_env_not_set() {
        env::remove_var(APP_HOST);
        env::set_var(APP_PORT, "3000");
        let expected = format!("{DEFAULT_APP_HOST}:3000");
        assert_eq!(Ok(get_socket_address()), expected.parse());
    }
}
