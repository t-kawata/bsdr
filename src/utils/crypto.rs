use bcrypt::{hash, verify};
use anyhow::{Context, Result};

pub fn get_hash_with_cost(bd: &str, cost: u32) -> Result<String> {
    if bd.is_empty() {
        anyhow::bail!("BD is empty.");
    }
    hash(bd, cost).context("Failed to generate hash.")
}

/// ハッシュ検証関数
/// 入力された平文 `bd` と `hashed` が一致するか検証する
pub fn verify_hash(bd: &str, hashed: &str) -> Result<bool> {
    if bd.is_empty() || hashed.is_empty() {
        return Ok(false);
    }
    // verify は平文とハッシュを受け取り、Result<bool, BcryptError> を返す
    verify(bd, hashed).context("Failed to verify hash.")
}