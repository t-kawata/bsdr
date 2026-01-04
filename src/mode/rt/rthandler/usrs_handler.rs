
const TAG: &str = "v1 Usr";

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
pub async fn search_usrs() -> &'static str {
    "Hello, World!"
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
pub async fn get_usr() -> &'static str {
    "Hello, World!"
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
pub async fn create_usr() -> &'static str {
    "Hello, World!"
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
pub async fn update_usr() -> &'static str {
    "Hello, World!"
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
pub async fn delete_usr() -> &'static str {
    "Hello, World!"
}

