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
) -> Result<Json<CreateUsrRes>, ApiError> {
    // 処理...
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
クライアントへ返却する JSON データの構造定義。

### 記述ルール
- `serde::Serialize` および `utoipa::ToSchema` を derive する。
- API ごとに専用の構造体を用意し、将来的な変更（フィールド追加など）に備える。

```rust
// TODO: レスポンスの共通ラップ構造体（Success/Data等）の決定後に追記
#[derive(Serialize, ToSchema)]
pub struct SearchUsrsRes {
    pub users: Vec<UsrInfo>,
}
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

### 記述ルール (TODO)
- **トランザクション管理**: BL 内でのアトミックな操作の保証。
- **権限の伝播**: `JwtUsr` 情報を BL に渡し、データの所有者チェックを行う。

```rust
// TODO: 具体的な DB アクセスパターンの定義
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
