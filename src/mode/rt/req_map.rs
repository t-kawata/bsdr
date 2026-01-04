use axum::{Router, Extension};
use crate::{config::VERSION, utils::cors::cors_layer, utils::db::DbPools};
use std::sync::Arc;
use utoipa::{OpenApi};
use utoipa_swagger_ui::SwaggerUi;
use utoipa_axum::{router::OpenApiRouter, routes};
use crate::mode::rt::rthandler::usrs_handler::*;
use utoipa::Modify;
use utoipa::openapi::security::{SecurityScheme, HttpBuilder, HttpAuthScheme};

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
    .routes(routes!(search_usrs))
    .routes(routes!(get_usr))
    .routes(routes!(create_usr))
    .routes(routes!(update_usr))
    .routes(routes!(delete_usr))
}

// ==============================
// リクエストマッピング
// ==============================
pub fn map_request(cors: bool, db: DbPools) -> Router {
    log::debug!("Mapping requests.");
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/v1", app_routes())
        .split_for_parts();
    let mut app = Router::new()
        .merge(router)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        .layer(Extension(Arc::new(db)));
    if cors {
        app = app.layer(cors_layer());
    }
    app
}
