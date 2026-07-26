use anyhow::Result;
use serde::Deserialize;
use std::{env, fs, path::PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub max_length: usize,
    pub show_args: bool,
    pub icons: bool,
    pub prefixes: bool,
    pub ignored_programs: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_length: 32,
            show_args: false,
            icons: true,
            prefixes: true,
            ignored_programs: Vec::new(),
        }
    }
}

pub fn load() -> Result<Config> {
    let Some(dir) = env::var_os("HERDR_PLUGIN_CONFIG_DIR") else {
        return Ok(Config::default());
    };
    let path = PathBuf::from(dir).join("config.toml");
    if !path.exists() {
        return Ok(Config::default());
    }
    Ok(toml::from_str(&fs::read_to_string(path)?)?)
}
