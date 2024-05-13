use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use serde_json::Value;

use crate::app_config::{Config, ListHandling};
use crate::props::PropertyConstructionError::{TopLevelArrayError, TopLevelPrimitiveError};

pub struct Properties {
  props: BTreeMap<String, String>,
}

#[derive(Debug)]
pub enum PropertyConstructionError {
  TopLevelPrimitiveError(Value),
  TopLevelArrayError(Value),
}

impl Display for PropertyConstructionError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      TopLevelPrimitiveError(v) => write!(
        f, "JSON value is a primitive, which cannot be formatted as properties: {}",
        v.to_string()),
      TopLevelArrayError(_) => write!(
        f, "JSON value is an array, which cannot be formatted as properties.\n\
        Break up the JSON into individual objects and convert them separately!"),
    }
  }
}

impl Error for PropertyConstructionError {}

impl Properties {
  pub fn try_from(value: Value, config: &Config) -> Result<Self, PropertyConstructionError> {
    PropertiesBuilder(config).build(value)
  }

  fn empty() -> Self {
    Properties {
      props: BTreeMap::new()
    }
  }
}

impl Properties {
  pub fn export(self, path: &Path) -> Result<(), anyhow::Error> {
    let f = fs::File::create(path)?;
    let mut w = BufWriter::new(f);
    for (k, v) in self.props {
      write!(w, "{k}={v}\n")?;
    }
    w.flush()?;
    Ok(())
  }
}

struct PropertiesBuilder<'a>(&'a Config);

impl<'a> PropertiesBuilder<'_> {
  fn build(&'a self, value: Value) -> Result<Properties, PropertyConstructionError> {
    match value {
      Value::Object(object_map) => Ok(self.parse_internal(object_map)),
      Value::Null => Ok(Properties::empty()),
      Value::String(_) | Value::Bool(_) | Value::Number(_) => Err(TopLevelPrimitiveError(value)),
      Value::Array(_) => Err(TopLevelArrayError(value)),
    }
  }

  fn parse_internal(&self, object_map: serde_json::Map<String, Value>) -> Properties {
    let props: BTreeMap<String, String> = object_map.into_iter()
      .flat_map(|(s, v)| self.parse_value(&s, v).into_iter())
      .collect();
    Properties { props }
  }

  fn parse_value(&self, namespace: &str, value: Value) -> Vec<(String, String)> {
    match value {
      Value::Null => vec![(String::from(namespace), String::from(""))],
      Value::Number(n) => vec![(String::from(namespace), n.to_string())],
      Value::String(s) => vec![(String::from(namespace), s)],
      Value::Bool(b) => vec![(String::from(namespace), b.to_string())],
      Value::Object(object_map) => object_map.into_iter()
        .flat_map(|(s, v)| {
          let inner_namespace = Self::concat_namespace(namespace, s);
          self.parse_value(&inner_namespace, v)
        })
        .collect::<Vec<(String, String)>>(),
      Value::Array(values) => match self.0.list_handling() {
        ListHandling::SingleProp => if Self::has_only_primitives(&values) {
          let list_val = values.iter().map(|it| it.to_string()).fold(String::new(), |mut a, b| {
            a.push_str(&b);
            a
          });
          vec![(String::from(namespace), list_val)]
        } else {
          vec![]
        },
        ListHandling::MultiProp => values.into_iter().enumerate()
          .flat_map(|(i, v)| {
            let inner_namespace = Self::concat_namespace(namespace, i.to_string());
            self.parse_value(&inner_namespace, v)
          })
          .collect::<Vec<(String, String)>>(),
      },
    }
  }

  fn concat_namespace(namespace: &str, sub_key: String) -> String {
    let mut inner_namespace = String::from(namespace);
    inner_namespace.push('.');
    inner_namespace.push_str(&sub_key);
    inner_namespace
  }

  fn has_only_primitives(values: &[Value]) -> bool {
    values.iter().all(|v| match v {
      Value::Array { .. } | Value::Object { .. } => false,
      _ => true
    })
  }
}

#[cfg(test)]
mod tests {
  use std::path::{Path, PathBuf};

  use crate::app_config::Config;
  use crate::app_config::ListHandling::MultiProp;
  use crate::props::Properties;

  fn assert_key_has_value(prop: &Properties, key: &str, expected: &str) {
    let actual = prop.props.get(key).expect(&format!("key {key} is present"));
    assert_eq!(actual, expected);
  }

  #[test]
  fn foo_1() {
    let config = Config::empty();
    let value = serde_json::json!({
        "a" : "a value",
        "b" : "b value",
        "c" : false
    });
    let prop = Properties::try_from(value, &config).expect("JSON is parsed");
    assert_key_has_value(&prop, "a", "a value");
    assert_key_has_value(&prop, "b", "b value");
    assert_key_has_value(&prop, "c", "false");
  }

  #[test]
  fn foo_2() {
    let config = Config::empty();
    let value = serde_json::json!({
      "a" : "a value",
      "b" : {
        "foo" : 123,
        "bar" : "bar val",
        "baz" : false
      },
      "c" : {
        "foo" : 999
      }
    });
    let prop = Properties::try_from(value, &config).expect("JSON is parsed");
    assert_eq!(prop.props.len(), 5);
    assert_key_has_value(&prop, "a", "a value");
    assert_key_has_value(&prop, "b.foo", "123");
    assert_key_has_value(&prop, "b.bar", "bar val");
    assert_key_has_value(&prop, "b.baz", "false");
    assert_key_has_value(&prop, "c.foo", "999");
  }
}
