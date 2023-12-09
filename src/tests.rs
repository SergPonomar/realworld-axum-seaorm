use chrono::{Duration, Local};
use entity::entities::{
    article, article_tag, comment, favorited_article, follower,
    prelude::{Article, ArticleTag, Comment, FavoritedArticle, Follower, Tag, User},
    tag, user,
};
use migration::{Migrator, MigratorTrait, SchemaManager};
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, DbErr, EntityTrait};
use std::{convert::From, error::Error, fmt};
use uuid::Uuid;

pub async fn init_test_db_connection() -> Result<DatabaseConnection, DbErr> {
    Database::connect("sqlite::memory:").await
}

// pub async fn create_table_for_test_db<E>(
//     connection: &DatabaseConnection,
//     entity: E,
// ) -> Result<ExecResult, DbErr>
// where
//     E: EntityTrait,
// {
//     let schema = Schema::new(DbBackend::Sqlite);
//     let stmt: TableCreateStatement = schema.create_table_from_entity(entity);

//     // Create table from entity
//     connection
//         .execute(connection.get_database_backend().build(&stmt))
//         .await
// }

/// Execute migration for provided connection. Useful for table creation.
pub async fn execute_migration(
    connection: &DatabaseConnection,
    migration_name: &str,
) -> Result<(), DbErr> {
    let manager = SchemaManager::new(connection);
    let migrations = Migrator::migrations();
    let migration = migrations
        .iter()
        .find(|mgr| mgr.name() == migration_name)
        .unwrap();

    migration.up(&manager).await
}

/// struct for creating test data
#[derive(Default, Debug, PartialEq)]
pub struct TestDataBuilder {
    users: Option<Vec<user::Model>>,
    articles: Option<Vec<article::Model>>,
    comments: Option<Vec<comment::Model>>,
    tags: Option<Vec<tag::Model>>,
    article_tags: Option<Vec<article_tag::Model>>,
    followers: Option<Vec<follower::Model>>,
    favorited_articles: Option<Vec<favorited_article::Model>>,
    error: Option<BldrErr>,
    only_models: bool,
}

pub struct RelUser(Vec<usize>);
pub struct RelAuthorArticle(Vec<(usize, usize)>);
pub struct RelArticleTag(Vec<(usize, usize)>);
pub struct RelUserFollower(pub Vec<(usize, usize)>);
pub struct RelArticleUser(Vec<(usize, usize)>);

/// error reterned by TestDataBuilder
#[derive(Debug, PartialEq)]
pub enum BldrErr {
    ZeroQty,
    EmptyRel,
    WrongOrder(String, String),
    OutOfRange(String, usize),
    InsertErr(DbErr),
    ConnErr(DbErr),
    DbErr(DbErr),
}

impl From<DbErr> for BldrErr {
    fn from(err: DbErr) -> BldrErr {
        match err {
            DbErr::RecordNotInserted => BldrErr::InsertErr(err),
            _ => BldrErr::ConnErr(err),
        }
    }
}

impl fmt::Display for BldrErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BldrErr::ZeroQty => write!(f, "qty parameter should be greater then zero"),
            BldrErr::EmptyRel => write!(f, "relations parameter should be not empty"),
            BldrErr::WrongOrder(before, after) => {
                write!(f, "{before} should be set before {after}")
            }
            BldrErr::OutOfRange(entity, high) => write!(
                f,
                "{entity} number should be between 1 and {high} inclusive"
            ),
            BldrErr::InsertErr(..) => write!(f, "unable to insert data"),
            BldrErr::ConnErr(..) => write!(f, "no connection was established"),
            BldrErr::DbErr(..) => write!(f, "orm error"),
        }
    }
}

impl Error for BldrErr {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            BldrErr::ZeroQty => None,
            BldrErr::EmptyRel => None,
            BldrErr::WrongOrder(..) => None,
            BldrErr::OutOfRange(..) => None,
            BldrErr::InsertErr(ref e) => Some(e),
            BldrErr::ConnErr(ref e) => Some(e),
            BldrErr::DbErr(ref e) => Some(e),
        }
    }
}

impl TestDataBuilder {
    pub fn new() -> Self {
        // UserOperation(Operation::Insert(RelUser(vec![3])));
        Self::default()
    }

    fn apply_error(mut self, err: BldrErr) -> Self {
        if self.error.is_none() {
            self.error = Some(err);
        }
        self
    }

    pub fn users(mut self, qty: usize) -> Self {
        if qty == 0 {
            return self.apply_error(BldrErr::ZeroQty);
        }
        let users = (1..=qty)
            .map(|x| user::Model {
                id: Uuid::new_v4(),
                email: format!("email{x}"),
                username: format!("username{x}"),
                bio: Some("bio".to_owned()),
                image: Some("image".to_owned()),
                password: "password".to_owned(),
            })
            .collect();
        self.users = Some(users);
        self
    }

    pub fn articles(mut self, relations: RelUser) -> Self {
        let values = relations.0;
        if values.is_empty() {
            return self.apply_error(BldrErr::EmptyRel);
        }
        if self.users.is_none() {
            return self.apply_error(BldrErr::WrongOrder(
                "users".to_owned(),
                "articles".to_owned(),
            ));
        }
        let users_len = self.users.as_ref().unwrap().len();
        if !values.iter().all(|&x| x >= 1 && x <= users_len) {
            return self.apply_error(BldrErr::OutOfRange("user".to_owned(), users_len));
        }
        let articles = values
            .iter()
            .enumerate()
            .map(|(idx, val)| {
                let current_time = (Local::now() + Duration::seconds(idx as i64 + 1)).naive_local();
                article::Model {
                    id: Uuid::new_v4(),
                    slug: format!("title{idx}"),
                    title: format!("title{idx}"),
                    description: "description".to_owned(),
                    body: "body".to_owned(),
                    author_id: self.users.as_ref().unwrap()[*val as usize - 1].id,
                    created_at: Some(current_time),
                    updated_at: Some(current_time),
                }
            })
            .collect();
        self.articles = Some(articles);
        self
    }

    pub fn comments(mut self, relations: RelAuthorArticle) -> Self {
        let values = relations.0;
        if values.is_empty() {
            return self.apply_error(BldrErr::EmptyRel);
        }
        if self.articles.is_none() {
            return self.apply_error(BldrErr::WrongOrder(
                "articles".to_owned(),
                "comments".to_owned(),
            ));
        }
        let users_len = self.users.as_ref().unwrap().len();
        if !values
            .iter()
            .all(|&(author, _)| author >= 1 && author <= users_len)
        {
            return self.apply_error(BldrErr::OutOfRange("author".to_owned(), users_len));
        }
        let articles_len = self.articles.as_ref().unwrap().len();
        if !values
            .iter()
            .all(|&(_, article)| article >= 1 && article <= articles_len)
        {
            return self.apply_error(BldrErr::OutOfRange("article".to_owned(), articles_len));
        }

        let comments = values
            .iter()
            .enumerate()
            .map(|(idx, (author, article))| {
                let current_time = (Local::now() + Duration::seconds(idx as i64 + 1)).naive_local();
                comment::Model {
                    id: Uuid::new_v4(),
                    body: format!("comment{idx}"),
                    author_id: self.users.as_ref().unwrap()[*author as usize - 1].id,
                    article_id: self.articles.as_ref().unwrap()[*article as usize - 1].id,
                    created_at: Some(current_time),
                    updated_at: Some(current_time),
                }
            })
            .collect();
        self.comments = Some(comments);
        self
    }

    pub fn tags(mut self, qty: usize) -> Self {
        if qty == 0 {
            return self.apply_error(BldrErr::ZeroQty);
        }
        let tags = (1..=qty)
            .map(|x| tag::Model {
                id: Uuid::new_v4(),
                tag_name: format!("tag_name{x}"),
            })
            .collect();
        self.tags = Some(tags);
        self
    }

    pub fn article_tags(mut self, relations: RelArticleTag) -> Self {
        let values = relations.0;
        if values.is_empty() {
            return self.apply_error(BldrErr::EmptyRel);
        }
        if self.articles.is_none() {
            return self.apply_error(BldrErr::WrongOrder(
                "articles".to_owned(),
                "article_tags".to_owned(),
            ));
        }
        if self.tags.is_none() {
            return self.apply_error(BldrErr::WrongOrder(
                "tags".to_owned(),
                "article_tags".to_owned(),
            ));
        }
        let articles_len = self.articles.as_ref().unwrap().len();
        if !values
            .iter()
            .all(|&(article, _)| article >= 1 && article <= articles_len)
        {
            return self.apply_error(BldrErr::OutOfRange("article".to_owned(), articles_len));
        }
        let tags_len = self.tags.as_ref().unwrap().len();
        if !values.iter().all(|&(_, tag)| tag >= 1 && tag <= tags_len) {
            return self.apply_error(BldrErr::OutOfRange("tag".to_owned(), tags_len));
        }

        let article_tags = values
            .iter()
            .map(|(article, tag)| article_tag::Model {
                article_id: self.articles.as_ref().unwrap()[*article as usize - 1].id,
                tag_id: self.tags.as_ref().unwrap()[*tag as usize - 1].id,
            })
            .collect();
        self.article_tags = Some(article_tags);
        self
    }

    pub fn followers(mut self, relations: RelUserFollower) -> Self {
        let values = relations.0;
        if values.is_empty() {
            return self.apply_error(BldrErr::EmptyRel);
        }
        if self.users.is_none() {
            return self.apply_error(BldrErr::WrongOrder(
                "users".to_owned(),
                "followers".to_owned(),
            ));
        }
        let users_len = self.users.as_ref().unwrap().len();
        if !values
            .iter()
            .all(|&(user, _)| user >= 1 && user <= users_len)
        {
            return self.apply_error(BldrErr::OutOfRange("user".to_owned(), users_len));
        }
        if !values
            .iter()
            .all(|&(_, follower)| follower >= 1 && follower <= users_len)
        {
            return self.apply_error(BldrErr::OutOfRange("follower".to_owned(), users_len));
        }

        let followers = values
            .iter()
            .map(|(user, follower)| follower::Model {
                user_id: self.users.as_ref().unwrap()[*user as usize - 1].id,
                follower_id: self.users.as_ref().unwrap()[*follower as usize - 1].id,
            })
            .collect();
        self.followers = Some(followers);
        self
    }

    pub fn favorited_articles(mut self, relations: RelArticleUser) -> Self {
        let values = relations.0;
        if values.is_empty() {
            return self.apply_error(BldrErr::EmptyRel);
        }
        if self.articles.is_none() {
            return self.apply_error(BldrErr::WrongOrder(
                "articles".to_owned(),
                "favorited_articles".to_owned(),
            ));
        }
        let articles_len = self.articles.as_ref().unwrap().len();
        if !values
            .iter()
            .all(|&(article, _)| article >= 1 && article <= articles_len)
        {
            return self.apply_error(BldrErr::OutOfRange("article".to_owned(), articles_len));
        }
        let users_len = self.users.as_ref().unwrap().len();
        if !values
            .iter()
            .all(|&(_, user)| user >= 1 && user <= users_len)
        {
            return self.apply_error(BldrErr::OutOfRange("user".to_owned(), users_len));
        }

        let favorited_articles = values
            .iter()
            .map(|(article, user)| favorited_article::Model {
                article_id: self.articles.as_ref().unwrap()[*article as usize - 1].id,
                user_id: self.users.as_ref().unwrap()[*user as usize - 1].id,
            })
            .collect();
        self.favorited_articles = Some(favorited_articles);
        self
    }

    async fn insert<E: EntityTrait, AM: ActiveModelTrait<Entity = E> + From<E::Model>>(
        &self,
        db: &DatabaseConnection,
        migrations: Vec<&str>,
        models: &Option<Vec<E::Model>>,
    ) -> Result<(), DbErr> {
        if models.is_none() {
            return Ok(());
        }
        for migration in migrations {
            execute_migration(db, migration).await?;
        }

        if !self.only_models {
            let actives = Self::activate_models::<E, AM>(models);
            E::insert_many(actives).exec(db).await?;
        }

        Ok(())
    }

    pub fn activate_models<E: EntityTrait, AM: ActiveModelTrait<Entity = E> + From<E::Model>>(
        models: &Option<Vec<E::Model>>,
    ) -> Vec<AM> {
        models
            .as_ref()
            .unwrap()
            .iter()
            .cloned()
            .map(|mdl| mdl.into())
            .map(|mdl: AM| mdl.reset_all())
            .collect()
    }

    pub fn only_models(mut self) -> Self {
        self.only_models = true;
        self
    }

    pub async fn build(self) -> Result<(DatabaseConnection, TestData), BldrErr> {
        let connection = init_test_db_connection().await?;

        self.insert::<User, user::ActiveModel>(
            &connection,
            vec![
                "m20231030_000001_create_user_table",
                "m20231112_000008_add_user_password",
            ],
            &self.users,
        )
        .await?;

        self.insert::<Article, article::ActiveModel>(
            &connection,
            vec!["m20231030_000002_create_article_table"],
            &self.articles,
        )
        .await?;

        self.insert::<Comment, comment::ActiveModel>(
            &connection,
            vec!["m20231030_000003_create_comment_table"],
            &self.comments,
        )
        .await?;

        self.insert::<Tag, tag::ActiveModel>(
            &connection,
            vec!["m20231030_000004_create_tag_table"],
            &self.tags,
        )
        .await?;

        self.insert::<ArticleTag, article_tag::ActiveModel>(
            &connection,
            vec!["m20231030_000005_create_article_tag_table"],
            &self.article_tags,
        )
        .await?;

        self.insert::<Follower, follower::ActiveModel>(
            &connection,
            vec!["m20231101_000006_create_follower_table"],
            &self.followers,
        )
        .await?;

        self.insert::<FavoritedArticle, favorited_article::ActiveModel>(
            &connection,
            vec!["m20231104_000007_create_favorited_article_table"],
            &self.favorited_articles,
        )
        .await?;

        Ok((
            connection,
            TestData {
                users: self.users,
                articles: self.articles,
                comments: self.comments,
                tags: self.tags,
                article_tags: self.article_tags,
                followers: self.followers,
                favorited_articles: self.favorited_articles,
            },
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TestData {
    pub users: Option<Vec<user::Model>>,
    pub articles: Option<Vec<article::Model>>,
    pub comments: Option<Vec<comment::Model>>,
    pub tags: Option<Vec<tag::Model>>,
    pub article_tags: Option<Vec<article_tag::Model>>,
    pub followers: Option<Vec<follower::Model>>,
    pub favorited_articles: Option<Vec<favorited_article::Model>>,
}

#[cfg(test)]
mod test_test_data_builder {
    use super::*;
    use sea_orm::RuntimeErr;
    use std::vec;
    use uuid::Uuid;

    #[test]
    fn test_display_error() {
        assert_eq!(
            BldrErr::ZeroQty.to_string(),
            "qty parameter should be greater then zero".to_owned()
        );
        assert_eq!(
            BldrErr::EmptyRel.to_string(),
            "relations parameter should be not empty".to_owned()
        );
        assert_eq!(
            BldrErr::WrongOrder("cat".to_owned(), "dog".to_owned()).to_string(),
            "cat should be set before dog".to_owned()
        );

        assert_eq!(
            BldrErr::OutOfRange("cat".to_owned(), 95).to_string(),
            "cat number should be between 1 and 95 inclusive".to_owned()
        );

        assert_eq!(
            BldrErr::InsertErr(DbErr::RecordNotInserted).to_string(),
            "unable to insert data".to_owned()
        );

        assert_eq!(
            BldrErr::ConnErr(DbErr::Conn(RuntimeErr::Internal(
                "not connected".to_owned()
            )))
            .to_string(),
            "no connection was established".to_owned()
        );

        assert_eq!(
            BldrErr::DbErr(DbErr::Exec(RuntimeErr::Internal("error text".to_owned()))).to_string(),
            "orm error".to_owned()
        );
    }

    #[test]
    fn test_source_error() {
        assert_eq!(
            BldrErr::InsertErr(DbErr::RecordNotInserted)
                .source()
                .unwrap()
                .to_string(),
            "None of the records are inserted".to_owned()
        );

        assert_eq!(
            BldrErr::ConnErr(DbErr::Conn(RuntimeErr::Internal(
                "not connected".to_owned()
            )))
            .source()
            .unwrap()
            .to_string(),
            "Connection Error: not connected".to_owned()
        );

        assert_eq!(
            BldrErr::DbErr(DbErr::Exec(RuntimeErr::Internal("error text".to_owned())))
                .source()
                .unwrap()
                .to_string(),
            "Execution Error: error text".to_owned()
        );
    }

    #[test]
    fn test_new() {
        let tested = TestDataBuilder::new();
        let expected = TestDataBuilder {
            users: None,
            articles: None,
            comments: None,
            tags: None,
            article_tags: None,
            followers: None,
            favorited_articles: None,
            error: None,
            only_models: false,
        };
        assert_eq!(tested, expected);
    }

    #[test]
    fn test_apply_error() {
        let tested = TestDataBuilder::new().apply_error(BldrErr::EmptyRel);
        assert_eq!(tested.error, Some(BldrErr::EmptyRel));
    }

    #[test]
    fn test_apply_error_two_times() {
        let expected = BldrErr::EmptyRel;
        let tested = TestDataBuilder::new().apply_error(BldrErr::EmptyRel);
        let tested = tested.apply_error(BldrErr::ZeroQty);
        assert_eq!(tested.error, Some(expected));
    }

    // TEST USERS
    #[test]
    fn test_users() {
        let tested = TestDataBuilder::new().users(2);
        assert_eq!(tested.users.unwrap().len(), 2);
    }

    #[test]
    fn test_users_zero_qty() {
        let expected = TestDataBuilder {
            error: Some(BldrErr::ZeroQty),
            ..Default::default()
        };
        assert_eq!(TestDataBuilder::new().users(0), expected);
    }

    // TEST ARTICLES
    #[test]
    fn test_articles() {
        let tested = TestDataBuilder::new()
            .users(2)
            .articles(RelUser(vec![1, 2, 2]));
        assert_eq!(tested.articles.unwrap().len(), 3);
    }

    #[test]
    fn test_articles_users_not_set() {
        let expected = Some(BldrErr::WrongOrder(
            "users".to_owned(),
            "articles".to_owned(),
        ));
        let tested = TestDataBuilder::new().articles(RelUser(vec![1, 2, 2]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_articles_empty_input() {
        let expected = Some(BldrErr::EmptyRel);
        let tested = TestDataBuilder::new().articles(RelUser(vec![]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_articles_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("user".to_owned(), 2));
        let tested1 = TestDataBuilder::new().users(2).articles(RelUser(vec![0]));
        assert_eq!(tested1.error, expected);
        let tested2 = TestDataBuilder::new().users(2).articles(RelUser(vec![3]));
        assert_eq!(tested2.error, expected);
    }

    // TEST COMMENTS
    #[test]
    fn test_comments() {
        let tested = TestDataBuilder::new()
            .users(2)
            .articles(RelUser(vec![1, 2, 2]))
            .comments(RelAuthorArticle(vec![(1, 1), (2, 3), (2, 2)]));
        assert_eq!(tested.comments.unwrap().len(), 3);
    }

    #[test]
    fn test_comments_articles_not_set() {
        let expected = Some(BldrErr::WrongOrder(
            "articles".to_owned(),
            "comments".to_owned(),
        ));
        let tested = TestDataBuilder::new()
            .users(2)
            .comments(RelAuthorArticle(vec![(1, 2), (1, 2)]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_comments_empty_input() {
        let expected = Some(BldrErr::EmptyRel);
        let tested = TestDataBuilder::new().comments(RelAuthorArticle(vec![]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_author_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("author".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(2)
            .articles(RelUser(vec![1, 2]))
            .comments(RelAuthorArticle(vec![(0, 2)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(2)
            .articles(RelUser(vec![1, 2]))
            .comments(RelAuthorArticle(vec![(3, 2)]));
        assert_eq!(tested2.error, expected);
    }

    #[test]
    fn test_article_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("article".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(2)
            .articles(RelUser(vec![1, 2]))
            .comments(RelAuthorArticle(vec![(1, 0)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(2)
            .articles(RelUser(vec![1, 2]))
            .comments(RelAuthorArticle(vec![(1, 3)]));
        assert_eq!(tested2.error, expected);
    }

    // TEST TAGS
    #[test]
    fn test_tags() {
        let tested = TestDataBuilder::new().tags(2);
        assert_eq!(tested.tags.unwrap().len(), 2);
    }

    #[test]
    fn test_tags_zero_qty() {
        let expected = TestDataBuilder {
            error: Some(BldrErr::ZeroQty),
            ..Default::default()
        };
        assert_eq!(TestDataBuilder::new().tags(0), expected);
    }

    // TEST ARTICLE_TAGS
    #[test]
    fn test_article_tags() {
        let tested = TestDataBuilder::new()
            .users(2)
            .articles(RelUser(vec![1, 2, 2]))
            .tags(2)
            .article_tags(RelArticleTag(vec![(1, 1), (2, 2), (3, 2)]));
        assert_eq!(tested.article_tags.unwrap().len(), 3);
    }

    #[test]
    fn test_article_tags_articles_not_set() {
        let expected = Some(BldrErr::WrongOrder(
            "articles".to_owned(),
            "article_tags".to_owned(),
        ));
        let tested = TestDataBuilder::new()
            .tags(2)
            .article_tags(RelArticleTag(vec![(1, 2), (1, 2)]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_article_tags_tags_not_set() {
        let expected = Some(BldrErr::WrongOrder(
            "tags".to_owned(),
            "article_tags".to_owned(),
        ));
        let tested = TestDataBuilder::new()
            .users(3)
            .articles(RelUser(vec![1, 2, 2]))
            .article_tags(RelArticleTag(vec![(1, 2), (1, 2)]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_article_tags_empty_input() {
        let expected = Some(BldrErr::EmptyRel);
        let tested = TestDataBuilder::new().comments(RelAuthorArticle(vec![]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_article_tags_article_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("article".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(2)
            .tags(2)
            .articles(RelUser(vec![1, 2]))
            .article_tags(RelArticleTag(vec![(0, 1)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(2)
            .tags(2)
            .articles(RelUser(vec![1, 2]))
            .article_tags(RelArticleTag(vec![(3, 1)]));
        assert_eq!(tested2.error, expected);
    }

    #[test]
    fn test_article_tags_tag_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("tag".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(2)
            .tags(2)
            .articles(RelUser(vec![1, 2]))
            .article_tags(RelArticleTag(vec![(1, 0)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(2)
            .tags(2)
            .articles(RelUser(vec![1, 2]))
            .article_tags(RelArticleTag(vec![(2, 3)]));
        assert_eq!(tested2.error, expected);
    }

    // TEST FOLLOWER
    #[test]
    fn test_followers() {
        let tested = TestDataBuilder::new()
            .users(2)
            .followers(RelUserFollower(vec![(1, 2), (2, 1)]));
        assert_eq!(tested.followers.unwrap().len(), 2);
    }

    #[test]
    fn test_followers_users_not_set() {
        let expected = Some(BldrErr::WrongOrder(
            "users".to_owned(),
            "followers".to_owned(),
        ));
        let tested = TestDataBuilder::new().followers(RelUserFollower(vec![(1, 2), (2, 1)]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_followers_empty_input() {
        let expected = Some(BldrErr::EmptyRel);
        let tested = TestDataBuilder::new().followers(RelUserFollower(vec![]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_follower_user_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("user".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(2)
            .followers(RelUserFollower(vec![(0, 2)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(2)
            .followers(RelUserFollower(vec![(3, 2)]));
        assert_eq!(tested2.error, expected);
    }

    #[test]
    fn test_follower_follower_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("follower".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(2)
            .followers(RelUserFollower(vec![(1, 0)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(2)
            .followers(RelUserFollower(vec![(1, 3)]));
        assert_eq!(tested2.error, expected);
    }

    // TEST FAVORITED_ARTICLES
    #[test]
    fn test_favorited_articles() {
        let tested = TestDataBuilder::new()
            .users(2)
            .articles(RelUser(vec![1, 2, 2]))
            .favorited_articles(RelArticleUser(vec![(1, 1), (2, 2), (3, 2)]));
        assert_eq!(tested.favorited_articles.unwrap().len(), 3);
    }

    #[test]
    fn test_favorited_articles_articles_not_set() {
        let expected = Some(BldrErr::WrongOrder(
            "articles".to_owned(),
            "favorited_articles".to_owned(),
        ));
        let tested =
            TestDataBuilder::new().favorited_articles(RelArticleUser(vec![(1, 1), (2, 2), (3, 2)]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_favorited_articles_empty_input() {
        let expected = Some(BldrErr::EmptyRel);
        let tested = TestDataBuilder::new().favorited_articles(RelArticleUser(vec![]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_favorited_articles_article_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("article".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(2)
            .articles(RelUser(vec![1, 2]))
            .favorited_articles(RelArticleUser(vec![(0, 1)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(2)
            .tags(2)
            .articles(RelUser(vec![1, 2]))
            .favorited_articles(RelArticleUser(vec![(3, 1)]));
        assert_eq!(tested2.error, expected);
    }

    #[test]
    fn test_favorited_articles_users_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("user".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(2)
            .articles(RelUser(vec![1, 2]))
            .favorited_articles(RelArticleUser(vec![(2, 0)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(2)
            .articles(RelUser(vec![1, 2]))
            .favorited_articles(RelArticleUser(vec![(2, 3)]));
        assert_eq!(tested2.error, expected);
    }

    #[tokio::test]
    async fn test_insert() -> Result<(), BldrErr> {
        let connection = init_test_db_connection().await?;
        let expected: Vec<user::Model> = (0..5)
            .map(|x| user::Model {
                id: Uuid::new_v4(),
                email: format!("email{x}"),
                username: format!("username{x}"),
                bio: Some("bio".to_owned()),
                image: Some("image".to_owned()),
                password: "password".to_owned(),
            })
            .collect();

        TestDataBuilder::new()
            .insert::<User, user::ActiveModel>(
                &connection,
                vec![
                    "m20231030_000001_create_user_table",
                    "m20231112_000008_add_user_password",
                ],
                &Some(expected.clone()),
            )
            .await?;

        let tested = User::find().all(&connection).await?;
        assert_eq!(expected, tested);

        Ok(())
    }

    #[tokio::test]
    async fn test_build() -> Result<(), BldrErr> {
        let tested = TestDataBuilder::new().users(2).build().await?;
        assert_eq!(tested.1.users.unwrap().len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_build_no_inserts() -> Result<(), BldrErr> {
        let tested = TestDataBuilder::new().build().await?;
        let expected = TestData::default();
        assert_eq!(tested.1, expected);

        Ok(())
    }
}
