use serde::Deserialize;
use garde::Validate;
use utoipa::{IntoParams, ToSchema};
use crate::mode::rt::rterr::rterr::*;

// ============================================================
// Search
// ============================================================
#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct SearchUsrsReq {
    /// name
    #[garde(custom(length_simple_err(0, 50)))]
    pub name: String,

    /// email
    #[garde(custom(email_err))]
    #[garde(custom(length_simple_err(0, 50)))]
    pub email: String,

    /// bgn_at
    #[garde(custom(required_simple_err(1, 100)))]
    #[garde(custom(datetime_err))]
    pub bgn_at: String,

    /// end_at
    #[garde(custom(required_simple_err(1, 100)))]
    #[garde(custom(datetime_err))]
    pub end_at: String,

    /// limit
    #[garde(custom(range_err(Some(1u16), Some(25u16))))]
    pub limit: u16,

    /// offset
    #[garde(custom(range_err(Some(0u16), None)))]
    pub offset: u16,
}

// ============================================================
// Create
// ============================================================
#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateUsrReq {
    /// name
    #[garde(custom(required_simple_err(1, 50)))]
    #[garde(custom(length_simple_err(0, 50)))]
    pub name: String,

    /// email
    #[garde(custom(required_simple_err(1, 50)))]
    #[garde(custom(email_err))]
    #[garde(custom(length_simple_err(0, 50)))]
    pub email: String,

    /// password
    #[garde(custom(required_simple_err(1, 100)))]
    pub password: String,

    /// bgn_at
    #[garde(custom(required_simple_err(1, 100)))]
    #[garde(custom(datetime_err))]
    pub bgn_at: String,

    /// end_at
    #[garde(custom(required_simple_err(1, 100)))]
    #[garde(custom(datetime_err))]
    pub end_at: String,

    /// type
    #[serde(rename = "type")]
    #[garde(inner(custom(range_err(Some(1u8), Some(2u8)))))]
    pub usr_type: Option<u8>,

    /// base_point
    #[garde(custom(range_err(Some(0u32), None)))]
    pub base_point: u32,

    /// belong_rate
    #[garde(custom(range_err(Some(0.0f64), None)))]
    pub belong_rate: f64,

    /// max_works
    #[garde(custom(range_err(Some(0u32), None)))]
    pub max_works: u32,

    /// flush_days
    #[garde(custom(range_err(Some(0u32), None)))]
    pub flush_days: u32,

    /// rate
    #[garde(custom(range_err(Some(0.0f64), None)))]
    pub rate: f64,

    /// flush_fee_rate
    #[garde(custom(range_err(Some(0.0f64), None)))]
    pub flush_fee_rate: f64,
}

// ============================================================
// Update
// ============================================================
#[derive(Deserialize, Validate, ToSchema)]
pub struct UpdateUsrReq {
    /// name
    #[garde(inner(custom(length_simple_err(0, 50))))]
    pub name: Option<String>,

    /// email
    #[garde(inner(custom(email_err)))]
    #[garde(inner(custom(length_simple_err(0, 50))))]
    pub email: Option<String>,

    /// password
    #[garde(skip)]
    pub password: Option<String>,

    /// bgn_at
    #[garde(inner(custom(datetime_err)))]
    pub bgn_at: Option<String>,

    /// end_at
    #[garde(inner(custom(datetime_err)))]
    pub end_at: Option<String>,

    /// type
    #[serde(rename = "type")]
    #[garde(inner(custom(range_err(Some(1u8), Some(2u8)))))]
    pub usr_type: Option<u8>,

    /// base_point
    #[garde(inner(custom(range_err(Some(0u32), None))))]
    pub base_point: Option<u32>,

    /// belong_rate
    #[garde(inner(custom(range_err(Some(0.0f64), None))))]
    pub belong_rate: Option<f64>,

    /// max_works
    #[garde(inner(custom(range_err(Some(0u32), None))))]
    pub max_works: Option<u32>,

    /// flush_days
    #[garde(inner(custom(range_err(Some(0u32), None))))]
    pub flush_days: Option<u32>,

    /// rate
    #[garde(inner(custom(range_err(Some(0.0f64), None))))]
    pub rate: Option<f64>,

    /// flush_fee_rate
    #[garde(inner(custom(range_err(Some(0.0f64), None))))]
    pub flush_fee_rate: Option<f64>,
}

// ============================================================
// Auth
// ============================================================
#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct AuthUsrReq {
    /// email
    #[garde(custom(required_simple_err(1, 100)))]
    pub email: String,

    /// password
    #[garde(custom(required_simple_err(1, 100)))]
    pub password: String,

    /// expire
    #[serde(default = "default_expire")]
    #[schema(default = 24)]
    #[garde(skip)]
    pub expire: Option<u32>,
}

fn default_expire() -> Option<u32> { Some(24) }
