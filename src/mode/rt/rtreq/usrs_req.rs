use utoipa::IntoParams;
use serde::Deserialize;

#[derive(Deserialize, IntoParams)]
pub struct AuthUsrReq {
    pub email: String,
    pub password: String,
    pub expire: u32,
}
