use std::sync::Arc;
use axum::{Extension, Json, extract::{Path, Query}, http::{HeaderMap, StatusCode}, response::IntoResponse};
use crate::{
    mode::rt::{rtreq::usrs_req::AuthUsrReq, rtres::{common_res::ApiError, usrs_res::AuthUsrRes}, rtutils::db_for_rt::DbPoolsExt},
    utils::{db::DbPools, jwt::{self, JwtConfig}}
};

const TAG: &str = "v1 Usr";

// ============================================================
// Auth
// ============================================================
const AUTH_DESC: &str = r#"
### 総則
- X-BD での認証時は、X-BD を入れ、apx_id=0、vdr_id=0、email & password はダミーを入力
- APX として認証する場合、apx_id=0、vdr_id=0、email & password は当該APXのもの
- VDR として認証する場合、apx_id=所属ApxID、vdr_id=0、email & password は当該VDRのもの
- USR として認証する場合、apx_id=所属ApxID、vdr_id=所属VdrID、email & password は当該USRのもの
- expire は hour で指定すること
### スタッフについて
- USRは、VDR の権限により、VDRのスタッフになることができる
- スタッフとしての立場を与えられた USRは、その後、スタッフとしての token のみを取得できる
- スタッフ token を使用した場合、システム内で常に VDR として振る舞うことになる
- その場合、全ての操作は当該 VDR が行ったものと同一の結果となる
- 行った操作が、どのスタッフによるものか記録したい場合は、token payload 内の usr_id で記録できる
- システム内部においては、ju.StaffID がそれにあたる
### 注意
- スタッフであるかどうかの確認は、tokenの取得のタイミングで1度だけ行われる
- 取得した token が、スタッフであるか否かを示す唯一の証明書である
- 当該 USR が真にスタッフであるかを問わず、システムは token によってのみスタッフか否かを判断する
- つまり、スタッフ token を取得後、VDR により当該 USR がスタッフ権限を剥奪されたとしても、当該 token の expire までは、そのスタッフ token は有効である

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `apx_id` | number | required | APX ID |
| `vdr_id` | number | required | VDR ID |
| `email` | string | required | メールアドレス |
| `password` | string | required | パスワード |
| `expire` | number | required | トークン有効期限（hour） |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/usrs/auth/{apx_id}/{vdr_id}",
    summary = "認証を行い、tokenを返す。",
    description = AUTH_DESC,
    params(
        ("X-BD" = Option<String>, Header),
        ("apx_id" = u32, Path),
        ("vdr_id" = u32, Path),
        AuthUsrReq,
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn auth_usr(
    headers: HeaderMap,
    Path((apx_id, vdr_id)): Path<(u32, u32)>,
    Query(req): Query<AuthUsrReq>,
    Extension(jwt_config): Extension<Arc<JwtConfig>>,
    Extension(db): Extension<Arc<DbPools>>,
) -> Result<Json<AuthUsrRes>, ApiError> {
    let conn = db.get_ro_for_rt()?;
    let x_bd = headers.get("X-BD").and_then(|h| h.to_str().ok()).unwrap_or("");
    let has_bd = !x_bd.is_empty();
    if has_bd { // For BD
        let token = jwt::auth_bd(conn, x_bd, &jwt_config.skey, req.expire)
            .await
            .map_err(|e| ApiError::new(StatusCode::UNAUTHORIZED, e.to_string()))?;
        return Ok(Json(AuthUsrRes { token }))
    } else { // 通常のユーザー認証
        if jwt::is_apx(&apx_id, &vdr_id) { // For APX
            let token = jwt::auth_apx(conn, req.email.clone(), req.password.clone(), &jwt_config.skey, req.expire)
                .await
                .map_err(|e| ApiError::new(StatusCode::UNAUTHORIZED, e.to_string()))?;
            return Ok(Json(AuthUsrRes { token }));
        } else if jwt::is_vdr(&apx_id, &vdr_id) { // For VDR
            let token = jwt::auth_vdr(conn, apx_id, req.email.clone(), req.password.clone(), &jwt_config.skey, req.expire)
                .await
                .map_err(|e| ApiError::new(StatusCode::UNAUTHORIZED, e.to_string()))?;
            return Ok(Json(AuthUsrRes { token }));
        } else if jwt::is_usr(&apx_id, &vdr_id) { // For USR
            let token = jwt::auth_usr(conn, apx_id, vdr_id, req.email.clone(), req.password.clone(), &jwt_config.skey, req.expire)
                .await
                .map_err(|e| ApiError::new(StatusCode::UNAUTHORIZED, e.to_string()))?;
            return Ok(Json(AuthUsrRes { token }));
        } else {
            return Err(ApiError::new(StatusCode::UNAUTHORIZED, "Invalid APX ID or VDR ID."));
        }
    }
}

// ============================================================
// Search
// ============================================================
const SEARCH_DESC: &str = r#"
### ⚫︎ 概要
- ユーザーを検索する。
- ユーザーを検索する。
- ユーザーを検索する。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `name` | string | required | ユーザー名 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/usrs/search",
    summary = "ユーザーを検索する。",
    description = SEARCH_DESC,
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn search_usrs(
    Extension(_db): Extension<Arc<DbPools>>,
) -> Result<impl IntoResponse, ApiError> {
    Ok("Hello, World!")
}

// ============================================================
// Get
// ============================================================
const GET_DESC: &str = r#"
### ⚫︎ 概要
- ユーザー情報を1件取得する。
- ユーザー情報を1件取得する。
- ユーザー情報を1件取得する。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | string | required | ユーザーID |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    security(("api_jwt_token" = [])),
    path = "/usrs/{id}",
    summary = "ユーザー情報を1件取得する。",
    description = GET_DESC,
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_usr(
    Extension(_db): Extension<Arc<DbPools>>,
) -> Result<impl IntoResponse, ApiError> {
    Ok("Hello, World!")
}

// ============================================================ 
// Create
// ============================================================ 
const CREATE_DESC: &str = r#"
### ⚫︎ 概要
- ユーザーを新規作成する。
- ユーザーを新規作成する。
- ユーザーを新規作成する。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `name` | string | required | ユーザー名 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/usrs",
    summary = "ユーザーを新規作成する。",
    description = CREATE_DESC,
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn create_usr(
    Extension(_db): Extension<Arc<DbPools>>,
) -> Result<impl IntoResponse, ApiError> {
    Ok("Hello, World!")
}

// ============================================================ 
// Update
// ============================================================ 
const UPDATE_DESC: &str = r#"
### ⚫︎ 概要
- ユーザー情報を更新する。
- ユーザー情報を更新する。
- ユーザー情報を更新する。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | string | required | ユーザーID |
"#;
#[utoipa::path(
    tag = TAG,
    patch,
    security(("api_jwt_token" = [])),
    path = "/usrs/{id}",
    summary = "ユーザー情報を更新する。",
    description = UPDATE_DESC,
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn update_usr(
    Extension(_db): Extension<Arc<DbPools>>,
) -> Result<impl IntoResponse, ApiError> {
    Ok("Hello, World!")
}

// ============================================================ 
// Delete
// ============================================================ 
const DELETE_DESC: &str = r#"
### ⚫︎ 概要
- ユーザーを削除する。
- ユーザーを削除する。
- ユーザーを削除する。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | string | required | ユーザーID |
"#;
#[utoipa::path(
    tag = TAG,
    delete,
    security(("api_jwt_token" = [])),
    path = "/usrs/{id}",
    summary = "ユーザーを削除する。",
    description = DELETE_DESC,
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn delete_usr(
    Extension(_db): Extension<Arc<DbPools>>,
) -> Result<impl IntoResponse, ApiError> {
    Ok("Hello, World!")
}

