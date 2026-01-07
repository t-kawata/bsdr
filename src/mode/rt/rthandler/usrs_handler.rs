use std::sync::Arc;
use axum::{Extension, Json, extract::{Path, Query}, http::{header::HeaderValue, StatusCode}, response::IntoResponse};
use garde::Validate;
use crate::{
    mode::rt::{
        rtreq::usrs_req::{AuthUsrReq, SearchUsrsReq, UpdateUsrReq, CreateUsrReq},
        rtres::{errs_res::ApiError, usrs_res::{AuthUsrRes, SearchUsrsRes, GetUsrRes, CreateUsrRes, UpdateUsrRes, DeleteUsrRes, HireUsrRes, DehireUsrRes}},
        rterr::rterr,
        rtutils::db_for_rt::DbPoolsExt
    },
    utils::{db::DbPools, jwt::{self, JwtConfig, JwtUsr, JwtIDs, JwtRole}}
};

type HeaderMap = axum::http::HeaderMap;

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
        (status = 200, description = "Success", body = AuthUsrRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
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
    let x_bd = headers.get("X-BD").and_then(|h: &HeaderValue| h.to_str().ok()).unwrap_or("");
    let has_bd = !x_bd.is_empty();
    let expire = req.expire.unwrap_or(24);
    if has_bd { // For BD
        log::debug!("<Auth> BD attempt. expire: {}h", expire);
        let token = jwt::auth_bd(conn, x_bd, &jwt_config.skey, expire)
            .await
            .map_err(|e| {
                log::debug!("<Auth> BD failed: {}", e);
                ApiError::new_system(StatusCode::UNAUTHORIZED, rterr::ERR_AUTH, e.to_string())
            })?;
        log::debug!("<Auth> BD success.");
        return Ok(Json(AuthUsrRes { token }))
    } else { // 通常のユーザー認証
        if jwt::is_apx(&apx_id, &vdr_id, &1) { // For APX (uid is dummy > 0)
            log::debug!("<Auth> APX attempt. email: {}, expire: {}h", req.email, expire);
            let token = jwt::auth_apx(conn, req.email.clone(), req.password.clone(), &jwt_config.skey, expire)
                .await
                .map_err(|e| {
                    log::debug!("<Auth> APX failed for {}: {}", req.email, e);
                    ApiError::new_system(StatusCode::UNAUTHORIZED, rterr::ERR_AUTH, e.to_string())
                })?;
            log::debug!("<Auth> APX success for {}.", req.email);
            return Ok(Json(AuthUsrRes { token }));
        } else if jwt::is_vdr(&apx_id, &vdr_id, &1) { // For VDR (uid is dummy > 0)
            log::debug!("<Auth> VDR attempt. apx: {}, email: {}, expire: {}h", apx_id, req.email, expire);
            let token = jwt::auth_vdr(conn, apx_id, req.email.clone(), req.password.clone(), &jwt_config.skey, expire)
                .await
                .map_err(|e| {
                    log::debug!("<Auth> VDR failed for apx:{} email:{}: {}", apx_id, req.email, e);
                    ApiError::new_system(StatusCode::UNAUTHORIZED, rterr::ERR_AUTH, e.to_string())
                })?;
            log::debug!("<Auth> VDR success for apx:{} email:{}.", apx_id, req.email);
            return Ok(Json(AuthUsrRes { token }));
        } else if jwt::is_usr(&apx_id, &vdr_id, &1) { // For USR (uid is dummy > 0)
            log::debug!("<Auth> USR attempt. apx: {}, vdr: {}, email: {}, expire: {}h", apx_id, vdr_id, req.email, expire);
            let token = jwt::auth_usr(conn, apx_id, vdr_id, req.email.clone(), req.password.clone(), &jwt_config.skey, expire)
                .await
                .map_err(|e| {
                    log::debug!("<Auth> USR failed for apx:{} vdr:{} email:{}: {}", apx_id, vdr_id, req.email, e);
                    ApiError::new_system(StatusCode::UNAUTHORIZED, rterr::ERR_AUTH, e.to_string())
                })?;
            log::debug!("<Auth> USR success for apx:{} vdr:{} email:{}.", apx_id, vdr_id, req.email);
            return Ok(Json(AuthUsrRes { token }));
        } else {
            log::debug!("<Auth> Invalid ID combination. apx: {}, vdr: {}", apx_id, vdr_id);
            return Err(ApiError::new_system(StatusCode::UNAUTHORIZED, rterr::ERR_INVALID_REQUEST, "Invalid APX ID or VDR ID."));
        }
    }
}

// ============================================================
// Search
// ============================================================
const SEARCH_DESC: &str = r#"
### ⚫︎ 概要
- VD は全てのユーザを検索できる
- APX は配下の VDR 以下の全てのユーザを検索できる
- VDR は、配下の全てのユーザを検索できる
- USR は使用できない

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `name` | string | max=50 | ユーザー名 |
| `email` | string | email, half, max=50 | メールアドレス |
| `bgn_at` | string | required, datetime | 開始日時 |
| `end_at` | string | required, datetime | 終了日時 |
| `limit` | number | gte=1, lte=25 | 取得数 |
| `offset` | number | gte=0 | オフセット |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/usrs/search",
    summary = "ユーザーを検索する。",
    description = SEARCH_DESC,
    request_body = SearchUsrsReq,
    responses(
        (status = 200, description = "Success", body = SearchUsrsRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn search_usrs(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Json(req): Json<SearchUsrsReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::BD, JwtRole::APX, JwtRole::VDR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_ro_for_rt()?;
    let res = crate::mode::rt::rtbl::usrs_bl::search_usrs(conn, &ju, &ids, req).await?;
    Ok(Json(res))
}

// ============================================================
// Get
// ============================================================
const GET_DESC: &str = r#"
### ⚫︎ 概要
- VD は全てのユーザを取得できる
- APX は配下の VDR 以下の全てのユーザを取得できる
- VDR は、配下の全てのユーザを取得できる
- USR は使用できない

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `usr_id` | number | required, gte=1 | ユーザーID |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    security(("api_jwt_token" = [])),
    path = "/usrs/{usr_id}",
    summary = "ユーザー情報を1件取得する。",
    description = GET_DESC,
    params(
        ("usr_id" = u32, Path),
    ),
    responses(
        (status = 200, description = "Success", body = GetUsrRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_usr(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Path(usr_id): Path<u32>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::APX, JwtRole::VDR, JwtRole::USR])?;
    let conn = db.get_ro_for_rt()?;
    let res = crate::mode::rt::rtbl::usrs_bl::get_usr(conn, &ju, &ids, usr_id).await?;
    Ok(Json(res))
}

// ============================================================ 
// Create
// ============================================================ 
const CREATE_DESC: &str = r#"
### ⚫︎ 概要
- BD で取得した token では APX のみを作成できる
- APX で取得した token では VDR のみを作成できる
- VDR で取得した token では USR のみを作成できる
- USR は USR を作れない

### パラメータについて
- type: 1: 法人, 2: 個人 (VDR作成時は無視される)
- base_point: VDRのみ必須 (バッジ授与時に授与者である個人に付与される基本ポイント数)
- belong_rate: VDRのみ必須 (所属によるポイント割増率)
- max_works: VDRのみ必須 (VDR内の個人が就労できる最大数)
- flush_fee_rate: VDRのみ必須 (現金プールを現金分配実行する時に、事務コストを賄うために Pool から引かれる割合)
- flush_days: 法人のみ必須 (現金プールを現金分配実行するためのサイクルとなる日数)
- rate: 法人のみ必須 (法人が、自分に所属するユーザーに対して付与する割増ポイント率)
- VDR作成時以外にVDR用項目を送信するとエラーとなる
- 法人作成時以外に法人用項目を送信するとエラーとなる

### name について
- type=2 (個人) の場合、姓名の間にスペース（半角・全角問わず）が必須
- 全角スペースは半角スペースに変換され、連続するスペースは1つにまとめられる

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `name` | string | required, max=50 | ユーザー名 |
| `email` | string | required, email, max=50 | メールアドレス |
| `password` | string | required, password | パスワード |
| `bgn_at` | string | required, datetime | 開始日時 |
| `end_at` | string | required, datetime | 終了日時 |
| `type` | number | 🔴 APX/VDRでは入れないこと(omitempty), oneof=1 2 | 1: 法人, 2: 個人 |
| `base_point` | number | ⭐️ VDR必須, gte=0 | 基本ポイント数 |
| `belong_rate` | number | ⭐️ VDR必須, gte=0 | 所属割増率 |
| `max_works` | number | ⭐️ VDR必須, gte=0 | 最大就労数 |
| `flush_fee_rate` | number | ⭐️ VDR必須, gte=0 | 事務コスト分配率 |
| `flush_days` | number | 🔷 法人必須, gte=0 | 現金分配サイクル日数 |
| `rate` | number | 🔷 法人必須, gte=0 | 割増ポイント率 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/usrs",
    summary = "ユーザーを新規作成する。",
    description = CREATE_DESC,
    request_body = CreateUsrReq,
    responses(
        (status = 200, description = "Success", body = CreateUsrRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn create_usr(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Json(req): Json<CreateUsrReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::BD, JwtRole::APX, JwtRole::VDR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_rw_for_rt()?;
    let res = crate::mode::rt::rtbl::usrs_bl::create_usr(conn, &ju, &ids, req).await?;
    Ok(Json(res))
}

// ============================================================ 
// Update
// ============================================================ 
const UPDATE_DESC: &str = r#"
### ⚫︎ 概要
- BD は安全の為、更新権限を持たない
- APX は配下の VDR 以下の全てのユーザを更新できる
- VDR は、配下の全てのユーザを更新できる
- USR は使用できない

### パラメータについて
- type: 1: 法人, 2: 個人 (VDR作成時は無視される)
- base_point: VDRのみ必須 (バッジ授与時に授与者である個人に付与される基本ポイント数)
- belong_rate: VDRのみ必須 (所属によるポイント割増率)
- max_works: VDRのみ必須 (VDR内の個人が就労できる最大数)
- flush_fee_rate: VDRのみ必須 (現金プールを現金分配実行する時に、事務コストを賄うために Pool から引かれる割合)
- flush_days: 法人のみ必須 (現金プールを現金分配実行するためのサイクルとなる日数)
- rate: 法人のみ必須 (法人が、自分に所属するユーザーに対して付与する割増ポイント率)
- VDR作成時以外にVDR用項目を送信するとエラーとなる
- 法人作成時以外に法人用項目を送信するとエラーとなる

### name について
- type=2 (個人) の場合、姓名の間にスペース（半角・全角問わず）が必須
- 全角スペースは半角スペースに変換され、連続するスペースは1つにまとめられる

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `usr_id` | number | required, gte=1 | ユーザーID |
| `name` | string | max=50 | ユーザー名 |
| `email` | string | email, max=50 | メールアドレス |
| `password` | string | password | パスワード |
| `bgn_at` | string | datetime | 開始日時 |
| `end_at` | string | datetime | 終了日時 |
| `type` | number | 🔴 APX/VDRでは入れないこと(omitempty), oneof=1 2 | 1: 法人, 2: 個人 |
| `base_point` | number | ⭐️ VDR必須, gte=0 | 基本ポイント数 |
| `belong_rate` | number | ⭐️ VDR必須, gte=0 | 所属割増率 |
| `max_works` | number | ⭐️ VDR必須, gte=0 | 最大就労数 |
| `flush_fee_rate` | number | ⭐️ VDR必須, gte=0 | 事務コスト分配率 |
| `flush_days` | number | 🔷 法人必須, gte=0 | 現金分配サイクル日数 |
| `rate` | number | 🔷 法人必須, gte=0 | 割増ポイント率 |
"#;
#[utoipa::path(
    tag = TAG,
    patch,
    security(("api_jwt_token" = [])),
    path = "/usrs/{usr_id}",
    summary = "ユーザー情報を更新する。",
    description = UPDATE_DESC,
    params(
        ("usr_id" = u32, Path),
    ),
    request_body = UpdateUsrReq,
    responses(
        (status = 200, description = "Success", body = UpdateUsrRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn update_usr(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Path(usr_id): Path<u32>,
    Json(req): Json<UpdateUsrReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::APX, JwtRole::VDR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_rw_for_rt()?;
    let res = crate::mode::rt::rtbl::usrs_bl::update_usr(conn, &ju, &ids, usr_id, req).await?;
    Ok(Json(res))
}

// ============================================================ 
// Delete
// ============================================================ 
const DELETE_DESC: &str = r#"
### ⚫︎ 概要
- BD は安全の為、削除権限を持たない
- APX は配下の VDR 以下の全てのユーザを削除できる
- VDR は、配下の全てのユーザを削除できる
- USR は使用できない

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `usr_id` | number | required, gte=1 | ユーザーID |
"#;
#[utoipa::path(
    tag = TAG,
    delete,
    security(("api_jwt_token" = [])),
    path = "/usrs/{usr_id}",
    summary = "ユーザーを削除する。",
    description = DELETE_DESC,
    params(
        ("usr_id" = u32, Path),
    ),
    responses(
        (status = 200, description = "Success", body = DeleteUsrRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn delete_usr(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Path(usr_id): Path<u32>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::APX, JwtRole::VDR])?;
    let conn = db.get_rw_for_rt()?;
    let res = crate::mode::rt::rtbl::usrs_bl::delete_usr(conn, &ju, &ids, usr_id).await?;
    Ok(Json(res))
}


// ============================================================ 
// Hire
// ============================================================ 
const HIRE_DESC: &str = r#"
### ⚫︎ 概要
- VDR は、配下の USR に対してスタッフ権限を付与できる
- スタッフとなった USR は、認証時にスタッフ token を取得できるようになる
- スタッフ token は VDR と同等の権限を持つ
- USR は自分自身のスタッフ権限を操作できない

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `usr_id` | number | required, gte=1 | ユーザーID |
"#;
#[utoipa::path(
    tag = TAG,
    patch,
    security(("api_jwt_token" = [])),
    path = "/usrs/{usr_id}/hire",
    summary = "ユーザーをスタッフとして雇用する。",
    description = HIRE_DESC,
    params(
        ("usr_id" = u32, Path),
    ),
    responses(
        (status = 200, description = "Success", body = HireUsrRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn hire_usr(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Path(usr_id): Path<u32>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::VDR])?;
    let conn = db.get_rw_for_rt()?;
    let res = crate::mode::rt::rtbl::usrs_bl::hire_usr(conn, &ju, &ids, usr_id).await?;
    Ok(Json(res))
}

// ============================================================ 
// Dehire
// ============================================================ 
const DEHIRE_DESC: &str = r#"
### ⚫︎ 概要
- VDR は、配下の スタッフ に対してスタッフ権限を剥奪できる
- 権限剥奪後も、既に発行された token の expire までは有効であることに注意

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `usr_id` | number | required, gte=1 | ユーザーID |
"#;
#[utoipa::path(
    tag = TAG,
    delete,
    security(("api_jwt_token" = [])),
    path = "/usrs/{usr_id}/hire",
    summary = "ユーザーのスタッフ権限を解除（解雇）する。",
    description = DEHIRE_DESC,
    params(
        ("usr_id" = u32, Path),
    ),
    responses(
        (status = 200, description = "Success", body = DehireUsrRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn dehire_usr(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Path(usr_id): Path<u32>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::VDR])?;
    let conn = db.get_rw_for_rt()?;
    let res = crate::mode::rt::rtbl::usrs_bl::dehire_usr(conn, &ju, &ids, usr_id).await?;
    Ok(Json(res))
}
