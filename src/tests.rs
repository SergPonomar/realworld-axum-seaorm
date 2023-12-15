use chrono::{Duration, Local};
use entity::entities::{
    article, article_tag, comment, favorited_article, follower,
    prelude::{Article, ArticleTag, Comment, FavoritedArticle, Follower, Tag, User},
    tag, user,
};
use migration::{Migrator, MigratorTrait, SchemaManager};
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, DbErr, EntityTrait};
use std::{convert::From, error::Error, fmt, matches, unreachable, vec};
use uuid::Uuid;

use crate::api::error::ApiErr;

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
    users: Option<Operation<Vec<user::Model>>>,
    articles: Option<Operation<Vec<article::Model>>>,
    comments: Option<Operation<Vec<comment::Model>>>,
    tags: Option<Operation<Vec<tag::Model>>>,
    article_tags: Option<Operation<Vec<article_tag::Model>>>,
    followers: Option<Operation<Vec<follower::Model>>>,
    favorited_articles: Option<Operation<Vec<favorited_article::Model>>>,
    error: Option<BldrErr>,
}

pub type Qty = usize;
pub type RelUser = Vec<usize>;
pub type RelAuthorArticle = Vec<(usize, usize)>;
pub type RelArticleTag = Vec<(usize, usize)>;
pub type RelUserFollower = Vec<(usize, usize)>;
pub type RelArticleUser = Vec<(usize, usize)>;

#[derive(Debug, Clone, PartialEq)]
pub enum Operation<T> {
    Insert(T),
    Create(T),
    Migration,
}

/// error returned by TestDataBuilder
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

/// error returned by Test
#[derive(Debug, PartialEq)]
pub enum TestErr {
    BldrErr(BldrErr),
    ApiErr(ApiErr),
    DbErr(DbErr),
}

impl From<BldrErr> for TestErr {
    fn from(err: BldrErr) -> TestErr {
        TestErr::BldrErr(err)
    }
}

impl From<ApiErr> for TestErr {
    fn from(err: ApiErr) -> TestErr {
        TestErr::ApiErr(err)
    }
}

impl From<DbErr> for TestErr {
    fn from(err: DbErr) -> TestErr {
        TestErr::DbErr(err)
    }
}

impl TestDataBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    fn apply_error(mut self, err: BldrErr) -> Self {
        if self.error.is_none() {
            self.error = Some(err);
        }
        self
    }

    pub fn users(mut self, operation: Operation<Qty>) -> Self {
        let gen_users = |qty| {
            (1..=qty)
                .map(|x| user::Model {
                    id: Uuid::new_v4(),
                    email: format!("email{x}"),
                    username: format!("username{x}"),
                    bio: Some("bio".to_owned()),
                    image: Some("image".to_owned()),
                    password: "password".to_owned(),
                })
                .collect()
        };

        if let Operation::Insert(0) | Operation::Create(0) = operation {
            return self.apply_error(BldrErr::ZeroQty);
        }

        let users = match operation {
            Operation::Insert(qty) => Operation::Insert(gen_users(qty)),
            Operation::Create(qty) => Operation::Create(gen_users(qty)),
            Operation::Migration => Operation::Migration,
        };

        self.users = Some(users);
        self
    }

    pub fn articles(mut self, operation: Operation<RelUser>) -> Self {
        if matches!(&operation, Operation::Insert(rels) | Operation::Create(rels) if rels.is_empty())
        {
            return self.apply_error(BldrErr::EmptyRel);
        }

        match (&operation, &self.users) {
            (Operation::Insert(rels), Some(Operation::Insert(mdls)))
            | (Operation::Create(rels), Some(Operation::Insert(mdls)))
            | (Operation::Create(rels), Some(Operation::Create(mdls))) => {
                let users_len = mdls.len();
                if !rels.iter().all(|&x| x >= 1 && x <= users_len) {
                    return self.apply_error(BldrErr::OutOfRange("user".to_owned(), users_len));
                }
            }
            (Operation::Migration, Some(_)) => (),
            _ => {
                return self.apply_error(BldrErr::WrongOrder(
                    "users".to_owned(),
                    "articles".to_owned(),
                ));
            }
        }

        let gen_articles = |relations: RelUser| {
            relations
                .iter()
                .enumerate()
                .map(|(idx, val)| {
                    let current_time =
                        (Local::now() + Duration::seconds(idx as i64 + 1)).naive_local();

                    match self.users.as_ref().unwrap() {
                        Operation::Insert(users) | Operation::Create(users) => article::Model {
                            id: Uuid::new_v4(),
                            slug: format!("title{}", idx + 1),
                            title: format!("title{}", idx + 1),
                            description: "description".to_owned(),
                            body: "body".to_owned(),
                            author_id: users[*val as usize - 1].id,
                            created_at: Some(current_time),
                            updated_at: Some(current_time),
                        },
                        _ => unreachable!(),
                    }
                })
                .collect()
        };

        let articles = match operation {
            Operation::Insert(rels) => Operation::Insert(gen_articles(rels)),
            Operation::Create(rels) => Operation::Create(gen_articles(rels)),
            Operation::Migration => Operation::Migration,
        };

        self.articles = Some(articles);
        self
    }

    pub fn comments(mut self, operation: Operation<RelAuthorArticle>) -> Self {
        if matches!(&operation, Operation::Insert(rels) | Operation::Create(rels) if rels.is_empty())
        {
            return self.apply_error(BldrErr::EmptyRel);
        }

        match (&operation, &self.users, &self.articles) {
            (
                Operation::Insert(rels),
                Some(Operation::Insert(usrs)),
                Some(Operation::Insert(artcls)),
            )
            | (
                Operation::Create(rels),
                Some(Operation::Create(usrs)),
                Some(Operation::Create(artcls)),
            )
            | (
                Operation::Create(rels),
                Some(Operation::Insert(usrs)),
                Some(Operation::Create(artcls)),
            )
            | (
                Operation::Create(rels),
                Some(Operation::Create(usrs)),
                Some(Operation::Insert(artcls)),
            )
            | (
                Operation::Create(rels),
                Some(Operation::Insert(usrs)),
                Some(Operation::Insert(artcls)),
            ) => {
                let users_len = usrs.len();
                if !rels
                    .iter()
                    .all(|&(author, _)| author >= 1 && author <= users_len)
                {
                    return self.apply_error(BldrErr::OutOfRange("author".to_owned(), users_len));
                }
                let articles_len = artcls.len();
                if !rels
                    .iter()
                    .all(|&(_, article)| article >= 1 && article <= articles_len)
                {
                    return self
                        .apply_error(BldrErr::OutOfRange("article".to_owned(), articles_len));
                }
            }
            (Operation::Migration, Some(_), Some(_)) => (),
            _ => {
                return self.apply_error(BldrErr::WrongOrder(
                    "articles".to_owned(),
                    "comments".to_owned(),
                ));
            }
        }

        let gen_comments = |relations: RelAuthorArticle| {
            relations
                .iter()
                .enumerate()
                .map(|(idx, (author, article))| {
                    let current_time =
                        (Local::now() + Duration::seconds(idx as i64 + 1)).naive_local();

                    match (
                        self.users.as_ref().unwrap(),
                        self.articles.as_ref().unwrap(),
                    ) {
                        (Operation::Insert(usrs), Operation::Insert(artcls))
                        | (Operation::Insert(usrs), Operation::Create(artcls))
                        | (Operation::Create(usrs), Operation::Create(artcls))
                        | (Operation::Create(usrs), Operation::Insert(artcls)) => comment::Model {
                            id: Uuid::new_v4(),
                            body: format!("comment{}", idx + 1),
                            author_id: usrs[*author as usize - 1].id,
                            article_id: artcls[*article as usize - 1].id,
                            created_at: Some(current_time),
                            updated_at: Some(current_time),
                        },
                        _ => unreachable!(),
                    }
                })
                .collect()
        };

        let comments = match operation {
            Operation::Insert(rels) => Operation::Insert(gen_comments(rels)),
            Operation::Create(rels) => Operation::Create(gen_comments(rels)),
            Operation::Migration => Operation::Migration,
        };

        self.comments = Some(comments);
        self
    }

    pub fn tags(mut self, operation: Operation<Qty>) -> Self {
        let gen_tags = |qty| {
            (1..=qty)
                .map(|x| tag::Model {
                    id: Uuid::new_v4(),
                    tag_name: format!("tag_name{x}"),
                })
                .collect()
        };

        if let Operation::Insert(0) | Operation::Create(0) = operation {
            return self.apply_error(BldrErr::ZeroQty);
        }

        let tags = match operation {
            Operation::Insert(qty) => Operation::Insert(gen_tags(qty)),
            Operation::Create(qty) => Operation::Create(gen_tags(qty)),
            Operation::Migration => Operation::Migration,
        };

        self.tags = Some(tags);
        self
    }

    pub fn article_tags(mut self, operation: Operation<RelArticleTag>) -> Self {
        if matches!(&operation, Operation::Insert(rels) | Operation::Create(rels) if rels.is_empty())
        {
            return self.apply_error(BldrErr::EmptyRel);
        }

        match (&operation, &self.articles, &self.tags) {
            (
                Operation::Insert(rels),
                Some(Operation::Insert(artcls)),
                Some(Operation::Insert(tgs)),
            )
            | (
                Operation::Create(rels),
                Some(Operation::Create(artcls)),
                Some(Operation::Create(tgs)),
            )
            | (
                Operation::Create(rels),
                Some(Operation::Insert(artcls)),
                Some(Operation::Create(tgs)),
            )
            | (
                Operation::Create(rels),
                Some(Operation::Create(artcls)),
                Some(Operation::Insert(tgs)),
            )
            | (
                Operation::Create(rels),
                Some(Operation::Insert(artcls)),
                Some(Operation::Insert(tgs)),
            ) => {
                let articles_len = artcls.len();
                if !rels
                    .iter()
                    .all(|&(article, _)| article >= 1 && article <= articles_len)
                {
                    return self
                        .apply_error(BldrErr::OutOfRange("article".to_owned(), articles_len));
                }
                let tags_len = tgs.len();
                if !rels.iter().all(|&(_, tag)| tag >= 1 && tag <= tags_len) {
                    return self.apply_error(BldrErr::OutOfRange("tag".to_owned(), tags_len));
                }
            }
            (Operation::Migration, Some(_), Some(_)) => (),
            (_, _, None) => {
                return self.apply_error(BldrErr::WrongOrder(
                    "tags".to_owned(),
                    "article_tags".to_owned(),
                ));
            }
            _ => {
                return self.apply_error(BldrErr::WrongOrder(
                    "articles".to_owned(),
                    "article_tags".to_owned(),
                ));
            }
        }

        let gen_article_tags = |relations: RelArticleTag| {
            relations
                .iter()
                .map(|(article, tag)| {
                    match (self.articles.as_ref().unwrap(), self.tags.as_ref().unwrap()) {
                        (Operation::Insert(artcls), Operation::Insert(tgs))
                        | (Operation::Insert(artcls), Operation::Create(tgs))
                        | (Operation::Create(artcls), Operation::Create(tgs))
                        | (Operation::Create(artcls), Operation::Insert(tgs)) => {
                            article_tag::Model {
                                article_id: artcls[*article as usize - 1].id,
                                tag_id: tgs[*tag as usize - 1].id,
                            }
                        }
                        _ => unreachable!(),
                    }
                })
                .collect()
        };

        let article_tags = match operation {
            Operation::Insert(rels) => Operation::Insert(gen_article_tags(rels)),
            Operation::Create(rels) => Operation::Create(gen_article_tags(rels)),
            Operation::Migration => Operation::Migration,
        };

        self.article_tags = Some(article_tags);
        self
    }

    pub fn followers(mut self, operation: Operation<RelUserFollower>) -> Self {
        if matches!(&operation, Operation::Insert(rels) | Operation::Create(rels) if rels.is_empty())
        {
            return self.apply_error(BldrErr::EmptyRel);
        }

        match (&operation, &self.users) {
            (Operation::Insert(rels), Some(Operation::Insert(mdls)))
            | (Operation::Create(rels), Some(Operation::Insert(mdls)))
            | (Operation::Create(rels), Some(Operation::Create(mdls))) => {
                let users_len = mdls.len();
                if !rels.iter().all(|&(user, _)| user >= 1 && user <= users_len) {
                    return self.apply_error(BldrErr::OutOfRange("user".to_owned(), users_len));
                }
                if !rels
                    .iter()
                    .all(|&(_, follower)| follower >= 1 && follower <= users_len)
                {
                    return self.apply_error(BldrErr::OutOfRange("follower".to_owned(), users_len));
                }
            }
            (Operation::Migration, Some(_)) => (),
            _ => {
                return self.apply_error(BldrErr::WrongOrder(
                    "users".to_owned(),
                    "followers".to_owned(),
                ));
            }
        }

        let gen_followers = |relations: RelUserFollower| {
            relations
                .iter()
                .map(|(user, follower)| match self.users.as_ref().unwrap() {
                    Operation::Insert(users) | Operation::Create(users) => follower::Model {
                        user_id: users[*user as usize - 1].id,
                        follower_id: users[*follower as usize - 1].id,
                    },
                    _ => unreachable!(),
                })
                .collect()
        };

        let followers = match operation {
            Operation::Insert(rels) => Operation::Insert(gen_followers(rels)),
            Operation::Create(rels) => Operation::Create(gen_followers(rels)),
            Operation::Migration => Operation::Migration,
        };

        self.followers = Some(followers);
        self
    }

    pub fn favorited_articles(mut self, operation: Operation<RelArticleUser>) -> Self {
        if matches!(&operation, Operation::Insert(rels) | Operation::Create(rels) if rels.is_empty())
        {
            return self.apply_error(BldrErr::EmptyRel);
        }

        match (&operation, &self.articles, &self.users) {
            (
                Operation::Insert(rels),
                Some(Operation::Insert(artcls)),
                Some(Operation::Insert(usrs)),
            )
            | (
                Operation::Create(rels),
                Some(Operation::Create(artcls)),
                Some(Operation::Create(usrs)),
            )
            | (
                Operation::Create(rels),
                Some(Operation::Insert(artcls)),
                Some(Operation::Create(usrs)),
            )
            | (
                Operation::Create(rels),
                Some(Operation::Create(artcls)),
                Some(Operation::Insert(usrs)),
            )
            | (
                Operation::Create(rels),
                Some(Operation::Insert(artcls)),
                Some(Operation::Insert(usrs)),
            ) => {
                let articles_len = artcls.len();
                if !rels
                    .iter()
                    .all(|&(article, _)| article >= 1 && article <= articles_len)
                {
                    return self
                        .apply_error(BldrErr::OutOfRange("article".to_owned(), articles_len));
                }
                let users_len = usrs.len();
                if !rels.iter().all(|&(_, user)| user >= 1 && user <= users_len) {
                    return self.apply_error(BldrErr::OutOfRange("user".to_owned(), users_len));
                }
            }
            (Operation::Migration, Some(_), Some(_)) => (),
            _ => {
                return self.apply_error(BldrErr::WrongOrder(
                    "articles".to_owned(),
                    "favorited_articles".to_owned(),
                ));
            }
        }

        let gen_favorited_articles = |relations: RelArticleUser| {
            relations
                .iter()
                .map(|(article, user)| {
                    match (
                        self.articles.as_ref().unwrap(),
                        self.users.as_ref().unwrap(),
                    ) {
                        (Operation::Insert(artcls), Operation::Insert(usrs))
                        | (Operation::Insert(artcls), Operation::Create(usrs))
                        | (Operation::Create(artcls), Operation::Create(usrs))
                        | (Operation::Create(artcls), Operation::Insert(usrs)) => {
                            favorited_article::Model {
                                article_id: artcls[*article as usize - 1].id,
                                user_id: usrs[*user as usize - 1].id,
                            }
                        }
                        _ => unreachable!(),
                    }
                })
                .collect()
        };

        let favorited_articles = match operation {
            Operation::Insert(rels) => Operation::Insert(gen_favorited_articles(rels)),
            Operation::Create(rels) => Operation::Create(gen_favorited_articles(rels)),
            Operation::Migration => Operation::Migration,
        };

        self.favorited_articles = Some(favorited_articles);
        self
    }

    async fn exec<E: EntityTrait, AM: ActiveModelTrait<Entity = E> + From<E::Model>>(
        &self,
        db: &DatabaseConnection,
        migrations: Vec<&str>,
        operations: &Option<Operation<Vec<E::Model>>>,
    ) -> Result<Option<Vec<E::Model>>, DbErr> {
        if operations.is_none() {
            return Ok(None);
        }
        for migration in migrations {
            execute_migration(db, migration).await?;
        }

        match operations.as_ref().unwrap() {
            Operation::Insert(models) => {
                let actives = Self::activate_models::<E, AM>(&Some(models.to_vec()));
                E::insert_many(actives).exec(db).await?;
                Ok(Some(models.to_vec()))
            }
            Operation::Create(models) => Ok(Some(models.to_vec())),
            Operation::Migration => Ok(None),
        }
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

    pub async fn build(self) -> Result<(DatabaseConnection, TestData), BldrErr> {
        if let Some(err) = self.error {
            return Err(err);
        }

        let connection = init_test_db_connection().await?;

        let users = self
            .exec::<User, user::ActiveModel>(
                &connection,
                vec![
                    "m20231030_000001_create_user_table",
                    "m20231112_000008_add_user_password",
                ],
                &self.users,
            )
            .await?;

        let articles = self
            .exec::<Article, article::ActiveModel>(
                &connection,
                vec!["m20231030_000002_create_article_table"],
                &self.articles,
            )
            .await?;

        let comments = self
            .exec::<Comment, comment::ActiveModel>(
                &connection,
                vec!["m20231030_000003_create_comment_table"],
                &self.comments,
            )
            .await?;

        let tags = self
            .exec::<Tag, tag::ActiveModel>(
                &connection,
                vec!["m20231030_000004_create_tag_table"],
                &self.tags,
            )
            .await?;

        let article_tags = self
            .exec::<ArticleTag, article_tag::ActiveModel>(
                &connection,
                vec!["m20231030_000005_create_article_tag_table"],
                &self.article_tags,
            )
            .await?;

        let followers = self
            .exec::<Follower, follower::ActiveModel>(
                &connection,
                vec!["m20231101_000006_create_follower_table"],
                &self.followers,
            )
            .await?;

        let favorited_articles = self
            .exec::<FavoritedArticle, favorited_article::ActiveModel>(
                &connection,
                vec!["m20231104_000007_create_favorited_article_table"],
                &self.favorited_articles,
            )
            .await?;

        Ok((
            connection,
            TestData {
                users,
                articles,
                comments,
                tags,
                article_tags,
                followers,
                favorited_articles,
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
    use crate::tests::Operation::Insert;
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
        let tested = TestDataBuilder::new().users(Insert(2));
        if let Some(Insert(models)) = tested.users {
            assert_eq!(models.len(), 2);
        } else {
            panic!("{:?}", "users not set in builder");
        }
    }

    #[test]
    fn test_users_zero_qty() {
        let expected = TestDataBuilder {
            error: Some(BldrErr::ZeroQty),
            ..Default::default()
        };
        assert_eq!(TestDataBuilder::new().users(Insert(0)), expected);
    }

    // TEST ARTICLES
    #[test]
    fn test_articles() {
        let tested = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 2, 2]));
        if let Some(Insert(models)) = tested.articles {
            assert_eq!(models.len(), 3);
        } else {
            panic!("{:?}", "articles not set in builder");
        }
    }

    #[test]
    fn test_articles_users_not_set() {
        let expected = Some(BldrErr::WrongOrder(
            "users".to_owned(),
            "articles".to_owned(),
        ));
        let tested = TestDataBuilder::new().articles(Insert(vec![1, 2, 2]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_articles_empty_input() {
        let expected = Some(BldrErr::EmptyRel);
        let tested = TestDataBuilder::new().articles(Insert(vec![]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_articles_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("user".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![0]));
        assert_eq!(tested1.error, expected);
        let tested2 = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![3]));
        assert_eq!(tested2.error, expected);
    }

    // TEST COMMENTS
    #[test]
    fn test_comments() {
        let tested = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 2, 2]))
            .comments(Insert(vec![(1, 1), (2, 3), (2, 2)]));
        if let Some(Insert(models)) = tested.comments {
            assert_eq!(models.len(), 3);
        } else {
            panic!("{:?}", "comments not set in builder");
        }
    }

    #[test]
    fn test_comments_articles_not_set() {
        let expected = Some(BldrErr::WrongOrder(
            "articles".to_owned(),
            "comments".to_owned(),
        ));
        let tested = TestDataBuilder::new()
            .users(Insert(2))
            .comments(Insert(vec![(1, 2), (1, 2)]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_comments_empty_input() {
        let expected = Some(BldrErr::EmptyRel);
        let tested = TestDataBuilder::new().comments(Insert(vec![]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_author_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("author".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 2]))
            .comments(Insert(vec![(0, 2)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 2]))
            .comments(Insert(vec![(3, 2)]));
        assert_eq!(tested2.error, expected);
    }

    #[test]
    fn test_article_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("article".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 2]))
            .comments(Insert(vec![(1, 0)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 2]))
            .comments(Insert(vec![(1, 3)]));
        assert_eq!(tested2.error, expected);
    }

    // TEST TAGS
    #[test]
    fn test_tags() {
        let tested = TestDataBuilder::new().tags(Insert(2));
        if let Some(Insert(models)) = tested.tags {
            assert_eq!(models.len(), 2);
        } else {
            panic!("{:?}", "tags not set in builder");
        }
    }

    #[test]
    fn test_tags_zero_qty() {
        let expected = TestDataBuilder {
            error: Some(BldrErr::ZeroQty),
            ..Default::default()
        };
        assert_eq!(TestDataBuilder::new().tags(Insert(0)), expected);
    }

    // TEST ARTICLE_TAGS
    #[test]
    fn test_article_tags() {
        let tested = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 2, 2]))
            .tags(Insert(2))
            .article_tags(Insert(vec![(1, 1), (2, 2), (3, 2)]));
        if let Some(Insert(models)) = tested.article_tags {
            assert_eq!(models.len(), 3);
        } else {
            panic!("{:?}", "article_tags not set in builder");
        }
    }

    #[test]
    fn test_article_tags_articles_not_set() {
        let expected = Some(BldrErr::WrongOrder(
            "articles".to_owned(),
            "article_tags".to_owned(),
        ));
        let tested = TestDataBuilder::new()
            .tags(Insert(2))
            .article_tags(Insert(vec![(1, 2), (1, 2)]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_article_tags_tags_not_set() {
        let expected = Some(BldrErr::WrongOrder(
            "tags".to_owned(),
            "article_tags".to_owned(),
        ));
        let tested = TestDataBuilder::new()
            .users(Insert(3))
            .articles(Insert(vec![1, 2, 2]))
            .article_tags(Insert(vec![(1, 2), (1, 2)]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_article_tags_empty_input() {
        let expected = Some(BldrErr::EmptyRel);
        let tested = TestDataBuilder::new().comments(Insert(vec![]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_article_tags_article_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("article".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(Insert(2))
            .tags(Insert(2))
            .articles(Insert(vec![1, 2]))
            .article_tags(Insert(vec![(0, 1)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(Insert(2))
            .tags(Insert(2))
            .articles(Insert(vec![1, 2]))
            .article_tags(Insert(vec![(3, 1)]));
        assert_eq!(tested2.error, expected);
    }

    #[test]
    fn test_article_tags_tag_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("tag".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(Insert(2))
            .tags(Insert(2))
            .articles(Insert(vec![1, 2]))
            .article_tags(Insert(vec![(1, 0)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(Insert(2))
            .tags(Insert(2))
            .articles(Insert(vec![1, 2]))
            .article_tags(Insert(vec![(2, 3)]));
        assert_eq!(tested2.error, expected);
    }

    // TEST FOLLOWER
    #[test]
    fn test_followers() {
        let tested = TestDataBuilder::new()
            .users(Insert(2))
            .followers(Insert(vec![(1, 2), (2, 1)]));
        if let Some(Insert(models)) = tested.followers {
            assert_eq!(models.len(), 2);
        } else {
            panic!("{:?}", "followers not set in builder");
        }
    }

    #[test]
    fn test_followers_users_not_set() {
        let expected = Some(BldrErr::WrongOrder(
            "users".to_owned(),
            "followers".to_owned(),
        ));
        let tested = TestDataBuilder::new().followers(Insert(vec![(1, 2), (2, 1)]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_followers_empty_input() {
        let expected = Some(BldrErr::EmptyRel);
        let tested = TestDataBuilder::new().followers(Insert(vec![]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_follower_user_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("user".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(Insert(2))
            .followers(Insert(vec![(0, 2)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(Insert(2))
            .followers(Insert(vec![(3, 2)]));
        assert_eq!(tested2.error, expected);
    }

    #[test]
    fn test_follower_follower_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("follower".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(Insert(2))
            .followers(Insert(vec![(1, 0)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(Insert(2))
            .followers(Insert(vec![(1, 3)]));
        assert_eq!(tested2.error, expected);
    }

    // TEST FAVORITED_ARTICLES
    #[test]
    fn test_favorited_articles() {
        let tested = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 2, 2]))
            .favorited_articles(Insert(vec![(1, 1), (2, 2), (3, 2)]));
        if let Some(Insert(models)) = tested.favorited_articles {
            assert_eq!(models.len(), 3);
        } else {
            panic!("{:?}", "favorited_articles not set in builder");
        }
    }

    #[test]
    fn test_favorited_articles_articles_not_set() {
        let expected = Some(BldrErr::WrongOrder(
            "articles".to_owned(),
            "favorited_articles".to_owned(),
        ));
        let tested =
            TestDataBuilder::new().favorited_articles(Insert(vec![(1, 1), (2, 2), (3, 2)]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_favorited_articles_empty_input() {
        let expected = Some(BldrErr::EmptyRel);
        let tested = TestDataBuilder::new().favorited_articles(Insert(vec![]));
        assert_eq!(tested.error, expected);
    }

    #[test]
    fn test_favorited_articles_article_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("article".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 2]))
            .favorited_articles(Insert(vec![(0, 1)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(Insert(2))
            .tags(Insert(2))
            .articles(Insert(vec![1, 2]))
            .favorited_articles(Insert(vec![(3, 1)]));
        assert_eq!(tested2.error, expected);
    }

    #[test]
    fn test_favorited_articles_users_not_in_range() {
        let expected = Some(BldrErr::OutOfRange("user".to_owned(), 2));
        let tested1 = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 2]))
            .favorited_articles(Insert(vec![(2, 0)]));
        assert_eq!(tested1.error, expected);

        let tested2 = TestDataBuilder::new()
            .users(Insert(2))
            .articles(Insert(vec![1, 2]))
            .favorited_articles(Insert(vec![(2, 3)]));
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
            .exec::<User, user::ActiveModel>(
                &connection,
                vec![
                    "m20231030_000001_create_user_table",
                    "m20231112_000008_add_user_password",
                ],
                &Some(Insert(expected.clone())),
            )
            .await?;

        let tested = User::find().all(&connection).await?;
        assert_eq!(expected, tested);

        Ok(())
    }

    #[tokio::test]
    async fn test_build() -> Result<(), BldrErr> {
        let tested = TestDataBuilder::new().users(Insert(2)).build().await?;
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
