use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SaveIoError {
    #[error("SaveIoError:\n  action: validate_save_id\n  save_id: {save_id}\n  reason: {reason}")]
    InvalidSaveId { save_id: String, reason: String },

    #[error(
        "SaveIoError:\n  action: create_save_directory\n  save_id: {save_id}\n  path: {path}\n  reason: {reason}"
    )]
    CreateSaveDirectory {
        save_id: String,
        path: String,
        reason: String,
    },

    #[error(
        "SaveIoError:\n  action: write_save_file\n  save_id: {save_id}\n  file: {file}\n  reason: {reason}"
    )]
    WriteSaveFile {
        save_id: String,
        file: String,
        reason: String,
    },

    #[error(
        "SaveIoError:\n  action: read_save_file\n  save_id: {save_id}\n  file: {file}\n  reason: {reason}"
    )]
    ReadSaveFile {
        save_id: String,
        file: String,
        reason: String,
    },

    #[error(
        "SaveIoError:\n  action: parse_manifest\n  save_id: {save_id}\n  file: {file}\n  reason: {reason}"
    )]
    ParseManifest {
        save_id: String,
        file: String,
        reason: String,
    },

    #[error(
        "SaveIoError:\n  action: validate_manifest\n  save_id: {save_id}\n  field: {field}\n  reason: {reason}"
    )]
    InvalidManifest {
        save_id: String,
        field: String,
        reason: String,
    },

    #[error(
        "SaveIoError:\n  action: encode_payload\n  save_id: {save_id}\n  layer: {layer}\n  reason: {reason}"
    )]
    EncodePayload {
        save_id: String,
        layer: String,
        reason: String,
    },

    #[error(
        "SaveIoError:\n  action: decode_payload\n  save_id: {save_id}\n  layer: {layer}\n  reason: {reason}"
    )]
    DecodePayload {
        save_id: String,
        layer: String,
        reason: String,
    },

    #[error("SaveIoError:\n  action: restore_world\n  save_id: {save_id}\n  reason: {reason}")]
    RestoreWorld { save_id: String, reason: String },
}
