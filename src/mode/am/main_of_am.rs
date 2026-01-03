use crate::utils::init::{CommonFlgs, HasCommonFlgs, init};
use clap::Parser;
use serde::Serialize;
use std::iter::{Chain, Cloned, Once};
use std::slice::Iter;

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

pub fn main_of_am(args: Chain<Once<String>, Cloned<Iter<'_, String>>>) {
    let (flgs, _env) = init::<AMFlgs>(args).expect("Failed to init am mode.");
    let flgs_json = serde_json::to_string(&flgs).expect("Failed to serialize flgs to json.");
    log::debug!("AM-FLAGS: {}", flgs_json);
}
