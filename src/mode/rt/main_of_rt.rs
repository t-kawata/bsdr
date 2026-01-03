use crate::utils::env::get_env_or;
use crate::utils::init::{CommonFlgs, HasCommonFlgs, init};
use clap::Parser;
use serde::Serialize;
use std::iter::{Chain, Cloned, Once};
use std::slice::Iter;

#[derive(Debug, Parser, Serialize)]
#[command(override_usage = "bsdr rt [OPTIONS]")]
pub struct RTFlgs {
    #[command(flatten)]
    pub common: CommonFlgs,

    #[arg(short = 'd', long = "dotenv", default_value_t = String::from(".env"), help = "Path to .env file")]
    pub dotenv: String,
}

impl HasCommonFlgs for RTFlgs {
    fn common_flgs(&self) -> &CommonFlgs {
        &self.common
    }
}

pub fn main_of_rt(args: Chain<Once<String>, Cloned<Iter<'_, String>>>) {
    // ==============================
    // 初期化
    // ==============================
    let (flgs, _env) = init::<RTFlgs>(args).expect("Failed to init rt mode.");

    // ==============================
    // .envファイルの読み込み
    // ==============================
    dotenvy::from_path(&flgs.dotenv).expect(&format!("Failed to load .env from {}", flgs.dotenv));
    log::debug!("Loaded .env from: {}", flgs.dotenv);

    // ==============================
    // フラグの出力
    // ==============================
    let flgs_json = serde_json::to_string(&flgs).expect("Failed to serialize flgs to json.");
    log::debug!("RT-FLAGS: {}", flgs_json);

    let test = get_env_or("TEST", false);
    log::debug!("TEST: {}", test);
}
