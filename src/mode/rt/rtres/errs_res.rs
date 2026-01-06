use utoipa::ToSchema;
use axum::{Json, http::StatusCode, response::{Response, IntoResponse}};
use serde::Serialize;

#[derive(Serialize, ToSchema)]
pub struct ErrorDetail {
    /// エラー箇所 (例: "email" / "system")
    #[schema(example = "email")]
    pub field: String,
    
    /// エラーコード (例: "E0001")
    #[schema(example = "E0001")]
    pub code: String,
    
    /// 人間が読めるメッセージ
    #[schema(example = "Invalid email address.")]
    pub message: String,
}

#[derive(Serialize, ToSchema)] // OpenAPIドキュメント生成とシリアライズ用
pub struct ApiError {
    /// HTTPステータスコード (例: 500)
    #[schema(example = 500)] 
    pub status: u16,
    
    /// 構造化されたエラー詳細のリスト
    pub errors: Vec<ErrorDetail>,
}

impl ApiError {
    /// 単一のメッセージを持つApiErrorを生成 (field="system")
    pub fn new_system(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: status.as_u16(),
            errors: vec![ErrorDetail {
                field: "system".to_string(),
                code: code.into(),
                message: message.into(),
            }],
        }
    }

    /// 複数の構造化エラーを持つApiErrorを生成
    pub fn new_many(status: StatusCode, errors: Vec<ErrorDetail>) -> Self {
        Self {
            status: status.as_u16(),
            errors,
        }
    }

    /// garde の Report から構造化されたエラー詳細を持つ ApiError を生成
    pub fn from_garde(report: garde::Report) -> Self {
        let mut errors = Vec::new();
        for (path, error) in report.iter() {
            let full_msg = error.to_string();
            // 「|」で分割を試みる
            let (code, message) = if let Some((c, m)) = full_msg.split_once('|') {
                (c.trim().to_string(), m.trim().to_string())
            } else {
                // 「|」が含まれない場合は、デフォルトのバリデーションエラーコードを使用
                (crate::mode::rt::rterr::rterr::ERR_VALIDATION.to_string(), full_msg)
            };
            errors.push(ErrorDetail {
                field: path.to_string(),
                code,
                message,
            });
        }
        Self::new_many(StatusCode::UNPROCESSABLE_ENTITY, errors)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}
