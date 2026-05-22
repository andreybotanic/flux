use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::NamespacedIdError;

use super::NamespacedId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrototypeId(NamespacedId);

impl PrototypeId {
    #[must_use]
    pub fn new(value: NamespacedId) -> Self {
        Self(value)
    }

    pub fn parse(value: &str) -> Result<Self, NamespacedIdError> {
        Ok(Self(NamespacedId::parse(value)?))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[must_use]
    pub fn namespace(&self) -> &str {
        self.0.namespace()
    }

    #[must_use]
    pub fn path(&self) -> &str {
        self.0.path()
    }

    #[must_use]
    pub fn as_namespaced_id(&self) -> &NamespacedId {
        &self.0
    }

    #[must_use]
    pub fn into_namespaced_id(self) -> NamespacedId {
        self.0
    }
}

impl Display for PrototypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PrototypeId {
    type Err = NamespacedIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for PrototypeId {
    type Error = NamespacedIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<NamespacedId> for PrototypeId {
    fn from(value: NamespacedId) -> Self {
        Self(value)
    }
}

impl From<PrototypeId> for NamespacedId {
    fn from(value: PrototypeId) -> Self {
        value.0
    }
}

impl Serialize for PrototypeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PrototypeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}
