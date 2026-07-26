use anyhow::{Context, Result};
use serde_json::Value;
use std::{env, path::Path, process::Command};

pub struct Client {
    binary: std::ffi::OsString,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            binary: env::var_os("HERDR_BIN_PATH").unwrap_or_else(|| "herdr".into()),
        }
    }
}

impl Client {
    pub fn run(&self, args: &[&str]) -> Result<Value> {
        let output = Command::new(&self.binary)
            .args(args)
            .output()
            .with_context(|| format!("running herdr {args:?}"))?;
        if !output.status.success() {
            anyhow::bail!(
                "herdr {args:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        serde_json::from_slice(&output.stdout)
            .with_context(|| format!("invalid JSON from herdr {args:?}"))
    }

    pub fn list(&self, kind: &str) -> Result<Vec<Value>> {
        let response = self.run(&[kind, "list"])?;
        Ok(array(&response, kind).into_iter().cloned().collect())
    }

    pub fn process_info(&self, pane_id: &str) -> Result<Value> {
        self.run(&["pane", "process-info", "--pane", pane_id])
    }

    pub fn rename(&self, kind: &str, id: &str, label: &str) -> Result<()> {
        let args = if label.is_empty() {
            vec![kind, "rename", id, "--clear"]
        } else {
            vec![kind, "rename", id, label]
        };
        self.run(&args)?;
        Ok(())
    }
}

pub fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    let result = value.get("result").unwrap_or(value);
    if let Some(array) = result.as_array() {
        return array.iter().collect();
    }
    result
        .get(match key {
            "workspace" => "workspaces",
            "tab" => "tabs",
            "pane" => "panes",
            _ => key,
        })
        .and_then(Value::as_array)
        .map(|array| array.iter().collect())
        .unwrap_or_default()
}

pub fn string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(key).and_then(Value::as_str).map(str::to_owned))
}

pub fn id(value: &Value, kind: &str) -> Option<String> {
    string(value, &[&format!("{kind}_id"), "id"])
}

pub fn foreground_program(value: &Value) -> Option<(String, Vec<String>)> {
    let info = value
        .get("result")
        .and_then(|result| result.get("process_info"))
        .unwrap_or(value);
    let process = info.get("foreground_processes")?.as_array()?.last()?;
    let argv = process
        .get("argv")
        .and_then(Value::as_array)
        .map(|argv| {
            argv.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let name = string(process, &["name", "argv0", "command"])?;
    let name = Path::new(&name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&name)
        .to_owned();
    Some((name, argv))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn array_supports_wrapped_and_named_responses() {
        let response = json!({"result": {"panes": [{"id": "p1"}]}});
        assert_eq!(array(&response, "pane").len(), 1);

        let response = json!({"result": [{"id": "p1"}]});
        assert_eq!(array(&response, "pane").len(), 1);
    }

    #[test]
    fn foreground_program_uses_the_last_process_and_basename() {
        let response = json!({
            "result": {
                "process_info": {
                    "foreground_processes": [
                        {"name": "zsh", "argv": ["zsh"]},
                        {"name": "/usr/bin/nvim", "argv": ["nvim", "file.rs"]}
                    ]
                }
            }
        });

        assert_eq!(
            foreground_program(&response),
            Some((
                "nvim".to_owned(),
                vec!["nvim".to_owned(), "file.rs".to_owned()]
            ))
        );
    }
}
