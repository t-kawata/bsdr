# REST API 開発厳格ルール

本ドキュメントは、本プロジェクトにおいて REST API を新しく実装、あるいは変更する際に遵守しなければならない厳格なルールを定めたものである。
「別のプロジェクトで同様の形式の API を実装する際のバイブル」となるよう、設計思想からコードの書き方までを網羅する。

## 実装の基本サイクル

開発は以下の順序で進めることを推奨する。この順序は、「インターフェース（入口）」を最初に固め、徐々に「内部ロジック（詳細）」へと進むことで、設計の矛盾を早期に発見することを目的としている。

1. **[Route]** `src/mode/rt/req_map.rs`: エンドポイントのパスを定義する（入口の設計）
2. **[Handler]** `src/mode/rt/rthandler/`: 窓口となる関数を作成する（制御の設計）
3. **[Request]** `src/mode/rt/rtreq/`: 入力データの構造とバリデーションを定義する（データの設計）
4. **[Response]** `src/mode/rt/rtres/`: 出力データの構造を定義する（結果の設計）
5. **[Logic]** `src/mode/rt/rtbl/`: 実際の業務ロジックを実装する（詳細の実装）

---

## 1. Routing 登録 (`src/mode/rt/req_map.rs`)

### 所在・ディレクトリ
- `src/mode/rt/req_map.rs`

### 目的
API のエンドポイント（URL パス）と、それを処理するハンドラを紐づける。また、Swagger 用のドキュメント（OpenAPI スキーマ）を自動生成するための基点となる。

### 記述ルール
- **CRUDの定義**: 本プロジェクトにおいて CRUD とは **Search / Get / Create / Update / Delete** の 5 つの操作を指す。
- **記述順序**: ファイル内での記述順序（抽出器の定義やルート登録等）は、必ず **Search -> Get -> Create -> Update -> Delete** の順でなければならない。
- `utoipa_axum` の `OpenApiRouter` を使用する。
- ハンドラーファイル内で定義された関数を `routes!` マクロを使用して登録する。
- 検索系の API であっても、URL の最大長制限などのリスクを避けるため、クエリパラメータではなく Body JSON を使用し、HTTP メソッドは `POST` を選択することを基本とする（プロジェクト方針）。

```rust
// src/mode/rt/req_map.rs

// 1. ハンドラーファイル（複数形）から関数をインポート
use crate::mode::rt::rthandler::usrs_handler::*;

// 2. app_routes 内で新しいエンドポイントを登録
fn app_routes() -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(search_usrs)) // 検索：POST (Body JSON)
        .routes(routes!(create_usr))  // 作成：POST
        // 開発したハンドラをここへ並べていく
}
```

---

## 2. Handler 実装 (`src/mode/rt/rthandler/`)

### 所在・ディレクトリ
- `src/mode/rt/rthandler/[機能名]_handler.rs`

### 目的
HTTP リクエストの受信、認証・認可の確認、およびレスポンスの返却。ビジネスロジックはここには書かず、後述の `rtbl` へ処理を委譲する「調整役」に徹する。

### 命名規則
- ハンドラーファイルの `{機能名}` は、原則として `src/entities` 内のテーブル名単位（複数形）とする（例: `usrs_handler.rs`, `bds_handler.rs`）。
- テーブルやエンティティとして存在しないものでも、REST API エンドポイントとしてまとまる必要があると合理的に判断された「例外的なもの」に限り、独自の機能名でハンドラーを作成して良い。

### 記述ルール
- **CRUDの定義と順序**: 本プロジェクトにおいて CRUD とは **Search / Get / Create / Update / Delete** を指し、ハンドラーファイル内での関数定義はこの順序で並べなければならない。
- **TAGの共通化**: ハンドラーファイル一つに対し、必ず一つの `const TAG: &str = "..."` を定義して `#[utoipa::path(tag = TAG, ...)]` で使用しなければならない。
- **Descriptionの定数化**: `#[utoipa::path]` の `description` は、属性のすぐ上に定数として定義する。
    - 内容には、API の概要、リクエストパラメータの詳細表、注意点などを Markdown 形式で詳細に記述する。
    - **アクセス権限の明記**: どのロール（BD, APX, VDR, USR）がどのような権限を持つかを必ず箇条書きで記載し、実際のコード（`ju.allow_roles`）と完全に一致させなければならない。
- **アクセス制限 (`ju.allow_roles`)**: ハンドラー関数の最上部で必ず `ju.allow_roles` を呼び出し、適切なロールに制限すること。
- **引数**: 以下の抽出器を必須項目（認証済み前提の場合）として含める。
    - `ju: JwtUsr`: 認証済みユーザー情報。ロールチェックに使用。
    - `ids: JwtIDs`: ロールに応じた実効ID（apx_id, vdr_id 等）。
    - `Extension(db): Extension<Arc<DbPools>>`: データベースプール。
- **返却値**: 成功時は `Result<Json<T>, ApiError>` を返し、エラー時はプロジェクト共通の `ApiError` を使用する。

```rust
// src/mode/rt/rthandler/usrs_handler.rs

const TAG: &str = "v1 Usr";

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
    responses(
        (status = 200, description = "Success")
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
    responses(
        (status = 200, description = "Success")
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
| `email` | string | required, email, half, max=50 | メールアドレス |
| `password` | string | required, password | パスワード |
| `bgn_at` | string | required, datetime | 開始日時 |
| `end_at` | string | required, datetime | 終了日時 |
| `type` | number | omitempty, oneof=1 2 | 1: 法人, 2: 個人 |
| `base_point` | number | gte=0 | 基本ポイント数 |
| `belong_rate` | number | gte=0 | 所属割増率 |
| `max_works` | number | gte=0 | 最大就労数 |
| `flush_days` | number | gte=0 | 現金分配サイクル日数 |
| `rate` | number | gte=0 | 割増ポイント率 |
| `flush_fee_rate` | number | gte=0 | 事務コスト分配率 |
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
| `email` | string | email, half, max=50 | メールアドレス |
| `password` | string | password | パスワード |
| `bgn_at` | string | datetime | 開始日時 |
| `end_at` | string | datetime | 終了日時 |
| `type` | number | oneof=1 2 | 1: 法人, 2: 個人 |
| `base_point` | number | gte=0 | 基本ポイント数 |
| `belong_rate` | number | gte=0 | 所属割増率 |
| `max_works` | number | gte=0 | 最大就労数 |
| `flush_days` | number | gte=0 | 現金分配サイクル日数 |
| `rate` | number | gte=0 | 割増ポイント率 |
| `flush_fee_rate` | number | gte=0 | 事務コスト分配率 |
"#;
#[utoipa::path(
    tag = TAG,
    patch,
    security(("api_jwt_token" = [])),
    path = "/usrs/{usr_id}",
    summary = "ユーザー情報を更新する。",
    description = UPDATE_DESC,
    responses(
        (status = 200, description = "Success")
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
    responses(
        (status = 200, description = "Success")
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
```

---

## 3. Request 定義 (`src/mode/rt/rtreq/`)

### 所在・ディレクトリ
- `src/mode/rt/rtreq/[機能名]_req.rs`

### 命名規則
- `{機能名}` は、ハンドラーと同様に `src/entities` 内のテーブル名単位（複数形）とする。
- **重要**: 必ず対応するハンドラーファイル名（`[機能名]_handler.rs`）と同じ機能名を使用しなければならない。

### 目的
クライアントから送信されるデータの型定義と、その妥当性検証（バリデーション）。

### 記述ルール（最重要）
- **CRUDの定義と順序**: 本プロジェクトにおいて CRUD とは **Search / Get / Create / Update / Delete** を指し、リクエスト構造体の定義順序はこの順序に従わなければならない。
- **エラーコードの義務**: 全てのバリデーションエラーには、システム共通のエラーコード（例: `E0006`）を付与しなければならない。
- **カスタムアダプターの使用**: `garde` の標準属性（`range` 等）を直接使うのは禁止。必ず `src/mode/rt/rterr/` で定義されたカスタムアダプターを `custom(...)` 経由で使用する。
- **Option型の扱い**: `Option<T>` フィールドを検証する場合は、`inner(...)` 属性を使用して包みの中身を検証する。

```rust
use crate::mode::rt::rterr::rterr::*;

#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateUsrReq {
    /// 必須・形式チェック（アダプター名_ルール名_err の形式）
    #[garde(custom(required_simple_err(1, 50)))] // 文字列の必須チェック
    #[garde(custom(email_err))]                 // メール形式
    pub email: String,

    /// 数値の範囲チェック（境界値の型を明示）
    #[garde(custom(range_err(Some(1u8), Some(2u8))))]
    pub usr_type: u8,

    /// 任意項目の場合
    #[garde(inner(custom(datetime_err)))] // 入力があった時のみ日時形式をチェック
    pub bgn_at: Option<String>,
}
```

---

## 4. Response 定義 (`src/mode/rt/rtres/`)

### 所在・ディレクトリ
- `src/mode/rt/rtres/[機能名]_res.rs`

### 命名規則
- `{機能名}` は、ハンドラーと同様に `src/entities` 内のテーブル名単位（複数形）とする。
- **重要**: 必ず対応するハンドラーファイル名（`[機能名]_handler.rs`）と同じ機能名を使用しなければならない。

### 目的
クライアントへ返却する JSON データの構造定義。Rust らしい「フラットかつ型安全な」構造を採用する。

### 記述ルール
- **CRUDの定義と順序**: 本プロジェクトにおいて CRUD とは **Search / Get / Create / Update / Delete** を指し、レスポンス構造体の定義順序はこの順序に従わなければならない。
- **フラットな構造**: レスポンス構造体は、内部構造体でラップせず直接フィールドを持つフラットな構成にする。
- **エラーフィールドの排除**: 成功時のレスポンス構造体に `errors` フィールドを持たせてはならない。バリデーションエラーやシステムエラーは、すべて共通の `ApiError` 構造体を介して返却される。
- **型変換の規則**:
    - ID などの識別子: `u32`
    - 日時: `String` (共通ヘルパー `datetime_to_str` を使用して `YYYY-MM-DDThh:mm:ss` 形式に整形)
    - Decimal (金額・率): `f64` (SeaORM モデルからの変換時に `to_f64()` を使用)
- **From トレイトの実装**: データベースモデル（SeaORM）からレスポンス構造体への変換は、`From<Model>` トレイトを実装することで行う。
    - 実装場所: `use` 文はファイル最上部にまとめ、`From` の実装コードは各セクション（Search / Get）の末尾、つまり構造体定義の直下に記述する。

### 実装例

```rust
use utoipa::ToSchema;
use serde::Serialize;
use rust_decimal::prelude::ToPrimitive;
use crate::entities::usrs;
use crate::utils::db::datetime_to_str;

// ============================================================ 
// Search
// ============================================================ 
#[derive(Serialize, ToSchema)]
pub struct SearchUsrsRes {
    pub usrs: Vec<SearchUsrsResItem>,
}

#[derive(Serialize, ToSchema)]
pub struct SearchUsrsResItem {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub bgn_at: String,
    pub end_at: String,
}

impl From<usrs::Model> for SearchUsrsResItem {
    fn from(m: usrs::Model) -> Self {
        Self {
            id: m.id as u32,
            name: m.name,
            email: m.email,
            bgn_at: datetime_to_str(m.bgn_at),
            end_at: datetime_to_str(m.end_at),
        }
    }
}

// ============================================================ 
// Get
// ============================================================ 
#[derive(Serialize, ToSchema)]
pub struct GetUsrRes {
    pub id: u32,
    pub name: String,
    pub bgn_at: String,
    pub end_at: String,
}

impl From<usrs::Model> for GetUsrRes {
    fn from(m: usrs::Model) -> Self {
        Self {
            id: m.id as u32,
            name: m.name,
            bgn_at: datetime_to_str(m.bgn_at),
            end_at: datetime_to_str(m.end_at),
        }
    }
}

// ============================================================ 
// Create / Update / Delete (書き込み系は対象の ID を返す)
// ============================================================ 
#[derive(Serialize, ToSchema)]
pub struct CreateUsrRes { pub id: u32 }

#[derive(Serialize, ToSchema)]
pub struct UpdateUsrRes { pub id: u32 }

#[derive(Serialize, ToSchema)]
pub struct DeleteUsrRes { pub id: u32 }
```

---

## 5. Business Logic 実装 (`src/mode/rt/rtbl/`)

### 所在・ディレクトリ
- `src/mode/rt/rtbl/[機能名]_bl.rs`

### 命名規則
- `{機能名}` は、ハンドラーと同様に `src/entities` 内のテーブル名単位（複数形）とする。
- **重要**: 必ず対応するハンドラーファイル名（`[機能名]_handler.rs`）と同じ機能名を使用しなければならない。

### 目的
データベース操作（SeaORM）、外部連携、複雑なロジックの実行。Handler から呼び出される純粋な Rust ロジック。

### 記述ルール
- **CRUDの定義と順序**: 本プロジェクトにおいて CRUD とは **Search / Get / Create / Update / Delete** を指し、ビジネスロジックの実装順序（関数定義順）はこの順序に従わなければならない。
- **クエリ構築の共通化 (Private Helper)**: `Search` および `Get` で使用するクエリ構築ロジック（特に権限に基づく参照範囲のフィルタリング）は、プライベートなヘルパー関数（例: `find_[resource]_base`）として切り出し、一元管理すること。
- **読み取り・更新における共通ロジックの利用**: `Update` および `Delete` を行う際は、必ず上記の共通ヘルパー関数を経由して対象レコードを取得すること。これにより、存在確認 (404) と権限/パーティションチェック（アクセス不可なレコードの排除）を同一のロジックで安全に行うことができる。
- **厳格なデータパーティショニング**: `apx_id` および `vdr_id` は完全なデータパーティションとして扱う。ロールに応じて、以下のフィルタリングを**常に、かつ漏れなく**適用しなければならない。
    - **APX**: `apx_id` による絞り込み。
    - **VDR**: `apx_id` および `vdr_id` による絞り込み。
    - **USR**: `apx_id`, `vdr_id`, および `id` (自分自身) による絞り込み。
- **ロール判定の明示化**: 権限チェックや参照範囲の分岐には、`ju.role()` と `match` を用いて、各ロールの挙動を意図的に明示すること。
- **任意項目の更新処理 (Update)**: `UpdateUsrReq` のような `Option` 型フィールドを含む更新リクエストでは、`if let Some(...)` 等を用いて値が存在する場合のみ更新を適用すること。また、更新後の値に基づいたバリデーション（例: 個人名のスペースチェック）が必要な場合は、既存の値と更新値を考慮して適切に行うこと。
- **パスワードのハッシュ化**: パスワードなどの秘匿情報を更新・保存する際は、必ず `crypto::get_hash_with_cost` 等を用いてハッシュ化を行うこと。
- **セクション区切り**: 
    - 各 CRUD 操作の区切りには `// =================...` を使用する。
    - 各操作内部のステップ（1. クエリ取得, 2. 条件追加...）の区切りには `// ----------------...` を使用する。
- **エラー変換**: SeaORM のエラーは、`map_err` を用いて `ApiError` に変換し、適切なステータスコードと原因を明記すること。
- **関連データの削除 (Delete)**: 削除対象が親組織（VDR 等）である場合、将来的に関連する子データ（Jobs, Matches 等）のトランザクション内での一括削除が必要となる。Entity が未定義の場合は TODO を残し、定義後は必ず実装すること。

### 実装例 (Search / Get / Update / Delete)

```rust
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QuerySelect, Select, ActiveModelTrait, IntoActiveModel, Set, ModelTrait};
use crate::entities::usrs;
use crate::utils::jwt::{JwtUsr, JwtIDs, JwtRole};
use crate::mode::rt::rtreq::usrs_req::{SearchUsrsReq, UpdateUsrReq};
use crate::mode::rt::rtres::usrs_res::{SearchUsrsRes, SearchUsrsResItem, GetUsrRes, UpdateUsrRes, DeleteUsrRes};
use crate::mode::rt::rtres::errs_res::ApiError;
use axum::http::StatusCode;
use crate::mode::rt::rterr::rterr;
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use crate::utils::{crypto, db::str_to_datetime};

// ============================================================
// Private Helper for Search and Get
// ============================================================
/// 権限に基づいた共通のクエリベースを作成する
async fn find_usrs_base(
    ju: &JwtUsr,
    ids: &JwtIDs,
) -> Result<Select<usrs::Entity>, ApiError> {
    let query = usrs::Entity::find();
    // 権限に基づくフィルタリング
    // apx_id と vdr_id は完全なパーティションとして扱うため、
    // VDR/USR ロールでは常に両方の条件を含める。
    match ju.role() {
        JwtRole::BD => {
            // BD は全てのユーザを取得できる
            Ok(query)
        }
        JwtRole::APX => {
            // APX は配下の全てのユーザを取得できる
            Ok(query.filter(usrs::Column::ApxId.eq(ids.apx_id)))
        }
        JwtRole::VDR => {
            // VDR は配下の全てのユーザを取得できる
            // apx_id と vdr_id による厳密なパーティションフィルタ
            Ok(query
                .filter(usrs::Column::ApxId.eq(ids.apx_id))
                .filter(usrs::Column::VdrId.eq(ids.vdr_id)))
        }
        JwtRole::USR => {
            // USR は自分自身のレコードのみ
            // apx_id と vdr_id による厳密なパーティションフィルタをかけつつ、ID で特定
            Ok(query
                .filter(usrs::Column::ApxId.eq(ids.apx_id))
                .filter(usrs::Column::VdrId.eq(ids.vdr_id))
                .filter(usrs::Column::Id.eq(ids.usr_id)))
        }
    }
}

// ============================================================
// Search
// ============================================================
pub async fn search_usrs(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    req: SearchUsrsReq,
) -> Result<SearchUsrsRes, ApiError> {
    // --------------------------------
    // 1. クエリの基本形を取得
    // --------------------------------
    let mut query = find_usrs_base(ju, ids).await?;
    // --------------------------------
    // 2. 検索条件（LIKE検索）
    // --------------------------------
    if !req.name.is_empty() {
        query = query.filter(usrs::Column::Name.contains(&req.name));
    }
    if !req.email.is_empty() {
        query = query.filter(usrs::Column::Email.contains(&req.email));
    }
    // --------------------------------
    // 3. 日時範囲のフィルタリング
    // --------------------------------
    let format = "%Y-%m-%dT%H:%M:%S";
    let bgn_at = NaiveDateTime::parse_from_str(&req.bgn_at, format).map_err(|e| ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, format!("Invalid bgn_at: {}", e)))?;
    let end_at = NaiveDateTime::parse_from_str(&req.end_at, format).map_err(|e| ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, format!("Invalid end_at: {}", e)))?;
    // モデルの [BgnAt, EndAt] が [req.bgn_at, req.end_at] と重なるものを抽出
    query = query.filter(usrs::Column::BgnAt.lte(end_at))
                 .filter(usrs::Column::EndAt.gte(bgn_at));
    // --------------------------------
    // 4. データの取得
    // --------------------------------
    let models = query
        .offset(req.offset as u64)
        .limit(req.limit as u64)
        .all(conn)
        .await
        .map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Search query error: {}", e)))?;
    // --------------------------------
    // 5. DBデータのレスポンス用変換
    // --------------------------------
    let usrs = models.into_iter().map(SearchUsrsResItem::from).collect();
    // --------------------------------
    // 6. 最終レスポンス
    // --------------------------------
    Ok(SearchUsrsRes { usrs })
}

// ============================================================
// Get
// ============================================================
pub async fn get_usr(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    target_usr_id: u32,
) -> Result<GetUsrRes, ApiError> {
    // --------------------------------
    // 1. クエリの基本形を取得
    // --------------------------------
    let query = find_usrs_base(ju, ids).await?;
    // --------------------------------
    // 2. ユーザーの取得
    // --------------------------------
    let model = query
        .filter(usrs::Column::Id.eq(target_usr_id))
        .one(conn)
        .await
        .map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Fetch user error: {}", e)))?
        .ok_or_else(|| ApiError::new_system(StatusCode::NOT_FOUND, rterr::ERR_INVALID_REQUEST, "User not found."))?;
    Ok(GetUsrRes::from(model))
}

// ============================================================
// Create
// ============================================================
pub async fn create_usr(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    req: CreateUsrReq,
) -> Result<CreateUsrRes, ApiError> {
    // --------------------------------
    // 1. ロールに基づくパラメータバリデーションと初期値設定
    // --------------------------------
    let aid: Option<u32>;
    let vid: Option<u32>;
    let utype: u8;
    let target_label: &str;

    match ju.role() {
        JwtRole::BD => {
            // BD は APX のみ作成可能
            aid = None; // 新しい APX なので apx_id は空
            vid = None;
            utype = UsrType::Corp as u8; // APX は常に法人タイプ
            target_label = "APX";
            // 不要な項目があればエラー
            if req.usr_type.is_some() || req.base_point > 0 || req.belong_rate > 0.0 || req.max_works > 0 || req.flush_days > 0 || req.rate > 0.0 || req.flush_fee_rate > 0.0 {
                return Err(ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "BD can only create APX. Unnecessary parameters provided."));
            }
        }
        JwtRole::APX => {
            // APX は配下に VDR のみ作成可能
            aid = Some(ids.apx_id);
            vid = None; // 新しい VDR なので vdr_id は空
            utype = UsrType::Corp as u8; // VDR は常に法人タイプ
            target_label = "VDR";
            // VDR 必須項目のチェック
            if req.base_point == 0 || req.belong_rate == 0.0 || req.max_works == 0 || req.flush_fee_rate == 0.0 {
                return Err(ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "VDR requires base_point, belong_rate, max_works, and flush_fee_rate."));
            }
            // 不要な項目があればエラー
            if req.usr_type.is_some() || req.flush_days > 0 || req.rate > 0.0 {
                return Err(ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "APX can only create VDR. Unnecessary parameters provided."));
            }
        }
        JwtRole::VDR => {
            // VDR は配下に USR (個人/法人) を作成可能
            aid = Some(ids.apx_id);
            vid = Some(ids.vdr_id);
            target_label = "USR";
            // type は必須
            let t = req.usr_type.ok_or_else(|| ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Usr type is required."))?;
            utype = t;
            // 不要な項目のチェック
            if req.base_point > 0 || req.belong_rate > 0.0 || req.max_works > 0 || req.flush_fee_rate > 0.0 {
                return Err(ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "VDR cannot set base_point, belong_rate, max_works, or flush_fee_rate for USR."));
            }
            if utype == UsrType::Corp as u8 {
                // 法人としての必須項目
                if req.flush_days == 0 || req.rate == 0.0 {
                    return Err(ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Corporate USR requires flush_days and rate."));
                }
            } else if utype == UsrType::Indi as u8 {
                // 個人としてのチェック (不要な項目)
                if req.flush_days > 0 || req.rate > 0.0 {
                    return Err(ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Personal USR cannot have flush_days or rate."));
                }
            }
        }
        JwtRole::USR => {
            return Err(ApiError::new_system(StatusCode::FORBIDDEN, rterr::ERR_AUTH, "USR is not allowed to create users."));
        }
    }
    // --------------------------------
    // 2. メールアドレスの重複チェック (パーティション内)
    // --------------------------------
    let exists = usrs::Entity::find()
        .filter(usrs::Column::Email.eq(&req.email))
        .filter(usrs::Column::ApxId.eq(aid))
        .filter(usrs::Column::VdrId.eq(vid))
        .one(conn)
        .await
        .map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Email check error: {}", e)))?;
    if exists.is_some() {
        return Err(ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, format!("Email already exists as {}.", target_label)));
    }
    // --------------------------------
    // 3. 名前の正規化 (個人タイプの場合)
    // --------------------------------
    let mut name = req.name.clone();
    if utype == UsrType::Indi as u8 {
        name = name.replace('　', " ");
        while name.contains("  ") {
            name = name.replace("  ", " ");
        }
        name = name.trim().to_string();
        if !name.contains(' ') {
            return Err(ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Personal name must contain a space between first and last name."));
        }
    }
    // --------------------------------
    // 4. パスワードハッシュ化
    // --------------------------------
    let hashed_pw = crypto::get_hash_with_cost(&req.password, 10).map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_UNEXPECTED, format!("Password hash error: {}", e)))?;
    // --------------------------------
    // 5. 日時変換
    // --------------------------------
    let bgn_at = str_to_datetime(&req.bgn_at).map_err(|e| ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, format!("Invalid bgn_at: {}", e)))?;
    let end_at = str_to_datetime(&req.end_at).map_err(|e| ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, format!("Invalid end_at: {}", e)))?;
    // --------------------------------
    // 6. ActiveModel 作成と保存 (Transaction)
    // --------------------------------
    let created_id = conn.transaction::<_, u32, ApiError>(|tx| {
        Box::pin(async move {
            let mut active: usrs::ActiveModel = Default::default();
            active.apx_id = Set(aid);
            active.vdr_id = Set(vid);
            active.name = Set(name);
            active.email = Set(req.email);
            active.password = Set(hashed_pw);
            active.bgn_at = bgn_at;
            active.end_at = end_at;
            active.r#type = Set(utype);
            active.base_point = Set(req.base_point);
            active.belong_rate = Set(Decimal::from_f64(req.belong_rate).unwrap_or_default());
            active.max_works = Set(req.max_works);
            active.flush_days = Set(req.flush_days);
            active.rate = Set(Decimal::from_f64(req.rate).unwrap_or_default());
            active.flush_fee_rate = Set(Decimal::from_f64(req.flush_fee_rate).unwrap_or_default());
            let res: usrs::Model = active.insert(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Insert user error: {}", e)))?;
            
            // TODO: VDR作成時の Pool テーブル作成 (Entity定義後)
            /*
            if ju.is_apx() {
                // let pool = pools::ActiveModel { ... };
                // pool.insert(tx).await?;
            }
            */

            Ok(res.id as u32)
        })
    }).await?;
    // --------------------------------
    // 7. 最終レスポンス
    // --------------------------------
    Ok(CreateUsrRes { id: created_id })
}

// ============================================================
// Update
// ============================================================
pub async fn update_usr(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    target_usr_id: u32,
    req: UpdateUsrReq,
) -> Result<UpdateUsrRes, ApiError> {
    // --------------------------------
    // 1. クエリの基本形を取得して存在確認 (404/Partitionチェック)
    // --------------------------------
    let model = find_usrs_base(ju, ids).await?
        .filter(usrs::Column::Id.eq(target_usr_id))
        .one(conn)
        .await
        .map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Fetch user error: {}", e)))?
        .ok_or_else(|| ApiError::new_system(StatusCode::NOT_FOUND, rterr::ERR_INVALID_REQUEST, "User not found."))?;
    // --------------------------------
    // 2. 更新用 ActiveModel の準備
    // --------------------------------
    let mut active: usrs::ActiveModel = model.clone().into_active_model();
    // --------------------------------
    // 3. 各フィールドの更新
    // --------------------------------
    // Type (usr_type)
    let current_type = req.usr_type.unwrap_or(model.r#type);
    if let Some(t) = req.usr_type {
        active.r#type = Set(t);
    }
    // Name (個人 type=2 の場合はスペースチェック)
    if let Some(mut name) = req.name {
        if current_type == 2 {
            // 全角スペースを半角に変換 / 連続するスペースを1つに / トリム
            name = name.replace('　', " ");
            while name.contains("  ") { name = name.replace("  ", " "); }
            name = name.trim().to_string();
            // 姓名の間にスペースが必須
            if !name.contains(' ') {
                return Err(ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Personal name must contain a space."));
            }
        }
        active.name = Set(name);
    }
    if let Some(email) = req.email {
        active.email = Set(email);
    }
    if let Some(password) = req.password {
        let hashed = crypto::get_hash_with_cost(&password, 10).map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_UNEXPECTED, format!("Hash error: {}", e)))?;
        active.password = Set(hashed);
    }
    if let Some(bgn_at) = req.bgn_at {
        active.bgn_at = str_to_datetime(&bgn_at).map_err(|e| ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, e.to_string()))?;
    }
    if let Some(end_at) = req.end_at {
        active.end_at = str_to_datetime(&end_at).map_err(|e| ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, e.to_string()))?;
    }
    // 数値項目 (Decimal変換含む)
    if let Some(v) = req.base_point { active.base_point = Set(v); }
    if let Some(v) = req.belong_rate { 
        active.belong_rate = Set(Decimal::from_f64(v).ok_or_else(|| ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Invalid belong_rate"))?); 
    }
    if let Some(v) = req.max_works { active.max_works = Set(v); }
    if let Some(v) = req.flush_days { active.flush_days = Set(v); }
    if let Some(v) = req.rate { 
        active.rate = Set(Decimal::from_f64(v).ok_or_else(|| ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Invalid rate"))?); 
    }
    if let Some(v) = req.flush_fee_rate { 
        active.flush_fee_rate = Set(Decimal::from_f64(v).ok_or_else(|| ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Invalid flush_fee_rate"))?); 
    }
    // --------------------------------
    // 4. 保存
    // --------------------------------
    active.update(conn).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Update error: {}", e)))?;
    // --------------------------------
    // 5. 最終レスポンス
    // --------------------------------
    Ok(UpdateUsrRes { id: target_usr_id })
}

// ============================================================
// Delete
// ============================================================
pub async fn delete_usr(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    target_usr_id: u32,
) -> Result<DeleteUsrRes, ApiError> {
    // --------------------------------
    // 1. クエリの基本形を取得して存在確認 (404/Partitionチェック)
    // --------------------------------
    let model = find_usrs_base(ju, ids).await?
        .filter(usrs::Column::Id.eq(target_usr_id))
        .one(conn)
        .await
        .map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Fetch error: {}", e)))?
        .ok_or_else(|| ApiError::new_system(StatusCode::NOT_FOUND, rterr::ERR_INVALID_REQUEST, "User not found."))?;
    // --------------------------------
    // 2. 削除の実行
    // --------------------------------
    // TODO: VDR 削除時の関連テーブル（Jobs, Matches, etc.）の一括削除実装（Entity定義後）
    model.delete(conn).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete error: {}", e)))?;
    // --------------------------------
    // 3. 最終レスポンス
    // --------------------------------
    Ok(DeleteUsrRes { id: target_usr_id })
}
```

---

## 補足: バリデーションアダプターの追加方法 (`src/mode/rt/rterr/`)

`garde` 標準にない独自のバリデーションや、プロジェクト固有のエラーコードを付与するバリデーションを追加する際は、以下の **4 つのステップ** を省略することなく実行してください。ここでは「数値のみ（numeric）」と「日時形式（datetime）」の実装を例に解説します。

### ステップ 1: 基底ロジックの定義 (`rterr.rs` 等)

バリデーションの判定基準となる型やトレイト、あるいは解析ロジックを定義します。

**例：numeric のためのトレイト定義**
```rust
// src/mode/rt/rterr/rterr.rs

pub trait Numeric {
    fn is_numeric(&self) -> bool;
}

impl Numeric for String {
    fn is_numeric(&self) -> bool {
        !self.is_empty() && self.chars().all(|c| c.is_ascii_digit())
    }
}
```

### ステップ 2: 汎用マクロの作成 (`validators.rs`)

`garde` の `custom` ルールが期待する関数シグネチャ `fn(&T, &()) -> garde::Result` を生成するマクロを記述します。

**例：numeric / datetime マクロ**
```rust
// src/mode/rt/rterr/validators.rs

// 数値チェック用
#[macro_export]
macro_rules! define_numeric_adapter {
    ($name:ident, $code:expr, $msg:expr) => {
        pub fn $name<T: $crate::mode::rt::rterr::Numeric>(v: &T, _ctx: &()) -> garde::Result {
            if v.is_numeric() {
                Ok(())
            } else {
                Err(garde::Error::new(format!("{} | {}", $code, $msg)))
            }
        }
    };
}

// 日時形式チェック用 (chronoを使用)
#[macro_export]
macro_rules! define_datetime_adapter {
    ($name:ident, $format:expr, $code:expr, $msg:expr) => {
        pub fn $name<T: AsRef<str>>(v: &T, _ctx: &()) -> garde::Result {
            let s = v.as_ref();
            if chrono::NaiveDateTime::parse_from_str(s, $format).is_ok() {
                Ok(())
            } else {
                Err(garde::Error::new(format!(
                    "{} | {} (Expected format: {})",
                    $code, $msg, $format
                )))
            }
        }
    };
}
```

### ステップ 3: アダプターの実体化 (`rterr.rs`)

作成したマクロを呼び出し、一貫した命名規則に従ってアダプターを公開します。ここでエラーコード（`E`から始まる番号）とメッセージを確定させます。

**例：アダプターの定義**
```rust
// src/mode/rt/rterr/rterr.rs

// 命名規則: [ルール名]_[err]
define_numeric_adapter!(numeric_err, "E0022", "Must be numeric.");

// サブタイプがある場合: [ルール名]_[サブタイプ]_[err]
define_datetime_adapter!(datetime_err, "%Y-%m-%dT%H:%M:%S", "E0023", "Invalid datetime format.");
```

### ステップ 4: リクエスト構造体への適用 (`rtreq`)

`custom` 属性、または `inner(custom(...))` 属性を使用して、定義したアダプターを適用します。

**例：実際の使用例**
```rust
// src/mode/rt/rtreq/usrs_req.rs

#[derive(Validate)]
pub struct CreateUsrReq {
    /// 数字のみであることを検証
    #[garde(custom(numeric_err))]
    pub phone_number: String,

    /// 必須かつ日時形式であることを検証
    #[garde(custom(required_simple_err(1, 100)))]
    #[garde(custom(datetime_err))]
    pub bgn_at: String,

    /// 任意項目の場合
    #[garde(inner(custom(datetime_err)))]
    pub end_at: Option<String>,
}
```

このように、マクロによる共通化とアダプターの実体化を分離することで、`rtreq` 側では「どのルールを適用するか」だけを意識すれば良い、安全でスムーズな開発が可能になります。
