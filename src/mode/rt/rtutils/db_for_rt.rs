use std::sync::Arc;
use axum::http::StatusCode;
use sea_orm::DatabaseConnection;
use crate::utils::db::DbPools;
use crate::mode::rt::rtres::common_res::ApiError;

pub trait DbPoolsExt {
    fn get_rw_for_rt(&self) -> Result<&DatabaseConnection, ApiError>;
    fn get_ro_for_rt(&self) -> Result<&DatabaseConnection, ApiError>;
}

impl DbPoolsExt for DbPools {
    fn get_rw_for_rt(&self) -> Result<&DatabaseConnection, ApiError> {
        self.get_rw().map_err(|e| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get RW connection: {}", e),
            )
        })
    }

    fn get_ro_for_rt(&self) -> Result<&DatabaseConnection, ApiError> {
        self.get_ro().map_err(|e| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get RO connection: {}", e),
            )
        })
    }
}

impl DbPoolsExt for Arc<DbPools> {
    fn get_rw_for_rt(&self) -> Result<&DatabaseConnection, ApiError> {
        self.as_ref().get_rw_for_rt()
    }

    fn get_ro_for_rt(&self) -> Result<&DatabaseConnection, ApiError> {
        self.as_ref().get_ro_for_rt()
    }
}
