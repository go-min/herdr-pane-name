use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, path::PathBuf};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub labels: HashMap<String, String>,
    pub manual_prefixes: HashMap<String, String>,
}

pub fn path() -> PathBuf {
    if let Some(path) = env::var_os("HERDR_PANE_NAME_STATE_FILE") {
        return PathBuf::from(path);
    }
    if let Some(dir) = env::var_os("HERDR_PLUGIN_CONFIG_DIR") {
        return PathBuf::from(dir).join("labels.json");
    }
    env::var_os("HERDR_PLUGIN_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::temp_dir().join("herdr-pane-name"))
        .join("labels.json")
}

pub fn load() -> State {
    fs::read_to_string(path())
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

pub fn save(state: &State) -> Result<()> {
    let path = path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(state)?)?;
    Ok(())
}
