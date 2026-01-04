use crate::utils::db::get_db;
use crate::utils::init::{CommonFlgs, HasCommonFlgs, init};
use clap::Parser;
use serde::Serialize;
use std::iter::{Chain, Cloned, Once};
use std::slice::Iter;
use crate::migration::{Migrator, MigratorTrait};

#[derive(Debug, Parser, Serialize)]
#[command(override_usage = "bsdr am [OPTIONS]")]
pub struct AMFlgs {
    #[command(flatten)]
    pub common: CommonFlgs,
}

impl HasCommonFlgs for AMFlgs {
    fn common_flgs(&self) -> &CommonFlgs {
        &self.common
    }
}

pub async fn main_of_am(args: Chain<Once<String>, Cloned<Iter<'_, String>>>) {
    // ==============================
    // 初期化
    // ==============================
    let (flgs, env) = init::<AMFlgs>(args).expect("Failed to init am mode.");

    // ==============================
    // フラグの出力
    // ==============================
    let flgs_json = serde_json::to_string(&flgs).expect("Failed to serialize flgs to json.");
    log::debug!("AM-FLAGS: {}", flgs_json);

    // ==============================
    // DB接続
    // ==============================
    let db_result = get_db(&env).await;
    let db = match db_result {
        Ok(db) => { log::debug!("DB created successfully."); db }
        Err(e) => { eprintln!("Failed to create DB: {}", e); std::process::exit(1); }
    };

    // ==============================
    // AutoMigration の実行
    // ==============================
    log::info!("Running AutoMigration...");
    let rw_conn = db.get_rw().expect("Failed to get RW connection for migration.");
    Migrator::up(rw_conn, None).await.expect("Failed to run migrations.");
    log::info!("AutoMigration completed successfully.");
}
