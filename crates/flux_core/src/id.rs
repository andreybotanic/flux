use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{ModIdError, NamespacedIdError};

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModId {
    value: String,
}

impl ModId {
    pub fn new(value: &str) -> Result<Self, ModIdError> {
        validate_mod_id(value)?;
        Ok(Self {
            value: value.to_owned(),
        })
    }

    pub fn parse(value: &str) -> Result<Self, ModIdError> {
        Self::new(value)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.value
    }
}

impl Display for ModId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ModId {
    type Err = ModIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for ModId {
    type Error = ModIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl Serialize for ModId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ModId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

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

fn validate_namespace(namespace: &str) -> Result<(), NamespacedIdError> {
    if namespace.is_empty() {
        return Err(NamespacedIdError::EmptyNamespace {
            value: String::new(),
        });
    }

    for (position, ch) in namespace.chars().enumerate() {
        if position == 0 && !ch.is_ascii_lowercase() {
            return Err(NamespacedIdError::InvalidNamespaceCharacter {
                value: namespace.to_owned(),
                position,
                character: ch,
                reason: namespace_start_reason(ch),
            });
        }

        if !is_namespace_char(ch) {
            return Err(NamespacedIdError::InvalidNamespaceCharacter {
                value: namespace.to_owned(),
                position,
                character: ch,
                reason: namespace_char_reason(ch),
            });
        }
    }

    Ok(())
}

fn validate_path(path: &str) -> Result<(), NamespacedIdError> {
    if path.is_empty() {
        return Err(NamespacedIdError::EmptyPath {
            value: String::new(),
        });
    }

    for (segment_index, segment) in path.split('/').enumerate() {
        if segment.is_empty() {
            return Err(NamespacedIdError::EmptyPathSegment {
                value: path.to_owned(),
                segment_index,
            });
        }

        for (position, ch) in segment.chars().enumerate() {
            if position == 0 && !ch.is_ascii_lowercase() {
                return Err(NamespacedIdError::InvalidPathSegmentStart {
                    value: path.to_owned(),
                    segment_index,
                    character: ch,
                    reason: path_segment_start_reason(ch),
                });
            }

            if !is_path_char(ch) {
                return Err(NamespacedIdError::InvalidPathCharacter {
                    value: path.to_owned(),
                    segment_index,
                    position,
                    character: ch,
                    reason: path_char_reason(ch),
                });
            }
        }
    }

    Ok(())
}

fn validate_mod_id(value: &str) -> Result<(), ModIdError> {
    if value.is_empty() {
        return Err(ModIdError::Empty);
    }

    for (position, ch) in value.chars().enumerate() {
        if position == 0 && !ch.is_ascii_lowercase() {
            return Err(ModIdError::InvalidFirstCharacter {
                value: value.to_owned(),
                character: ch,
                reason: namespace_start_reason(ch),
            });
        }

        if !is_namespace_char(ch) {
            return Err(ModIdError::InvalidCharacter {
                value: value.to_owned(),
                position,
                character: ch,
                reason: namespace_char_reason(ch),
            });
        }
    }

    Ok(())
}

fn is_namespace_char(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'
}

fn is_path_char(ch: char) -> bool {
    is_namespace_char(ch)
}

fn namespace_start_reason(ch: char) -> String {
    if ch.is_ascii_uppercase() {
        return "uppercase letters are not allowed".to_owned();
    }
    if ch.is_ascii_digit() {
        return "must start with an ASCII lowercase letter".to_owned();
    }
    if ch == '_' {
        return "must start with an ASCII lowercase letter".to_owned();
    }
    if ch == '-' {
        return "hyphen is not allowed".to_owned();
    }
    if ch == '.' {
        return "dot is not allowed".to_owned();
    }
    if ch == '/' || ch == '\\' {
        return "slash characters are not allowed".to_owned();
    }
    if ch.is_whitespace() {
        return "whitespace is not allowed".to_owned();
    }
    format!("invalid character `{ch}`")
}

fn namespace_char_reason(ch: char) -> String {
    if ch.is_ascii_uppercase() {
        return "uppercase letters are not allowed".to_owned();
    }
    if ch == '-' {
        return "hyphen is not allowed".to_owned();
    }
    if ch == '.' {
        return "dot is not allowed".to_owned();
    }
    if ch == '/' || ch == '\\' {
        return "slash characters are not allowed".to_owned();
    }
    if ch == ':' {
        return "colon is not allowed inside namespace".to_owned();
    }
    if ch.is_whitespace() {
        return "whitespace is not allowed".to_owned();
    }
    format!("invalid character `{ch}`")
}

fn path_segment_start_reason(ch: char) -> String {
    if ch.is_ascii_uppercase() {
        return "uppercase letters are not allowed".to_owned();
    }
    if ch.is_ascii_digit() {
        return "path segment must start with an ASCII lowercase letter".to_owned();
    }
    if ch == '_' {
        return "path segment must start with an ASCII lowercase letter".to_owned();
    }
    if ch == '-' {
        return "hyphen is not allowed".to_owned();
    }
    if ch == '.' {
        return "dot is not allowed".to_owned();
    }
    if ch == '\\' {
        return "backslash is not allowed".to_owned();
    }
    if ch.is_whitespace() {
        return "whitespace is not allowed".to_owned();
    }
    format!("invalid character `{ch}`")
}

fn path_char_reason(ch: char) -> String {
    if ch.is_ascii_uppercase() {
        return "uppercase letters are not allowed".to_owned();
    }
    if ch == '-' {
        return "hyphen is not allowed".to_owned();
    }
    if ch == '.' {
        return "dot is not allowed".to_owned();
    }
    if ch == '\\' {
        return "backslash is not allowed".to_owned();
    }
    if ch.is_whitespace() {
        return "whitespace is not allowed".to_owned();
    }
    if ch == ':' {
        return "colon is not allowed inside path".to_owned();
    }
    format!("invalid character `{ch}`")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespaced_id_roundtrip() {
        let id = NamespacedId::parse("base:building/gas_pump").expect("valid id");
        assert_eq!(id.namespace(), "base");
        assert_eq!(id.path(), "building/gas_pump");
        assert_eq!(id.as_str(), "base:building/gas_pump");
        assert_eq!(id.to_string(), "base:building/gas_pump");

        let rebuilt = NamespacedId::new("base", "building/gas_pump").expect("valid id");
        assert_eq!(rebuilt, id);
    }

    #[test]
    fn namespaced_id_missing_separator_is_structured() {
        let error = NamespacedId::parse("base_building/gas_pump").expect_err("must fail");
        assert_eq!(
            error,
            NamespacedIdError::MissingSeparator {
                value: "base_building/gas_pump".to_owned(),
            }
        );
    }

    #[test]
    fn namespaced_id_multiple_separators_is_structured() {
        let error = NamespacedId::parse("base:building:gas_pump").expect_err("must fail");
        assert_eq!(
            error,
            NamespacedIdError::MultipleSeparators {
                value: "base:building:gas_pump".to_owned(),
                count: 2,
            }
        );
    }

    #[test]
    fn namespaced_id_rejects_bad_namespace_characters() {
        let error = NamespacedId::parse("Base:building/gas_pump").expect_err("must fail");
        assert!(matches!(
            error,
            NamespacedIdError::InvalidNamespaceCharacter {
                position: 0,
                character: 'B',
                ..
            }
        ));
        assert!(error.to_string().contains("uppercase"));
    }

    #[test]
    fn namespaced_id_rejects_bad_path_forms() {
        let cases = [
            "base:building//gas_pump",
            "base:building/Gas_pump",
            "base:building/gas-pump",
            "base:building/gas.pump",
            "base:building\\gas_pump",
            "base:building/gas pump",
        ];

        for value in cases {
            let error = NamespacedId::parse(value).expect_err("must fail");
            assert!(
                matches!(
                    error,
                    NamespacedIdError::EmptyPathSegment { .. }
                        | NamespacedIdError::InvalidPathSegmentStart { .. }
                        | NamespacedIdError::InvalidPathCharacter { .. }
                ),
                "unexpected error for {value}: {error:?}"
            );
        }
    }

    #[test]
    fn mod_id_accepts_valid_namespace() {
        let mod_id = ModId::parse("advanced_chemistry").expect("valid mod id");
        assert_eq!(mod_id.as_str(), "advanced_chemistry");
        assert_eq!(mod_id.to_string(), "advanced_chemistry");
    }

    #[test]
    fn mod_id_rejects_invalid_namespace() {
        let starts_with_digit = ModId::parse("1bad_mod").expect_err("must fail");
        assert!(matches!(
            starts_with_digit,
            ModIdError::InvalidFirstCharacter { character: '1', .. }
        ));

        let bad_character = ModId::parse("bad-mod").expect_err("must fail");
        assert!(matches!(
            bad_character,
            ModIdError::InvalidCharacter {
                position: 3,
                character: '-',
                ..
            }
        ));
    }

    #[test]
    fn prototype_id_roundtrip() {
        let parsed = PrototypeId::parse("base:scenario/bootstrap_smoke").expect("valid id");
        let reparsed = parsed
            .to_string()
            .parse::<PrototypeId>()
            .expect("roundtrip parse");
        assert_eq!(parsed, reparsed);
        assert_eq!(parsed.namespace(), "base");
        assert_eq!(parsed.path(), "scenario/bootstrap_smoke");
    }

    #[test]
    fn namespaced_and_mod_ids_serialize_as_strings() {
        let namespaced = NamespacedId::parse("base:material/oxygen").expect("valid id");
        let mod_id = ModId::parse("base").expect("valid mod id");
        let prototype = PrototypeId::parse("base:building/gas_pump").expect("valid id");

        let namespaced_json = serde_json::to_string(&namespaced).expect("serialize");
        let mod_json = serde_json::to_string(&mod_id).expect("serialize");
        let prototype_json = serde_json::to_string(&prototype).expect("serialize");

        assert_eq!(namespaced_json, "\"base:material/oxygen\"");
        assert_eq!(mod_json, "\"base\"");
        assert_eq!(prototype_json, "\"base:building/gas_pump\"");

        let namespaced_back: NamespacedId = serde_json::from_str(&namespaced_json).expect("parse");
        let mod_back: ModId = serde_json::from_str(&mod_json).expect("parse");
        let prototype_back: PrototypeId = serde_json::from_str(&prototype_json).expect("parse");

        assert_eq!(namespaced_back, namespaced);
        assert_eq!(mod_back, mod_id);
        assert_eq!(prototype_back, prototype);
    }
}
