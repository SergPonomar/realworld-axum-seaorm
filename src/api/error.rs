use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use sea_orm::DbErr;
use serde_json::json;

/// error returned by Api
#[derive(Debug, PartialEq)]
pub enum ApiErr {
    DbErr(DbErr),
}

impl From<DbErr> for ApiErr {
    fn from(err: DbErr) -> ApiErr {
        ApiErr::DbErr(err)
    }
}

// TODO test this
impl IntoResponse for ApiErr {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiErr::DbErr(DbErr::Exec(_)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Record with same parameters already exist",
            ),
            ApiErr::DbErr(DbErr::RecordNotUpdated) => (StatusCode::NOT_FOUND, "Record not exist"),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "The server cannot process the request",
            ),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
