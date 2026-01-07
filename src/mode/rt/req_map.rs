use axum::{Router, Extension};
use crate::utils::jwt::JwtConfig;
use crate::{config::VERSION, utils::cors::cors_layer, utils::db::DbPools};
use std::sync::Arc;
use utoipa::{OpenApi};
use utoipa_swagger_ui::SwaggerUi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa::Modify;
use utoipa::openapi::security::{SecurityScheme, HttpBuilder, HttpAuthScheme};
use crate::mode::rt::rthandler::usrs_handler::*;
use crate::mode::rt::rthandler::bds_handler::*;
use crate::mode::rt::rthandler::cryptos_handler::*;

// ==============================
// セキュリティアドオン作成
// ==============================
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // componentsがNoneの場合に備えて取り出す
        let components = openapi.components.as_mut().unwrap(); 
        components.add_security_scheme(
            "api_jwt_token", // この名前を後で参照
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT") // 任意でフォーマットを指定
                    .build(),
            ),
        );
    }
}

// ==============================
// Swagger 共通定義
// ==============================
#[derive(OpenApi)]
#[openapi(modifiers(&SecurityAddon), info(
    title = "BSDR",
    version = VERSION,
    description = "## API概要\nBSDR REST APIを定義する。\nURL最大長のリスクを避ける為、検索は query parameter ではなく body json を使用する。\n検索は POST にて行う。",
))]
struct ApiDoc;

// ==============================
// Route & Handler を設定
// ==============================
fn app_routes() -> OpenApiRouter { OpenApiRouter::new()
    .routes(routes!(create_bd_hash))
    .routes(routes!(check_bd_hash))
    .routes(routes!(auth_usr))
    .routes(routes!(search_usrs))
    .routes(routes!(get_usr))
    .routes(routes!(create_usr))
    .routes(routes!(update_usr))
    .routes(routes!(delete_usr))
    .routes(routes!(hire_usr))
    .routes(routes!(dehire_usr))
    .routes(routes!(encrypt_handler))
    .routes(routes!(decrypt_handler))
    .routes(routes!(create_vdr_token_handler))
    .routes(routes!(get_vdr_token_handler))
}

// ==============================
// リクエストマッピング
// ==============================
pub fn map_request(cors: bool, db: DbPools, rt_skey: &str, rt_crypto_key: &str) -> Router {
    log::debug!("Mapping requests.");
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/v1", app_routes())
        .split_for_parts();
    let mut app = Router::new()
        .merge(router)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        .layer(Extension(Arc::new(db)))
        .layer(Extension(Arc::new(JwtConfig {
            skey: rt_skey.to_string(),
            crypto_key: rt_crypto_key.to_string(),
        })));
    if cors {
        app = app.layer(cors_layer());
    }
    app
}
