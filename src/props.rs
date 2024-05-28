use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{BufWriter, Write};

use log::debug;
use serde_json::Value;

use crate::app_config::{Config, ListHandling};
use crate::props::prop_key::PropKey;
use crate::props::PropertyConstructionError::{TopLevelArrayError, TopLevelPrimitiveError};
use crate::str_constant;

pub struct Properties {
  props: BTreeMap<PropKey, String>,
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
        v),
      TopLevelArrayError(_) => write!(
        f, "JSON value is an array, which cannot be formatted as properties.\n\
        Break up the JSON into individual objects and convert them separately!"),
    }
  }
}

impl Error for PropertyConstructionError {}

impl Properties {
  pub fn create(value: Value, config: &Config) -> anyhow::Result<Self> {
    PropertiesBuilder(config).build(value).map_err(anyhow::Error::new)
  }

  fn empty() -> Self {
    Properties {
      props: BTreeMap::new()
    }
  }

  pub fn export(self, config: &Config) -> anyhow::Result<()> {
    let out = match config.dest() {
      None => Box::new(std::io::stdout()) as Box<dyn Write>,
      Some(p) => Box::new(File::create(p)?) as Box<dyn Write>,
    };
    let mut w = BufWriter::new(out);

    let sep = config.entry_separator();
    for (k, v) in self.props {
      writeln!(w, "{k}{sep}{v}")?;
    }
    w.flush().map_err(anyhow::Error::new)
  }
}

struct PropertiesBuilder<'a>(&'a Config);

impl PropertiesBuilder<'_> {
  fn build(&self, value: Value) -> Result<Properties, PropertyConstructionError> {
    match value {
      Value::Object(object_map) => Ok(self.parse_internal(object_map)),
      Value::Null => Ok(Properties::empty()),
      Value::String(_) | Value::Bool(_) | Value::Number(_) => Err(TopLevelPrimitiveError(value)),
      Value::Array(_) => Err(TopLevelArrayError(value)),
    }
  }

  fn parse_internal(&self, object_map: serde_json::Map<String, Value>) -> Properties {
    let props: BTreeMap<PropKey, String> = object_map.into_iter()
      .flat_map(|(s, v)| self.parse_value(&s, v).into_iter())
      .collect();
    Properties { props }
  }

  fn parse_value(&self, namespace: &str, value: Value) -> Vec<(PropKey, String)> {
    let key = PropKey::new(namespace);
    match value {
      Value::Null => vec![(key, String::from(""))],
      Value::Number(n) => vec![(key, n.to_string())],
      Value::String(s) => vec![(key, s.normalise(self.0.discard_wsp))],
      Value::Bool(b) => vec![(key, b.to_string())],
      Value::Object(object_map) => object_map.into_iter()
        .flat_map(|(s, v)| {
          let inner_namespace = Self::concat_namespace(namespace, &s);
          self.parse_value(&inner_namespace, v)
        })
        .collect(),
      Value::Array(values) => match self.0.list_handling() {
        ListHandling::SingleProp => if Self::has_only_primitives(&values) {
          let list_val = values.into_iter()
            .map(Self::primitive_to_string)
            .collect::<Vec<String>>()
            .join(str_constant::COMMA);
          vec![(key, list_val.normalise(self.0.discard_wsp))]
        } else {
          debug!(
            "{0} denotes a list, and its members are not exclusively primitives!\n\
            List handling is configured to run as [ single-prop ], thus key {0} shall be omitted.\n\
            The list values were: {1:?}",
            namespace, &values);
          vec![]
        },
        ListHandling::MultiProp => values.into_iter().enumerate()
          .flat_map(|(i, v)| {
            let inner_namespace = Self::concat_namespace(namespace, &i.to_string());
            self.parse_value(&inner_namespace, v)
          })
          .collect(),
      },
    }
  }

  fn concat_namespace(namespace: &str, sub_key: &str) -> String {
    let mut inner_namespace = String::with_capacity(namespace.len() + sub_key.len() + 1);
    inner_namespace.push_str(namespace);
    inner_namespace.push('.');
    inner_namespace.push_str(sub_key);
    inner_namespace
  }

  fn has_only_primitives(values: &[Value]) -> bool {
    values.iter().all(|v| !matches!(v, Value::Array { .. } | Value::Object { .. }))
  }

  fn primitive_to_string(value: Value) -> String {
    match value {
      Value::String(s) => s,
      Value::Bool { .. } | Value::Number { .. } | Value::Null => value.to_string(),
      _ => unreachable!()
    }
  }
}

/// .properties file behaviour
///
/// Trailing whitespace is always significant.
///
/// A leading whitespace is dropped because the following formats should yield the same result:
/// ```properties
/// key=val
/// key = val
/// ```
/// (There are optional whitespaces around the entry separator.)
/// To preserve the whitespace, it must be escaped with a backslash:
/// ```properties
/// key=\     val
/// ```
///
trait WhiteSpaceNormalised {
  /// Normalises a value to abide by the `.properties` file leading whitespace rules.
  fn normalise(self, discard_wsp: bool) -> Self;
}

impl WhiteSpaceNormalised for String {
  /// Normalises a [String] to abide by the `.properties` file leading whitespace rules.
  ///
  /// If the provided argument is `true` the provided [String] is trimmed of leading whitespace if
  /// necessary.
  /// - `"foo"` and `"    foo"` will both be rendered as `"foo"`
  ///
  /// If the provided argument is `false`, a leading backslash is inserted if necessary to preserve
  /// the leading whitespace:
  /// - `"bar"` will be left unchanged
  /// - `"    bar"` will be rendered as `"\    bar"`
  fn normalise(self, discard_wsp: bool) -> Self {
    let starts_with_wsp = match self.chars().next() {
      Some(c) => c.is_whitespace(),
      _ => false,
    };

    if starts_with_wsp && discard_wsp {
      self.trim_start().into()
    } else if starts_with_wsp {
      let mut ret = String::with_capacity(self.len() + 1);
      ret.push('\\');
      ret.push_str(&self);
      ret
    } else {
      self
    }
  }
}

mod prop_key {
  use std::fmt::{Display, Formatter};

  #[derive(PartialEq, PartialOrd, Eq, Ord)]
  pub(super) struct PropKey(String);

  impl PropKey {
    pub(super) fn new(s: &str) -> Self {
      // if the string starts with '#', we need to escape it. If it doesn't there is no need (only 
      // line commencing '#' would signal a comment line).
      // There is a possibility the string starts with leading whitespace and the first
      // non-whitespace character is a '#' => the escaping loop later accounts for that: escaping
      // " #foo" as "\ #foo" is sufficient.
      let mut inner = if let Some('#') = s.chars().next() {
        let mut temp = String::with_capacity(s.len() + 1);
        temp.push('\\');
        temp
      } else {
        String::with_capacity(s.len())
      };
      for c in s.chars() {
        if c == ' ' || c == ':' || c == '=' {
          inner.extend(&['\\', c]);
        } else {
          inner.push(c);
        }
      }

      PropKey(inner)
    }
  }

  impl Display for PropKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
      f.write_str(&(self.0))
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::app_config::Config;
  use crate::props::prop_key::PropKey;
  use crate::props::Properties;

  fn assert_key_has_value(prop: &Properties, key: &str, expected: &str) {
    let k = PropKey::new(key);
    let actual = prop.props.get(&k).unwrap_or_else(|| panic!("key {key} is present"));
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
    let prop = Properties::create(value, &config).expect("JSON is parsed");
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
    let prop = Properties::create(value, &config).expect("JSON is parsed");
    assert_eq!(prop.props.len(), 5);
    assert_key_has_value(&prop, "a", "a value");
    assert_key_has_value(&prop, "b.foo", "123");
    assert_key_has_value(&prop, "b.bar", "bar val");
    assert_key_has_value(&prop, "b.baz", "false");
    assert_key_has_value(&prop, "c.foo", "999");
  }

  #[test]
  fn creating_prop_key_with_a_simple_string_leaves_the_string_unchanged() {
    let k = PropKey::new("foo");
    assert_eq!(format!("{k}"), "foo");
  }

  #[test]
  fn creating_prop_key_with_colon_has_the_colon_escaped() {
    let k = PropKey::new("fo:o");
    assert_eq!(format!("{k}"), "fo\\:o");
  }
  
  #[test]
  fn creating_prop_key_with_leading_number_sign_escapes_the_first_character() {
    let k = PropKey::new("#foo");
    assert_eq!(format!("{k}"), "\\#foo");
  }
  
  #[test]
  fn creating_prop_key_with_leading_wsp_and_number_sign_escapes_the_wsp_only() {
    let k = PropKey::new("  #foo");
    assert_eq!(format!("{k}"), "\\ \\ #foo");
  }
}
