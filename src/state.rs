use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, path::PathBuf};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub labels: HashMap<String, String>,
    pub manual_prefixes: HashMap<String, String>,
    pub pane_ids: HashMap<String, String>,
    pub agent_labels: HashMap<String, AgentLabel>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AgentLabel {
    pub agent: String,
    pub name: Option<String>,
    pub previous: Option<String>,
    pub last: String,
    pub manual: bool,
    #[serde(default)]
    pub cleared: bool,
}

impl State {
    pub fn reconcile_panes(&mut self, panes: &[serde_json::Value]) {
        let next: HashMap<_, _> = panes
            .iter()
            .filter_map(|pane| {
                Some((
                    crate::herdr::string(pane, &["terminal_id"])?,
                    crate::herdr::id(pane, "pane")?,
                ))
            })
            .collect();
        // Migrate in two passes: a destination ID may be another moved pane's old ID.
        for labels in [&mut self.labels, &mut self.manual_prefixes] {
            for (terminal, old) in &self.pane_ids {
                if !next.contains_key(terminal) {
                    labels.remove(&format!("pane:{old}"));
                }
            }
            let moved: Vec<_> = next
                .iter()
                .filter_map(|(terminal, id)| {
                    let old = self.pane_ids.get(terminal)?;
                    (old != id).then(|| (format!("pane:{old}"), format!("pane:{id}")))
                })
                .filter_map(|(old, new)| labels.remove(&old).map(|label| (new, label)))
                .collect();
            labels.extend(moved);
            labels.retain(|key, _| {
                !key.starts_with("pane:") || next.values().any(|id| key == &format!("pane:{id}"))
            });
        }
        self.pane_ids = next;
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reused_pane_id_does_not_inherit_closed_terminal_ownership() {
        let mut state = State::default();
        state.pane_ids.insert("old".into(), "w1:p1".into());
        state.labels.insert("pane:w1:p1".into(), "1:zsh".into());
        state.reconcile_panes(&[json!({"terminal_id":"new", "pane_id":"w1:p1"})]);
        assert!(state.labels.is_empty());
    }

    #[test]
    fn moving_terminal_preserves_ownership_and_manual_prefixes() {
        let mut state = State::default();
        state.pane_ids.insert("term1".into(), "w1:p1".into());
        state.labels.insert("pane:w1:p1".into(), "1:zsh".into());
        state
            .manual_prefixes
            .insert("pane:w1:p1".into(), "review".into());
        state.reconcile_panes(&[json!({"terminal_id":"term1", "pane_id":"w2:p4"})]);
        assert_eq!(state.labels.get("pane:w2:p4").unwrap(), "1:zsh");
        assert_eq!(state.manual_prefixes.get("pane:w2:p4").unwrap(), "review");
        assert!(!state.labels.contains_key("pane:w1:p1"));
        state.reconcile_panes(&[]);
        assert!(state.labels.is_empty());
        assert!(state.manual_prefixes.is_empty());
    }
}
