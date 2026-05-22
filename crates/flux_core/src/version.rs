use std::fmt::{Display, Formatter};
use std::str::FromStr;

use semver::Version;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{ENGINE_VERSION, VersionParseError};

macro_rules! define_version_wrapper {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Version);

        impl $name {
            #[must_use]
            pub fn new(version: Version) -> Self {
                Self(version)
            }

            pub fn parse(value: &str) -> Result<Self, VersionParseError> {
                Version::parse(value).map(Self).map_err(|error| {
                    VersionParseError::invalid_semver(stringify!($name), value, error.to_string())
                })
            }

            #[must_use]
            pub fn as_semver(&self) -> &Version {
                &self.0
            }

            #[must_use]
            pub fn into_semver(self) -> Version {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                Display::fmt(&self.0, f)
            }
        }

        impl FromStr for $name {
            type Err = VersionParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::parse(s)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = VersionParseError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(&value).map_err(serde::de::Error::custom)
            }
        }
    };
}

define_version_wrapper!(ApiVersion);
define_version_wrapper!(ModVersion);
define_version_wrapper!(EngineVersion);

pub fn engine_version() -> Result<EngineVersion, VersionParseError> {
    EngineVersion::parse(ENGINE_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_version_roundtrip() {
        let parsed = ApiVersion::parse("1.2.3").expect("valid semver");
        assert_eq!(parsed.to_string(), "1.2.3");
        let roundtrip = "1.2.3".parse::<ApiVersion>().expect("valid semver");
        assert_eq!(parsed, roundtrip);
    }

    #[test]
    fn mod_version_rejects_invalid_semver() {
        let error = ModVersion::parse("1.2").expect_err("must fail");
        assert!(matches!(
            error,
            VersionParseError::InvalidSemver {
                kind: "ModVersion",
                ..
            }
        ));
        assert!(error.to_string().contains("invalid ModVersion"));
    }

    #[test]
    fn wrappers_serialize_as_semver_strings() {
        let api = ApiVersion::parse("0.8.0-alpha.1").expect("valid semver");
        let mod_version = ModVersion::parse("2.0.0+build.5").expect("valid semver");
        let engine = EngineVersion::parse("0.0.0").expect("valid semver");

        let api_json = serde_json::to_string(&api).expect("serialize");
        let mod_json = serde_json::to_string(&mod_version).expect("serialize");
        let engine_json = serde_json::to_string(&engine).expect("serialize");

        assert_eq!(api_json, "\"0.8.0-alpha.1\"");
        assert_eq!(mod_json, "\"2.0.0+build.5\"");
        assert_eq!(engine_json, "\"0.0.0\"");

        let api_back: ApiVersion = serde_json::from_str(&api_json).expect("deserialize");
        let mod_back: ModVersion = serde_json::from_str(&mod_json).expect("deserialize");
        let engine_back: EngineVersion = serde_json::from_str(&engine_json).expect("deserialize");

        assert_eq!(api_back, api);
        assert_eq!(mod_back, mod_version);
        assert_eq!(engine_back, engine);
    }

    #[test]
    fn engine_version_matches_constant() {
        let typed = engine_version().expect("workspace package version must be valid semver");
        assert_eq!(typed.to_string(), ENGINE_VERSION);
    }
}
