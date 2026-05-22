use crate::{ModIdError, NamespacedIdError};

use super::{ModId, NamespacedId, PrototypeId};

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
