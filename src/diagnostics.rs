use std::path::Path;

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
