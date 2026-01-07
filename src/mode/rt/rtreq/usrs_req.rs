use serde::Deserialize;
use garde::Validate;
use utoipa::{IntoParams, ToSchema};
use crate::mode::rt::rterr::rterr::*;

// ============================================================
// Auth
// ============================================================
#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct AuthUsrReq {
    #[garde(custom(required_simple_err(1, 100)))]
    pub email: String,

    #[garde(custom(required_simple_err(1, 100)))]
    pub password: String,

    #[serde(default = "default_expire")]
    #[schema(default = 24)]
    #[garde(skip)]
    pub expire: Option<u32>,
}

fn default_expire() -> Option<u32> { Some(24) }

// ============================================================
// Search
// ============================================================
#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct SearchUsrsReq {
    #[garde(custom(length_simple_err(0, 50)))]
    pub name: String,

    #[garde(custom(email_err))]
    #[garde(custom(length_simple_err(0, 50)))]
    pub email: String,

    #[garde(custom(required_simple_err(1, 100)))]
    #[garde(custom(datetime_err))]
    pub bgn_at: String,

    #[garde(custom(required_simple_err(1, 100)))]
    #[garde(custom(datetime_err))]
    pub end_at: String,

    #[garde(custom(range_err(Some(1u16), Some(25u16))))]
    pub limit: u16,

    #[garde(custom(range_err(Some(0u16), None)))]
    pub offset: u16,
}

// ============================================================
// Create
// ============================================================
#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateUsrReq {
    #[garde(custom(required_simple_err(1, 50)))]
    #[garde(custom(length_simple_err(0, 50)))]
    pub name: String,

    #[garde(custom(required_simple_err(1, 50)))]
    #[garde(custom(email_err))]
    #[garde(custom(length_simple_err(0, 50)))]
    pub email: String,

    #[garde(custom(required_simple_err(1, 100)))]
    pub password: String,

    #[garde(custom(required_simple_err(1, 100)))]
    #[garde(custom(datetime_err))]
    pub bgn_at: String,

    #[garde(custom(required_simple_err(1, 100)))]
    #[garde(custom(datetime_err))]
    pub end_at: String,

    #[serde(rename = "type")]
    #[garde(inner(custom(range_err(Some(1u8), Some(2u8)))))]
    pub usr_type: Option<u8>,

    #[garde(custom(range_err(Some(0u32), None)))]
    pub base_point: u32,

    #[garde(custom(range_err(Some(0.0f64), None)))]
    pub belong_rate: f64,

    #[garde(custom(range_err(Some(0u32), None)))]
    pub max_works: u32,

    #[garde(custom(range_err(Some(0u32), None)))]
    pub flush_days: u32,

    #[garde(custom(range_err(Some(0.0f64), None)))]
    pub rate: f64,

    #[garde(custom(range_err(Some(0.0f64), None)))]
    pub flush_fee_rate: f64,
}

// ============================================================
// Update
// ============================================================
#[derive(Deserialize, Validate, ToSchema)]
pub struct UpdateUsrReq {
    #[garde(inner(custom(length_simple_err(0, 50))))]
    pub name: Option<String>,

    #[garde(inner(custom(email_err)))]
    #[garde(inner(custom(length_simple_err(0, 50))))]
    pub email: Option<String>,

    #[garde(skip)]
    pub password: Option<String>,

    #[garde(inner(custom(datetime_err)))]
    pub bgn_at: Option<String>,

    #[garde(inner(custom(datetime_err)))]
    pub end_at: Option<String>,

    #[serde(rename = "type")]
    #[garde(inner(custom(range_err(Some(1u8), Some(2u8)))))]
    pub usr_type: Option<u8>,

    #[garde(inner(custom(range_err(Some(0u32), None))))]
    pub base_point: Option<u32>,

    #[garde(inner(custom(range_err(Some(0.0f64), None))))]
    pub belong_rate: Option<f64>,

    #[garde(inner(custom(range_err(Some(0u32), None))))]
    pub max_works: Option<u32>,

    #[garde(inner(custom(range_err(Some(0u32), None))))]
    pub flush_days: Option<u32>,

    #[garde(inner(custom(range_err(Some(0.0f64), None))))]
    pub rate: Option<f64>,

    #[garde(inner(custom(range_err(Some(0.0f64), None))))]
    pub flush_fee_rate: Option<f64>,
}
