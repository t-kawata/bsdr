use chrono::{Utc, TimeDelta};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, prelude::Expr, ColumnTrait};
use crate::entities::usrs;
use crate::vo::usrs_vo::AuthUsrVo;
use crate::utils::crypto::verify_hash;
use crate::utils::bd::is_valid_bd;
use anyhow::{Result, anyhow};

pub struct JwtConfig {
    pub skey: String,
}

/// JWTのペイロード（Claims）構造体
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub apx_id: Option<u32>,
    pub vdr_id: Option<u32>,
    pub usr_id: Option<u32>,
    pub email: String,
    #[serde(rename = "type")]
    pub usr_type: u8,
    pub is_staff: bool,
    pub exp: i64, // Unixタイムスタンプ
}

pub struct JwtUsr {
    pub apx_id: Option<u32>,
    pub vdr_id: Option<u32>,
    pub usr_id: Option<u32>,
    pub staff_id: Option<u32>,
    pub email: String,
    pub usr_type: u8,
}

pub fn is_apx(aid: &u32, vid: &u32) -> bool {
    *aid == 0 && *vid == 0
}

pub fn is_vdr(aid: &u32, vid: &u32) -> bool {
    *aid > 0 && *vid == 0
}

pub fn is_usr(aid: &u32, vid: &u32) -> bool {
    *aid > 0 && *vid > 0
}

pub fn generate_token(
    skey: &str,
    aid: Option<u32>,
    vid: Option<u32>,
    uid: Option<u32>,
    is_staff: bool,
    usr_type: u8,
    email: String,
    expire: u32,
) -> Result<String, jsonwebtoken::errors::Error> {
    let staff_id = if is_staff { uid } else { Some(0) };
    let jwt_usr = JwtUsr {
        apx_id: aid,
        vdr_id: vid,
        usr_id: uid,
        staff_id,
        email,
        usr_type,
    };
    generate_token_base(skey, expire, &jwt_usr)
}

pub fn generate_token_for_bd(skey: &str, expire: u32) -> Result<String, jsonwebtoken::errors::Error> {
    generate_token(skey, Some(0), Some(0), Some(0), false, 0, "bd@bd.com".to_string(), expire)
}

pub fn generate_token_for_apx(skey: &str, uid: u32, email: String, expire: u32) -> Result<String, jsonwebtoken::errors::Error> {
    generate_token(skey, Some(0), Some(0), Some(uid), false, 0, email, expire)
}

pub fn generate_token_for_vdr(skey: &str, aid: u32, uid: u32, email: String, expire: u32) -> Result<String, jsonwebtoken::errors::Error> {
    generate_token(skey, Some(aid), Some(0), Some(uid), false, 0, email, expire)
}

pub fn generate_token_for_usr(skey: &str, aid: u32, vid: u32, uid: u32, email: String, expire: u32) -> Result<String, jsonwebtoken::errors::Error> {
    generate_token(skey, Some(aid), Some(vid), Some(uid), false, 0, email, expire)
}

// ------------------------------------------------------------
// Authentication Logic
// ------------------------------------------------------------

pub async fn auth_bd(conn: &DatabaseConnection, x_bd: &str, skey: &str, expire: u32) -> Result<String> {
    let is_valid = is_valid_bd(conn, x_bd.to_string())
        .await
        .map_err(|e| anyhow!("BD verification error: {}", e))?;
    if !is_valid {
        return Err(anyhow!("Invalid BD."));
    }
    generate_token_for_bd(skey, expire).map_err(|e| anyhow!("Token generation error: {}", e))
}

pub async fn auth_apx(conn: &DatabaseConnection, email: String, password: String, skey: &str, expire: u32) -> Result<String> {
    let result = usrs::Entity::find()
        .select_only()
        .column(usrs::Column::Id)
        .column(usrs::Column::Email)
        .column(usrs::Column::Password)
        .filter(usrs::Column::ApxId.is_null())
        .filter(usrs::Column::VdrId.is_null())
        .filter(usrs::Column::Email.eq(email))
        .filter(Expr::col(usrs::Column::BgnAt).lte(Expr::current_timestamp()))
        .filter(Expr::col(usrs::Column::EndAt).gte(Expr::current_timestamp()))
        .into_model::<AuthUsrVo>()
        .one(conn)
        .await
        .map_err(|e| anyhow!("Failed to fetch APX user: {}", e))?;
    let usr = result.ok_or_else(|| anyhow!("Invalid email or password for APX."))?;
    let is_valid = verify_hash(&password, &usr.password)
        .map_err(|e| anyhow!("Password verification error: {}", e))?;
    if !is_valid {
        return Err(anyhow!("Invalid email or password for APX."));
    }
    generate_token_for_apx(skey, usr.id as u32, usr.email, expire).map_err(|e| anyhow!("APX token generation error: {}", e))
}

pub async fn auth_vdr(conn: &DatabaseConnection, apx_id: u32, email: String, password: String, skey: &str, expire: u32) -> Result<String> {
    let result = usrs::Entity::find()
        .select_only()
        .column(usrs::Column::Id)
        .column(usrs::Column::Email)
        .column(usrs::Column::Password)
        .filter(usrs::Column::ApxId.eq(apx_id))
        .filter(usrs::Column::VdrId.is_null())
        .filter(usrs::Column::Email.eq(email))
        .filter(Expr::col(usrs::Column::BgnAt).lte(Expr::current_timestamp()))
        .filter(Expr::col(usrs::Column::EndAt).gte(Expr::current_timestamp()))
        .into_model::<AuthUsrVo>()
        .one(conn)
        .await
        .map_err(|e| anyhow!("Failed to fetch VDR user: {}", e))?;
    let usr = result.ok_or_else(|| anyhow!("Invalid email or password for VDR."))?;
    let is_valid = verify_hash(&password, &usr.password)
        .map_err(|e| anyhow!("Password verification error: {}", e))?;
    if !is_valid {
        return Err(anyhow!("Invalid email or password for VDR."));
    }
    generate_token_for_vdr(skey, apx_id, usr.id as u32, usr.email, expire).map_err(|e| anyhow!("VDR token generation error: {}", e))
}

pub async fn auth_usr(conn: &DatabaseConnection, apx_id: u32, vdr_id: u32, email: String, password: String, skey: &str, expire: u32) -> Result<String> {
    let result = usrs::Entity::find()
        .select_only()
        .column(usrs::Column::Id)
        .column(usrs::Column::Email)
        .column(usrs::Column::Password)
        .filter(usrs::Column::ApxId.eq(apx_id))
        .filter(usrs::Column::VdrId.eq(vdr_id))
        .filter(usrs::Column::Email.eq(email))
        .filter(Expr::col(usrs::Column::BgnAt).lte(Expr::current_timestamp()))
        .filter(Expr::col(usrs::Column::EndAt).gte(Expr::current_timestamp()))
        .into_model::<AuthUsrVo>()
        .one(conn)
        .await
        .map_err(|e| anyhow!("Failed to fetch USR user: {}", e))?;
    let usr = result.ok_or_else(|| anyhow!("Invalid email or password for USR."))?;
    let is_valid = verify_hash(&password, &usr.password)
        .map_err(|e| anyhow!("Password verification error: {}", e))?;
    if !is_valid {
        return Err(anyhow!("Invalid email or password for USR."));
    }
    generate_token_for_usr(skey, apx_id, vdr_id, usr.id as u32, usr.email, expire).map_err(|e| anyhow!("USR token generation error: {}", e))
}

fn generate_token_base(
    skey: &str,
    life_time: u32,
    u: &JwtUsr,
) -> Result<String, jsonwebtoken::errors::Error> {
    // 有効期限の計算 (現在時刻 + life_time 時間)
    let exp = Utc::now()
        .checked_add_signed(TimeDelta::hours(life_time as i64))
        .expect("valid timestamp")
        .timestamp();
    let claims = Claims {
        apx_id: u.apx_id,
        vdr_id: u.vdr_id,
        usr_id: u.usr_id,
        email: u.email.clone(),
        usr_type: u.usr_type,
        is_staff: u.staff_id.map(|id| id > 0).unwrap_or(false),
        exp,
    };
    // HS256アルゴリズムでエンコード
    let header = Header::new(Algorithm::HS256);
    encode(&header, &claims, &EncodingKey::from_secret(skey.as_bytes()))
}

