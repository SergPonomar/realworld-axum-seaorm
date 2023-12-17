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
    println!("Server listening on {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

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
