use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::NamespacedIdError;

use super::validation::{validate_namespace, validate_path};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamespacedId {
    raw: String,
    namespace_len: usize,
}

impl NamespacedId {
    pub fn new(namespace: &str, path: &str) -> Result<Self, NamespacedIdError> {
        validate_namespace(namespace)?;
        validate_path(path)?;

        let raw = format!("{namespace}:{path}");
        Ok(Self {
            raw,
            namespace_len: namespace.len(),
        })
    }

    pub fn parse(value: &str) -> Result<Self, NamespacedIdError> {
        let count = value.chars().filter(|ch| *ch == ':').count();
        if count == 0 {
            return Err(NamespacedIdError::MissingSeparator {
                value: value.to_owned(),
            });
        }
        if count > 1 {
            return Err(NamespacedIdError::MultipleSeparators {
                value: value.to_owned(),
                count,
            });
        }

        let (namespace, path) = value.split_once(':').expect("count checked above");
        if namespace.is_empty() {
            return Err(NamespacedIdError::EmptyNamespace {
                value: value.to_owned(),
            });
        }
        if path.is_empty() {
            return Err(NamespacedIdError::EmptyPath {
                value: value.to_owned(),
            });
        }

        validate_namespace(namespace)?;
        validate_path(path)?;

        Ok(Self {
            raw: value.to_owned(),
            namespace_len: namespace.len(),
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.raw[..self.namespace_len]
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.raw[self.namespace_len + 1..]
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.raw
    }
}

impl Display for NamespacedId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for NamespacedId {
    type Err = NamespacedIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for NamespacedId {
    type Error = NamespacedIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl Serialize for NamespacedId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for NamespacedId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}
