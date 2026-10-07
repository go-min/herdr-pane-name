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
        // Mutation commands such as report-metadata succeed without JSON output.
        if output.stdout.iter().all(u8::is_ascii_whitespace) {
            return Ok(Value::Null);
        }
        let response: Value = serde_json::from_slice(&output.stdout)
            .with_context(|| format!("invalid JSON from herdr {args:?}"))?;
        if let Some(error) = response.get("error") {
            anyhow::bail!("herdr {args:?} failed: {error}");
        }
        Ok(response)
    }

    pub fn snapshot(&self) -> Result<Snapshot> {
        snapshot_records(&self.run(&["api", "snapshot"])?)
    }

    pub fn display_agent(&self, pane: &str, agent: &str, label: Option<&str>) -> Result<()> {
        let mut args = vec![
            "pane",
            "report-metadata",
            pane,
            "--source",
            "plugin:herdr.pane-name",
            "--agent",
            agent,
        ];
        if let Some(label) = label {
            args.extend(["--display-agent", label]);
        } else {
            args.push("--clear-display-agent");
        }
        self.run(&args)?;
        Ok(())
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

pub struct Snapshot {
    pub workspaces: Vec<Value>,
    pub tabs: Vec<Value>,
    pub panes: Vec<Value>,
    pub agents: Vec<Value>,
}

fn snapshot_records(value: &Value) -> Result<Snapshot> {
    let result = value.get("result").unwrap_or(value);
    let result = result.get("snapshot").unwrap_or(result);
    let records = |key| {
        result
            .get(key)
            .and_then(Value::as_array)
            .cloned()
            .with_context(|| {
                format!("Herdr snapshot is missing {key}; update the running server to 0.9.3+")
            })
    };
    Ok(Snapshot {
        workspaces: records("workspaces")?,
        tabs: records("tabs")?,
        panes: records("panes")?,
        agents: records("agents")?,
    })
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

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn snapshot_requires_all_resource_arrays() {
        assert!(snapshot_records(
            &json!({"result":{"type":"session_snapshot", "snapshot":{"workspaces":[], "tabs":[], "panes":[], "agents":[]}}})
        )
        .is_ok());
        assert!(snapshot_records(&json!({"error":{"code":"unknown_method"}})).is_err());
        assert!(snapshot_records(&json!({"result":{"workspaces":[]}})).is_err());
    }
}
