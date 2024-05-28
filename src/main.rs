use std::fs;
use std::time::SystemTime;

use clap::Parser;
use log::debug;
use serde_json::Value;

use crate::app_config::Config;
use crate::props::Properties;

mod app_config;
mod props;
mod str_constant;

fn main() -> anyhow::Result<()> {
  let config: Config = parse_config()?;
  setup_logger(&config)?;
  debug!("Logger initialised: Configuration is: {:?}", &config);
  if config.dest().is_none() {
    debug!("No destination file specified. Writing to standard output...");
  }

  parse_json(&config)
    .and_then(|json| Properties::create(json, &config))
    .and_then(|prop| prop.export(&config))
}

fn parse_config() -> anyhow::Result<Config> {
  Config::parse().validate().map_err(anyhow::Error::new)
}

fn setup_logger(config: &Config) -> Result<(), fern::InitError> {
  let level_filter = if config.debug { log::LevelFilter::Debug } else { log::LevelFilter::Info };
  let mut logger = fern::Dispatch::new()
    .format(|out, message, record| {
      out.finish(format_args!(
        "[{} {} {}] {}",
        humantime::format_rfc3339_seconds(SystemTime::now()),
        record.level(),
        record.target(),
        message
      ))
    })
    .level(level_filter);
  if config.debug || config.dest().is_some() {
    logger = logger.chain(std::io::stdout());
  }
  logger.chain(fern::log_file("output.log")?).apply()?;
  Ok(())
}

fn parse_json(config: &Config) -> anyhow::Result<Value> {
  let s = fs::read_to_string(config.source())?;
  serde_json::from_str(&s).map_err(anyhow::Error::new)
}
