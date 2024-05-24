use std::fs;
use std::time::SystemTime;

use clap::Parser;
use log::debug;
use serde_json::Value;

use crate::app_config::Config;
use crate::props::Properties;

mod app_config;
mod props;

const STR_EMPTY: &str = "";
const STR_COMMA: &str = ",";

fn main() -> anyhow::Result<()> {
  let config: Config = parse_config()?;
  setup_logger(&config)?;
  debug!("Logger initialised: Configuration is: {:?}", &config);
  parse_json(&config)
    .and_then(|json| Properties::create(json, &config).map_err(anyhow::Error::new))
    .and_then(|prop| match config.dest() {
      Some(p) => { prop.export(p, &config) }
      None => { prop.print(&config) }
    })
}

fn parse_config() -> anyhow::Result<Config> {
  Config::parse().validate().map_err(anyhow::Error::new)
}

fn setup_logger(config: &Config) -> Result<(), fern::InitError> {
  let level_filter = if config.debug { log::LevelFilter::Debug } else { log::LevelFilter::Info };
  println!("{level_filter:?}");
  fern::Dispatch::new()
    .format(|out, message, record| {
      out.finish(format_args!(
        "[{} {} {}] {}",
        humantime::format_rfc3339_seconds(SystemTime::now()),
        record.level(),
        record.target(),
        message
      ))
    })
    .level(level_filter)
    .chain(std::io::stdout())
    .chain(fern::log_file("output.log")?)
    .apply()?;
  Ok(())
}

fn parse_json(config: &Config) -> anyhow::Result<Value> {
  let s = fs::read_to_string(config.source())?;
  serde_json::from_str(&s).map_err(anyhow::Error::new)
}
