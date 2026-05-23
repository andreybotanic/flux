use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ContentRegistryError {
    #[error(
        "ContentRegistryError:\n  action: add_prototype\n  reason: registry is frozen\n  prototype_kind: {prototype_kind}\n  prototype_id: {prototype_id}"
    )]
    RegistryFrozenMutation {
        prototype_kind: Box<str>,
        prototype_id: Box<str>,
    },

    #[error(
        "ContentRegistryError:\n  action: discover_content\n  mod: {mod_id}\n  path: {path}\n  reason: failed to inspect directory ({reason})"
    )]
    DirectoryRead {
        mod_id: Box<str>,
        path: Box<str>,
        reason: Box<str>,
    },

    #[error(
        "ContentRegistryError:\n  action: read_content_file\n  mod: {mod_id}\n  file: {file}\n  reason: {reason}"
    )]
    FileRead {
        mod_id: Box<str>,
        file: Box<str>,
        reason: Box<str>,
    },

    #[error(
        "ContentRegistryError:\n  action: parse_content_file\n  mod: {mod_id}\n  file: {file}\n  prototype_kind: {prototype_kind}\n  reason: {reason}"
    )]
    FileParse {
        mod_id: Box<str>,
        file: Box<str>,
        prototype_kind: Box<str>,
        reason: Box<str>,
    },

    #[error(
        "ContentRegistryError:\n  action: parse_patch_file\n  mod: {mod_id}\n  first_file: {first_file}\n  duplicate_file: {duplicate_file}\n  target: {target}\n  reason: duplicate patch target in one mod"
    )]
    DuplicatePatchTargetInMod {
        mod_id: Box<str>,
        first_file: Box<str>,
        duplicate_file: Box<str>,
        target: Box<str>,
    },

    #[error(
        "ContentRegistryError:\n  action: validate_content\n  mod: {mod_id}\n  file: {file}\n  prototype_id: {prototype_id}\n  field: {field}\n  reason: {reason}"
    )]
    InvalidPrototypeField {
        mod_id: Box<str>,
        file: Box<str>,
        prototype_id: Box<str>,
        field: Box<str>,
        reason: Box<str>,
    },

    #[error(
        "ContentRegistryError:\n  action: apply_patch\n  mod: {mod_id}\n  file: {file}\n  target: {target}\n  patch_kind: {patch_kind}\n  reason: patch body has no fields"
    )]
    EmptyPatchBody {
        mod_id: Box<str>,
        file: Box<str>,
        target: Box<str>,
        patch_kind: Box<str>,
    },

    #[error(
        "ContentRegistryError:\n  action: apply_patch\n  mod: {mod_id}\n  file: {file}\n  target: {target}\n  reason: target prototype not found"
    )]
    PatchTargetNotFound {
        mod_id: Box<str>,
        file: Box<str>,
        target: Box<str>,
    },

    #[error(
        "ContentRegistryError:\n  action: apply_patch\n  mod: {mod_id}\n  file: {file}\n  target: {target}\n  target_kind: {target_kind}\n  patch_kind: {patch_kind}\n  reason: patch kind does not match target kind"
    )]
    PatchKindMismatch {
        mod_id: Box<str>,
        file: Box<str>,
        target: Box<str>,
        target_kind: Box<str>,
        patch_kind: Box<str>,
    },

    #[error(
        "ContentRegistryError:\n  action: apply_patch\n  mod: {mod_id}\n  file: {file}\n  target: {target}\n  reason: patch apply failed ({reason})"
    )]
    PatchApplyFailed {
        mod_id: Box<str>,
        file: Box<str>,
        target: Box<str>,
        reason: Box<str>,
    },

    #[error(
        "ContentRegistryError:\n  action: register_content\n  prototype_id: {prototype_id}\n  reason: duplicate prototype id\n  existing: kind={existing_kind}, mod={existing_mod}, file={existing_file}\n  duplicate: kind={duplicate_kind}, mod={duplicate_mod}, file={duplicate_file}"
    )]
    DuplicatePrototypeId {
        prototype_id: Box<str>,
        existing_kind: Box<str>,
        existing_mod: Box<str>,
        existing_file: Box<str>,
        duplicate_kind: Box<str>,
        duplicate_mod: Box<str>,
        duplicate_file: Box<str>,
    },

    #[error(
        "ContentRegistryError:\n  action: load_content\n  mod: {mod_id}\n  reason: mod is present in resolved order but missing from discovered set"
    )]
    ResolvedModMissing { mod_id: Box<str> },
}
