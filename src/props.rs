use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{LineWriter, Write};
use std::path::Path;

use serde_json::Value;

pub struct Properties {
  props: HashMap<String, String>,
}

#[derive(Debug)]
pub struct PropertyConstructionError;

impl Display for PropertyConstructionError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    todo!()
  }
}

impl Error for PropertyConstructionError {}

impl TryFrom<Value> for Properties {
  type Error = anyhow::Error;

  fn try_from(value: Value) -> Result<Self, Self::Error> {
    match value {
      Value::Object(object_map) => Ok(Self::parse_internal(object_map)),
      Value::Null => Ok(Self::empty()),
      _ => Err(anyhow::Error::new(PropertyConstructionError)),
    }
  }
}

impl Properties {
  fn parse_internal(object_map: serde_json::Map<String, Value>) -> Self {
    let props: HashMap<String, String> = object_map.into_iter()
      .flat_map(|(s, v)| Self::parse_value(&s, v).into_iter())
      .collect();
    Properties { props }
  }

  fn parse_value(namespace: &str, value: Value) -> Vec<(String, String)> {
    match value {
      Value::Null => vec![(String::from(namespace), String::from(""))],
      Value::Number(n) => vec![(String::from(namespace), n.to_string())],
      Value::String(s) => vec![(String::from(namespace), s)],
      Value::Bool(b) => vec![(String::from(namespace), b.to_string())],
      Value::Object(object_map) => object_map.into_iter()
        .flat_map(|(s, v)| {
          let inner_namespace = Self::concat_namespace(namespace, s);
          Self::parse_value(&inner_namespace, v)
        })
        .collect::<Vec<(String, String)>>(),
      Value::Array(values) => values.into_iter().enumerate()
        .flat_map(|(i, v)| {
          let inner_namespace = Self::concat_namespace(namespace, i.to_string());
          Self::parse_value(&inner_namespace, v)
        })
        .collect::<Vec<(String, String)>>(),
    }
  }

  fn concat_namespace(namespace: &str, sub_key: String) -> String {
    let mut inner_namespace = String::from(namespace);
    inner_namespace.push('.');
    inner_namespace.push_str(&sub_key);
    inner_namespace
  }

  fn empty() -> Self {
    Properties {
      props: HashMap::new()
    }
  }
}

#[derive(Debug)]
pub struct PropertyExportErr;

impl Display for PropertyExportErr {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    todo!()
  }
}

impl Error for PropertyExportErr {}

impl Properties {
  pub fn export(self, path: &Path) -> Result<(), anyhow::Error> {
    let f = fs::OpenOptions::new().create(true).append(true).open(path)?;
    let mut f = LineWriter::new(f);
    for (a, b) in self.props {
      println!("{a}={b}");
      write!(f, "{a}={b}")?;
    }
    Ok::<(), anyhow::Error>(())
  }
}

#[cfg(test)]
mod tests {
  use crate::props::Properties;

  fn assert_key_has_value(prop: &Properties, key: &str, expected: &str) {
    let actual = prop.props.get(key).expect(&format!("key {key} is present"));
    assert_eq!(actual, expected);
  }

  #[test]
  fn foo_1() {
    let value = serde_json::json!({
        "a" : "a value",
        "b" : "b value",
        "c" : false
    });
    let prop = Properties::try_from(value).expect("JSON is parsed");
    assert_key_has_value(&prop, "a", "a value");
    assert_key_has_value(&prop, "b", "b value");
    assert_key_has_value(&prop, "c", "false");

  }

  #[test]
  fn foo_2() {
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
    let prop = Properties::try_from(value).expect("JSON is parsed");
    assert_eq!(prop.props.len(), 5);
    assert_key_has_value(&prop, "a", "a value");
    assert_key_has_value(&prop, "b.foo", "123");
    assert_key_has_value(&prop, "b.bar", "bar val");
    assert_key_has_value(&prop, "b.baz", "false");
    assert_key_has_value(&prop, "c.foo", "999");
  }
}
