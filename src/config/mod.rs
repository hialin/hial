use std::path::PathBuf;

pub(crate) fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|path| path.join("hial"))
}
