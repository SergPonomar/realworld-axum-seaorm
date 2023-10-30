use axum::{routing::get, Router};
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

const DEFAULT_APP_PORT: u16 = 3000;
const DEFAULT_APP_HOST: &str = "127.0.0.1";
const APP_PORT: &str = "APP_PORT";
const APP_HOST: &str = "APP_HOST";

pub async fn start() {
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root));

    let addr = get_socket_address();
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
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
