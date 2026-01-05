use utoipa::IntoParams;
use serde::Deserialize;

#[derive(Deserialize, IntoParams)]
pub struct CreateBdHashReq {
    pub bd: String,
}

#[derive(Deserialize, IntoParams)]
pub struct CheckBdHashReq {
    pub bd: String,
}
