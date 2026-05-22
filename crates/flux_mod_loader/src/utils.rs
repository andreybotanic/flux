use std::path::Path;

pub(crate) fn mod_directory_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path_for_error(path))
}

pub(crate) fn path_for_error(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
