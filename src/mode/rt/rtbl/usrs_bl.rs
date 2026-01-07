use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QuerySelect, Select, ActiveModelTrait, IntoActiveModel, Set, ModelTrait, TransactionTrait, Condition};
use crate::entities::{usrs, pools, jobs, matches, match_statuses, works, belongs, badges, usr_badges, points, payments, flushes, payouts, cryptos};
use crate::utils::jwt::{JwtUsr, JwtIDs, JwtRole};
use crate::mode::rt::rtreq::usrs_req::{SearchUsrsReq, UpdateUsrReq, CreateUsrReq};
use crate::mode::rt::rtres::usrs_res::{SearchUsrsRes, SearchUsrsResItem, GetUsrRes, UpdateUsrRes, DeleteUsrRes, CreateUsrRes, HireUsrRes, DehireUsrRes};
use crate::mode::rt::rtres::errs_res::ApiError;
use axum::http::StatusCode;
use crate::mode::rt::rterr::rterr;
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use crate::enums::usrtype::UsrType;
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
    // IDs (JwtIDs) は既にロールに応じて正規化されている。
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
    let is_vdr_creation = ju.role() == JwtRole::APX;
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
            // VDR作成時のみ Pool を作成
            if is_vdr_creation {
                let pool = pools::ActiveModel {
                    apx_id: Set(aid.unwrap_or(0)),
                    vdr_id: Set(res.id as u32),
                    remain: Set(0),
                    total_in: Set(0),
                    total_out: Set(0),
                    ..Default::default()
                };
                pool.insert(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Insert pool error: {}", e)))?;
            }
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
    // 1. クエリの基本形を取得して存在確認
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
            // 全角スペースを半角に変換
            name = name.replace('　', " ");
            // 連続するスペースを1つに
            while name.contains("  ") {
                name = name.replace("  ", " ");
            }
            name = name.trim().to_string();
            // 姓名の間にスペースが必須
            if !name.contains(' ') {
                return Err(ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Personal name must contain a space between first and last name."));
            }
        }
        active.name = Set(name);
    }
    if let Some(email) = req.email {
        active.email = Set(email);
    }
    if let Some(password) = req.password {
        let hashed = crypto::get_hash_with_cost(&password, 10).map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_UNEXPECTED, format!("Password hash error: {}", e)))?;
        active.password = Set(hashed);
    }
    if let Some(bgn_at) = req.bgn_at {
        active.bgn_at = str_to_datetime(&bgn_at).map_err(|e| ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, e.to_string()))?;
    }
    if let Some(end_at) = req.end_at {
        active.end_at = str_to_datetime(&end_at).map_err(|e| ApiError::new_system(StatusCode::BAD_REQUEST, rterr::ERR_INVALID_REQUEST, e.to_string()))?;
    }
    // VDR/法人 関連項目
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
    active.update(conn)
        .await
        .map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Update user error: {}", e)))?;
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
    // 1. クエリの基本形を取得して存在確認
    // --------------------------------
    let model = find_usrs_base(ju, ids).await?
        .filter(usrs::Column::Id.eq(target_usr_id))
        .one(conn)
        .await
        .map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Fetch user error: {}", e)))?
        .ok_or_else(|| ApiError::new_system(StatusCode::NOT_FOUND, rterr::ERR_INVALID_REQUEST, "User not found."))?;
    // --------------------------------
    // 2. 削除の実行
    // --------------------------------
    conn.transaction::<_, (), ApiError>(|tx| {
        Box::pin(async move {
            let target_id = model.id as u32;
            if model.apx_id.is_some() && model.vdr_id.is_none() {
                // (1) VDR だった場合の一括削除
                let vid = target_id;
                usrs::Entity::delete_many().filter(usrs::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete sub-usrs error: {}", e)))?;
                jobs::Entity::delete_many().filter(jobs::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete jobs error: {}", e)))?;
                matches::Entity::delete_many().filter(matches::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete matches error: {}", e)))?;
                match_statuses::Entity::delete_many().filter(match_statuses::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete match_statuses error: {}", e)))?;
                works::Entity::delete_many().filter(works::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete works error: {}", e)))?;
                belongs::Entity::delete_many().filter(belongs::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete belongs error: {}", e)))?;
                badges::Entity::delete_many().filter(badges::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete badges error: {}", e)))?;
                usr_badges::Entity::delete_many().filter(usr_badges::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete usr_badges error: {}", e)))?;
                points::Entity::delete_many().filter(points::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete points error: {}", e)))?;
                payments::Entity::delete_many().filter(payments::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete payments error: {}", e)))?;
                pools::Entity::delete_many().filter(pools::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete pools error: {}", e)))?;
                flushes::Entity::delete_many().filter(flushes::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete flushes error: {}", e)))?;
                payouts::Entity::delete_many().filter(payouts::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete payouts error: {}", e)))?;
                cryptos::Entity::delete_many().filter(cryptos::Column::VdrId.eq(vid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete cryptos error: {}", e)))?;
            } else if model.apx_id.is_some() && model.vdr_id.is_some() {
                // (2) USR だった場合の一括削除
                let uid = target_id;
                // matches (from, to)
                matches::Entity::delete_many().filter(Condition::any().add(matches::Column::From.eq(uid)).add(matches::Column::To.eq(uid))).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete matches error: {}", e)))?;
                // match_statuses (from, to)
                match_statuses::Entity::delete_many().filter(Condition::any().add(match_statuses::Column::From.eq(uid)).add(match_statuses::Column::To.eq(uid))).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete match_statuses error: {}", e)))?;
                // works (from, to)
                works::Entity::delete_many().filter(Condition::any().add(works::Column::From.eq(uid)).add(works::Column::To.eq(uid))).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete works error: {}", e)))?;
                // belongs (corp_id, usr_id)
                belongs::Entity::delete_many().filter(Condition::any().add(belongs::Column::CorpId.eq(uid)).add(belongs::Column::UsrId.eq(uid))).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete belongs error: {}", e)))?;
                // usr_badges (corp_id, from, to)
                usr_badges::Entity::delete_many().filter(Condition::any().add(usr_badges::Column::CorpId.eq(uid)).add(usr_badges::Column::From.eq(uid)).add(usr_badges::Column::To.eq(uid))).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete usr_badges error: {}", e)))?;
                // points (corp_id, from, to)
                points::Entity::delete_many().filter(Condition::any().add(points::Column::CorpId.eq(uid)).add(points::Column::From.eq(uid)).add(points::Column::To.eq(uid))).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete points error: {}", e)))?;
                // payments (corp_id)
                payments::Entity::delete_many().filter(payments::Column::CorpId.eq(uid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete payments error: {}", e)))?;
                // payouts (usr_id)
                payouts::Entity::delete_many().filter(payouts::Column::UsrId.eq(uid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete payouts error: {}", e)))?;
                // jobs (corp_id)
                jobs::Entity::delete_many().filter(jobs::Column::CorpId.eq(uid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete jobs error: {}", e)))?;
                // badges (corp_id)
                badges::Entity::delete_many().filter(badges::Column::CorpId.eq(uid)).exec(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete badges error: {}", e)))?;
            }
            model.delete(tx).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Delete user error: {}", e)))?;
            Ok(())
        })
    }).await?;
    // --------------------------------
    // 3. 最終レスポンス
    // --------------------------------
    Ok(DeleteUsrRes { id: target_usr_id })
}
// ============================================================
// Staff Management (Hire/Dehire)
// ============================================================
pub async fn hire_usr(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    target_usr_id: u32,
) -> Result<HireUsrRes, ApiError> {
    // 1. 権限チェックと対象ユーザーの取得 (VDRのパーティション内かつ is_staff=0)
    let model = find_usrs_base(ju, ids).await?
        .filter(usrs::Column::Id.eq(target_usr_id))
        .filter(usrs::Column::IsStaff.eq(0))
        .one(conn)
        .await
        .map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Fetch user error: {}", e)))?
        .ok_or_else(|| ApiError::new_system(StatusCode::NOT_FOUND, rterr::ERR_INVALID_REQUEST, "User not found or already a staff."))?;

    // 2. 更新
    let mut active = model.into_active_model();
    active.is_staff = Set(1);
    active.update(conn).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Update user staff status error: {}", e)))?;

    Ok(HireUsrRes { id: target_usr_id })
}

pub async fn dehire_usr(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    target_usr_id: u32,
) -> Result<DehireUsrRes, ApiError> {
    // 1. 権限チェックと対象ユーザーの取得 (VDRのパーティション内かつ is_staff=1)
    let model = find_usrs_base(ju, ids).await?
        .filter(usrs::Column::Id.eq(target_usr_id))
        .filter(usrs::Column::IsStaff.eq(1))
        .one(conn)
        .await
        .map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Fetch user error: {}", e)))?
        .ok_or_else(|| ApiError::new_system(StatusCode::NOT_FOUND, rterr::ERR_INVALID_REQUEST, "User not found or not a staff."))?;

    // 2. 更新
    let mut active = model.into_active_model();
    active.is_staff = Set(0);
    active.update(conn).await.map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Update user staff status error: {}", e)))?;

    Ok(DehireUsrRes { id: target_usr_id })
}
