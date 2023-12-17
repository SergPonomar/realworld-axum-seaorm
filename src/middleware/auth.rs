use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use axum::extract::rejection::TypedHeaderRejection;
use axum::TypedHeader;
use axum::{
    headers::authorization::{Authorization, Credentials},
    http::{HeaderValue, Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use chrono::Duration;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand_core::OsRng;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use std::env;

const SECRET_KEY: &str = "SECRET_KEY";

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Token {
    pub exp: usize,
    pub id: Uuid,
}

impl Credentials for Token {
    const SCHEME: &'static str = "Token";

    fn decode(value: &HeaderValue) -> Option<Self> {
        debug_assert!(
            value.as_bytes().starts_with(b"Token "),
            "HeaderValue to decode should start with \"Token ..\", received = {:?}",
            value,
        );

        let tkn_str = value.to_str().unwrap().replace("Token ", "");
        decode(
            &tkn_str,
            &DecodingKey::from_secret(get_secret_key().as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .ok()
        .map(|data| data.claims)
    }

    fn encode(&self) -> HeaderValue {
        let token_header = Header::default();
        let secret = get_secret_key();
        let key = EncodingKey::from_secret(secret.as_bytes());

        let tkn = encode(&token_header, &self, &key).unwrap();
        let bytes = Bytes::from(format!("Token {tkn}"));
        HeaderValue::from_maybe_shared(bytes)
            .expect("base64 encoding is always a valid HeaderValue")
    }
}

pub async fn auth<B: std::fmt::Debug>(
    maybe_token: Result<TypedHeader<Authorization<Token>>, TypedHeaderRejection>,
    mut request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    match maybe_token {
        Ok(TypedHeader(Authorization(token))) => {
            request.extensions_mut().insert(token);
            let response = next.run(request).await;
            Ok(response)
        }
        Err(err) => {
            let response = match request.method() {
                &Method::GET => next.run(request).await,
                _ => err.into_response(),
            };
            Ok(response)
        }
    }
}

pub async fn optional_auth<B: std::fmt::Debug>(
    maybe_token: Option<TypedHeader<Authorization<Token>>>,
    mut request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    if let Some(TypedHeader(Authorization(token))) = maybe_token {
        request.extensions_mut().insert(token);
    }
    let response = next.run(request).await;
    Ok(response)
}

pub fn create_token(id: &Uuid) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Local::now();
    let expires_at = now + Duration::seconds(100);
    let exp = expires_at.timestamp() as usize;
    let claims = Token { exp, id: *id };
    let token_header = Header::default();

    let secret = get_secret_key();
    let key = EncodingKey::from_secret(secret.as_bytes());

    encode(&token_header, &claims, &key)
}

pub fn hash_password(pass: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pass.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

pub fn check_passwords(tested: &str, real: &str) -> Result<(), argon2::password_hash::Error> {
    PasswordHash::new(real)
        .map(|parsed_hash| Argon2::default().verify_password(tested.as_bytes(), &parsed_hash))?
}

/// Get secret key from .env file
fn get_secret_key() -> String {
    env::var(SECRET_KEY).expect("env variable SECRET_KEY should be set for JWT generation")
}
