use std::path::{Path, PathBuf};

pub fn load_env() -> Vec<PathBuf> {
    let manifest_env = Path::new(env!("CARGO_MANIFEST_DIR")).join(".env");
    let mut loaded = Vec::new();

    if dotenvy::from_path(&manifest_env).is_ok() {
        loaded.push(manifest_env);
    }
    if let Ok(path) = dotenvy::dotenv() {
        if !loaded.contains(&path) {
            loaded.push(path);
        }
    }

    loaded
}
