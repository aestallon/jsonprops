use std::fs;

use serde_json::Value;

use crate::app_config::Config;
use crate::props::Properties;

mod app_config;
mod props;

static STR_EMPTY: &'static str = "";

fn main() {
  let config: Config = Config::from_args(std::env::args()).expect("parsing arguments failed");
  let res = parse_json(&config)
    .and_then(|json| Properties::try_from(json, &config).map_err(anyhow::Error::new))
    .and_then(|prop| prop.export(&config.dest()));
  match res { 
    Err(e) => eprintln!("{e}"),
    _ => println!("Done!")
  }
}

fn parse_json(config: &Config) -> anyhow::Result<Value> {
  let s = fs::read_to_string(config.source())?;
  serde_json::from_str(&s).map_err(anyhow::Error::new)
}
