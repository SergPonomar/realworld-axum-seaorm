use axum::extract::rejection::TypedHeaderRejection;
use axum::TypedHeader;
use axum::{
    headers::authorization::{Authorization, Credentials},
    http::{HeaderValue, Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use bytes::Bytes;
use chrono::Duration;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use thiserror::Error;
const SECRET_KEY: &str = "SECRET_KEY";

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Token {
    exp: usize,
    pub username: String,
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
    // WithRejection(TypedHeader(Authorization(token)), _): WithRejection<
    //     TypedHeader<Authorization<Token>>,
    //     ApiError,
    // >,
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
    // // match extract_token

    //     Err(StatusCode::UNAUTHORIZED)
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

pub fn create_token(username: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now();
    let expires_at = now + Duration::seconds(100);
    let exp = expires_at.timestamp() as usize;
    let claims = Token {
        exp,
        username: username.to_string(),
    };
    let token_header = Header::default();

    let secret = get_secret_key();
    let key = EncodingKey::from_secret(secret.as_bytes());

    encode(&token_header, &claims, &key)
}

/// Get secret key
fn get_secret_key() -> String {
    env::var(SECRET_KEY).expect("env variable SECRET_KEY should be set for JWT generation")
}

// We derive `thiserror::Error`
#[derive(Debug, Error)]
pub enum ApiError {
    // The `#[from]` attribute generates `From<JsonRejection> for ApiError`
    // implementation. See `thiserror` docs for more information
    #[error(transparent)]
    TypedHeaderExtractorRejection(#[from] TypedHeaderRejection),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        // let (status, message) = match self {
        //     ApiError::TypedHeaderExtractorRejection(typed_header_rejection) => (
        //         typed_header_rejection.status(),
        //         typed_header_rejection.body_text(),
        //     ),
        // };

        let payload = json!({
            "errors":{
                "body": [
                    "can't be empty"
                ]
            }
        });

        (axum::http::StatusCode::BAD_REQUEST, Json(payload)).into_response()
    }
}
