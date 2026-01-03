use crate::config::settings::{Env, get_env};
use chrono::{FixedOffset, Utc};
use clap::{Args, Parser};
use fern::Dispatch;
use log::LevelFilter;
use serde::Serialize;
use std::iter::{Chain, Cloned, Once};
use std::slice::Iter;

#[derive(Debug, Args, Serialize, Clone)]
pub struct CommonFlgs {
    #[arg(short = 'e', long = "env", default_value_t = String::from("local"), help = "Environment")]
    pub env: String,
    #[arg(short = 'l', long = "log_level", default_value_t = String::from("debug"), help = "Log level")]
    pub log_level: String,
    #[arg(short = 'o', long = "output", default_value_t = String::from("stdout"), help = "Destination of log output (stdout, /path/to/file).")]
    pub output: String,
}

pub trait HasCommonFlgs {
    fn common_flgs(&self) -> &CommonFlgs;
}

pub fn init<T>(
    args: Chain<Once<String>, Cloned<Iter<'_, String>>>,
) -> Result<(T, Env), Box<dyn std::error::Error>>
where
    T: Parser + HasCommonFlgs,
{
    let flgs = T::parse_from(args);
    let common = flgs.common_flgs();

    let env = get_env(&common.env);
    if env.empty {
        return Err(format!("Invalid environment: {}", common.env).into());
    }

    let level = match common.log_level.to_lowercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Debug,
    };

    let mut log_dispatch_base = Dispatch::new().level(level).format(|out, message, record| {
        let jst = Utc::now().with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap());
        out.finish(format_args!(
            "{} [{}] <{}> {}",
            jst.format("%Y-%m-%d_%H:%M:%S"),
            record.target(),
            record.level(),
            message,
        ))
    });

    match common.output.as_str() {
        "stdout" => {
            log_dispatch_base = log_dispatch_base.chain(std::io::stdout());
        }
        path => {
            log_dispatch_base = log_dispatch_base.chain(fern::log_file(path)?);
        }
    }

    log_dispatch_base.apply()?;
    Ok((flgs, env))
}
