use crate::{ModIdError, NamespacedIdError};

pub(super) fn validate_namespace(namespace: &str) -> Result<(), NamespacedIdError> {
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

pub(super) fn validate_path(path: &str) -> Result<(), NamespacedIdError> {
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

pub(super) fn validate_mod_id(value: &str) -> Result<(), ModIdError> {
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
