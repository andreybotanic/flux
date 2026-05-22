use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IdError {
    #[error(transparent)]
    Namespaced(#[from] NamespacedIdError),
    #[error(transparent)]
    Mod(#[from] ModIdError),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NamespacedIdError {
    #[error("invalid namespaced id `{value}`: expected exactly one `:` separator")]
    MissingSeparator { value: String },

    #[error("invalid namespaced id `{value}`: expected exactly one `:` separator, found {count}")]
    MultipleSeparators { value: String, count: usize },

    #[error("invalid namespaced id `{value}`: namespace must not be empty")]
    EmptyNamespace { value: String },

    #[error("invalid namespaced id `{value}`: path must not be empty")]
    EmptyPath { value: String },

    #[error(
        "invalid namespaced id namespace `{value}` at char {position} (`{character}`): {reason}"
    )]
    InvalidNamespaceCharacter {
        value: String,
        position: usize,
        character: char,
        reason: String,
    },

    #[error(
        "invalid namespaced id path segment {segment_index} in `{value}` starts with `{character}`: {reason}"
    )]
    InvalidPathSegmentStart {
        value: String,
        segment_index: usize,
        character: char,
        reason: String,
    },

    #[error("invalid namespaced id path `{value}`: empty segment at index {segment_index}")]
    EmptyPathSegment { value: String, segment_index: usize },

    #[error(
        "invalid namespaced id path segment {segment_index} in `{value}` at char {position} (`{character}`): {reason}"
    )]
    InvalidPathCharacter {
        value: String,
        segment_index: usize,
        position: usize,
        character: char,
        reason: String,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ModIdError {
    #[error("invalid mod id: namespace must not be empty")]
    Empty,

    #[error("invalid mod id `{value}` starts with `{character}`: {reason}")]
    InvalidFirstCharacter {
        value: String,
        character: char,
        reason: String,
    },

    #[error("invalid mod id `{value}` at char {position} (`{character}`): {reason}")]
    InvalidCharacter {
        value: String,
        position: usize,
        character: char,
        reason: String,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VersionParseError {
    #[error("invalid {kind} `{value}`: {reason}")]
    InvalidSemver {
        kind: &'static str,
        value: String,
        reason: String,
    },
}

impl VersionParseError {
    #[must_use]
    pub fn invalid_semver(kind: &'static str, value: &str, reason: String) -> Self {
        Self::InvalidSemver {
            kind,
            value: value.to_owned(),
            reason,
        }
    }
}
