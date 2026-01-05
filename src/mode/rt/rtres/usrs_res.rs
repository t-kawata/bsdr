use utoipa::ToSchema;
use serde::Serialize;

#[derive(Serialize, ToSchema)]
pub struct AuthUsrRes {
    pub token: String,
}
