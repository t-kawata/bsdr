use utoipa::{IntoParams, ToSchema};
use serde::Deserialize;
use garde::Validate;
use crate::mode::rt::rterr::rterr::*;

#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct CreateBdHashReq {
    /// BD文字列
    #[garde(custom(required_simple_err(1, 10000)))]
    pub bd: String,
}

#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct CheckBdHashReq {
    /// BD文字列
    #[garde(custom(required_simple_err(1, 10000)))]
    pub bd: String,
}
