use utoipa::ToSchema;
use axum::{Json, http::StatusCode, response::{Response, IntoResponse}};
use serde::Serialize;

#[derive(Serialize, ToSchema)] // OpenAPIドキュメント生成とシリアライズ用
pub struct ApiError {
    /// HTTPステータスコード (例: 500)
    #[schema(example = 500)] 
    pub status: u16,
    
    /// エラー内容に関するメッセージ
    #[schema(example = "Error message.")]
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status: status.as_u16(),
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}
