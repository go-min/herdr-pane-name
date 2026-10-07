use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    config::Config,
    herdr::{self, Client},
    naming,
    state::{self, AgentLabel, State},
};

pub fn run(cfg: &Config, reset: bool) -> Result<()> {
    synchronize(cfg, reset, reset)
}

fn synchronize(cfg: &Config, reset: bool, forget: bool) -> Result<()> {
    let lock_path = state::path().with_extension("lock");
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)?;
    // Hold the OS lock through snapshot, mutations, and state save across all hooks.
    fs2::FileExt::lock_exclusive(&lock)?;
    let client = Client::default();
    let mut state = state::load();
    let herdr::Snapshot {
        workspaces,
        tabs,
        panes,
        agents,
    } = client.snapshot()?;
    state.reconcile_panes(&panes);

    for labels in [&mut state.labels, &mut state.manual_prefixes] {
        labels.retain(|key, _| {
            let (kind, id) = split_key(key);
            match kind {
                "workspace" => workspaces
                    .iter()
                    .any(|record| herdr::id(record, kind).as_deref() == Some(id)),
                "tab" => tabs
                    .iter()
                    .any(|record| herdr::id(record, kind).as_deref() == Some(id)),
                _ => true,
            }
        });
    }
    let result = (|| -> Result<()> {
        sync_workspaces(cfg, reset, &client, &mut state, &workspaces)?;
        sync_panes(cfg, reset, &client, &mut state, &panes)?;
        sync_tabs(cfg, reset, &client, &mut state, &tabs, &panes)?;

        sync_agents(cfg, reset, &client, &mut state, &agents)?;
        if reset {
            reset_owned(&client, &mut state, forget)?;
            if forget {
                state.agent_labels.clear();
            }
        }
        Ok(())
    })();
    // Keep ownership of successful mutations even if a later API call fails.
    state::save(&state)?;
    result
}

pub fn clear(cfg: &Config) -> Result<()> {
    // Reconcile current labels before clearing, so manual edits are preserved.
    synchronize(cfg, true, false)
}

fn sync_workspaces(
    cfg: &Config,
    reset: bool,
    client: &Client,
    state: &mut State,
    workspaces: &[Value],
) -> Result<()> {
    for (position, workspace) in workspaces.iter().enumerate() {
        let Some(id) = herdr::id(workspace, "workspace") else {
            continue;
        };
        let current = herdr::string(workspace, &["label", "name"]).unwrap_or_default();
        let key = format!("workspace:{id}");
        if let Some(last) = state.labels.get(&key) {
            if current != *last && !current.is_empty() {
                state.labels.remove(&key);
                continue;
            }
        }
        let base = naming::without_prefix(&current);
        let next = naming::prefixed(cfg, base, position);
        if !reset {
            if current != next {
                client.rename("workspace", &id, &next)?;
            }
            state.labels.insert(key, next);
        }
    }
    Ok(())
}

fn sync_panes(
    cfg: &Config,
    reset: bool,
    client: &Client,
    state: &mut State,
    panes: &[Value],
) -> Result<()> {
    let mut positions = HashMap::<String, usize>::new();
    for pane in panes {
        let Some(id) = herdr::id(pane, "pane") else {
            continue;
        };
        let tab_id = herdr::string(pane, &["tab_id"]).unwrap_or_default();
        let position = next_position(&mut positions, tab_id);
        let current = herdr::string(pane, &["label", "name"]).unwrap_or_default();
        let key = format!("pane:{id}");
        if reconcile_manual_prefix(
            cfg,
            client,
            state,
            LabelRef {
                key: &key,
                kind: "pane",
                id: &id,
                current: &current,
                position,
            },
        )? {
            continue;
        }
        let Some((process, argv)) = client
            .process_info(&id)
            .ok()
            .and_then(|response| herdr::foreground_program(&response))
        else {
            continue;
        };
        if naming::is_ignored(cfg, &process) {
            continue;
        }
        let generated = naming::for_pane(cfg, pane, &process, &argv, position);
        if !state.labels.contains_key(&key)
            && !current.is_empty()
            && current != generated
            && current != process
        {
            continue;
        }
        if !reset {
            if current != generated {
                client.rename("pane", &id, &generated)?;
            }
            state.labels.insert(key, generated);
        }
    }
    Ok(())
}

fn sync_tabs(
    cfg: &Config,
    reset: bool,
    client: &Client,
    state: &mut State,
    tabs: &[Value],
    panes: &[Value],
) -> Result<()> {
    let mut positions = HashMap::<String, usize>::new();
    for tab in tabs {
        let Some(id) = herdr::id(tab, "tab") else {
            continue;
        };
        let workspace_id = herdr::string(tab, &["workspace_id"]).unwrap_or_default();
        let position = next_position(&mut positions, workspace_id);
        let current = herdr::string(tab, &["label", "name"]).unwrap_or_default();
        let key = format!("tab:{id}");
        if reconcile_manual_prefix(
            cfg,
            client,
            state,
            LabelRef {
                key: &key,
                kind: "tab",
                id: &id,
                current: &current,
                position,
            },
        )? {
            continue;
        }

        let focused = panes
            .iter()
            .filter(|pane| herdr::string(pane, &["tab_id"]) == Some(id.clone()))
            .find(|pane| {
                pane.get("focused")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            });
        let source = focused.or_else(|| {
            if state.labels.contains_key(&key) {
                None
            } else {
                panes
                    .iter()
                    .find(|pane| herdr::string(pane, &["tab_id"]) == Some(id.clone()))
            }
        });
        let Some(source) = source else {
            continue;
        };
        let Some(pane_id) = herdr::id(source, "pane") else {
            continue;
        };
        let Some((process, argv)) = client
            .process_info(&pane_id)
            .ok()
            .and_then(|response| herdr::foreground_program(&response))
        else {
            continue;
        };
        if naming::is_ignored(cfg, &process) {
            continue;
        }
        let generated = naming::for_pane(cfg, source, &process, &argv, position);
        if !state.labels.contains_key(&key)
            && !current.is_empty()
            && current != generated
            && !naming::is_default_label(&current)
            && !naming::is_numeric_label(&current)
        {
            continue;
        }
        if !reset {
            if current != generated {
                client.rename("tab", &id, &generated)?;
            }
            state.labels.insert(key, generated);
        }
    }
    Ok(())
}

fn plan_agent(
    cfg: &Config,
    agent: &Value,
    old: Option<&AgentLabel>,
    position: usize,
) -> Option<AgentLabel> {
    let kind = herdr::string(agent, &["agent"])?;
    let name = herdr::string(agent, &["name"]);
    let mut current = herdr::string(agent, &["display_agent"]);
    if old.is_some_and(|old| {
        (old.agent != kind || old.name != name) && current.as_deref() == Some(&old.last)
    }) {
        current = None;
    }
    let mut owned = match old.filter(|old| old.agent == kind && old.name == name) {
        Some(old) => {
            let mut owned = old.clone();
            if current.as_deref() != Some(&old.last) && !(old.cleared && current == old.previous) {
                owned.manual = true;
            }
            owned
        }
        None => AgentLabel {
            agent: kind.clone(),
            name: name.clone(),
            previous: current,
            last: String::new(),
            manual: false,
            cleared: false,
        },
    };
    if !owned.manual {
        let base = owned
            .previous
            .as_deref()
            .or(name.as_deref())
            .unwrap_or(&kind);
        owned.last = naming::prefixed(cfg, base, position);
    }
    owned.cleared = false;
    Some(owned)
}

fn sync_agents(
    cfg: &Config,
    reset: bool,
    client: &Client,
    state: &mut State,
    agents: &[Value],
) -> Result<()> {
    let mut positions = HashMap::new();
    let mut live = Vec::new();
    for agent in agents {
        let Some(terminal) = herdr::string(agent, &["terminal_id"]) else {
            continue;
        };
        let Some(pane) = herdr::id(agent, "pane") else {
            continue;
        };
        live.push(terminal.clone());
        let position = next_position(
            &mut positions,
            herdr::string(agent, &["workspace_id"]).unwrap_or_default(),
        );
        if reset {
            if let Some(old) = state.agent_labels.get_mut(&terminal) {
                let current = herdr::string(agent, &["display_agent"]);
                if !old.manual
                    && herdr::string(agent, &["agent"]).as_deref() == Some(&old.agent)
                    && herdr::string(agent, &["name"]) == old.name
                    && current.as_deref() == Some(&old.last)
                {
                    client.display_agent(&pane, &old.agent, old.previous.as_deref())?;
                    old.cleared = true;
                } else if current.as_deref() != Some(&old.last)
                    && !(old.cleared && current == old.previous)
                {
                    old.manual = true;
                }
            }
            continue;
        }
        let Some(owned) = plan_agent(cfg, agent, state.agent_labels.get(&terminal), position)
        else {
            continue;
        };
        let current = herdr::string(agent, &["display_agent"]);
        if !owned.manual && current.as_deref() != Some(&owned.last) {
            client.display_agent(&pane, &owned.agent, Some(&owned.last))?;
        }
        state.agent_labels.insert(terminal, owned);
    }
    state
        .agent_labels
        .retain(|terminal, _| live.contains(terminal));
    Ok(())
}

struct LabelRef<'a> {
    key: &'a str,
    kind: &'a str,
    id: &'a str,
    current: &'a str,
    position: usize,
}

fn reconcile_manual_prefix(
    cfg: &Config,
    client: &Client,
    state: &mut State,
    label: LabelRef<'_>,
) -> Result<bool> {
    if let Some(stored) = state.manual_prefixes.get(label.key).cloned() {
        if naming::has_prefix(label.current) {
            let base = naming::without_prefix(label.current).to_owned();
            let next = naming::prefixed(cfg, &base, label.position);
            if label.current != next {
                client.rename(label.kind, label.id, &next)?;
            }
            state.manual_prefixes.insert(label.key.to_owned(), base);
        } else if label.current != naming::prefixed(cfg, &stored, label.position) {
            state.manual_prefixes.remove(label.key);
        }
        return Ok(true);
    }

    let Some(last) = state.labels.get(label.key).cloned() else {
        return Ok(false);
    };
    if label.current == last || label.current.is_empty() {
        return Ok(false);
    }
    if naming::has_prefix(label.current) {
        let base = naming::without_prefix(label.current).to_owned();
        let next = naming::prefixed(cfg, &base, label.position);
        if label.current != next {
            client.rename(label.kind, label.id, &next)?;
        }
        state.manual_prefixes.insert(label.key.to_owned(), base);
    }
    state.labels.remove(label.key);
    Ok(true)
}

fn next_position(positions: &mut HashMap<String, usize>, group: String) -> usize {
    let position = positions.entry(group).or_insert(0);
    let current = *position;
    *position += 1;
    current
}

fn reset_owned(client: &Client, state: &mut State, forget: bool) -> Result<()> {
    for (key, old) in state.labels.clone() {
        let (kind, id) = split_key(&key);
        let label = if kind == "workspace" {
            naming::without_prefix(&old)
        } else {
            ""
        };
        client.rename(kind, id, label)?;
    }
    if forget {
        state.labels.clear();
    }
    Ok(())
}

fn split_key(key: &str) -> (&str, &str) {
    key.split_once(':').unwrap_or((key, ""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn positions_are_local_to_each_group() {
        let mut positions = HashMap::new();
        assert_eq!(next_position(&mut positions, "w1".into()), 0);
        assert_eq!(next_position(&mut positions, "w1".into()), 1);
        assert_eq!(next_position(&mut positions, "w2".into()), 0);
    }

    #[test]
    fn prefixed_manual_name_keeps_its_base() {
        let cfg = Config::default();
        assert_eq!(naming::prefixed(&cfg, "review", 1), "2:review");
    }
}

#[cfg(test)]
mod agent_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn manual_agent_clear_is_preserved_but_plugin_clear_can_resume() {
        let cfg = Config::default();
        let agent = json!({"agent":"codex"});
        let mut old = plan_agent(&cfg, &agent, None, 0).unwrap();
        assert!(plan_agent(&cfg, &agent, Some(&old), 0).unwrap().manual);
        old.cleared = true;
        let resumed = plan_agent(&cfg, &agent, Some(&old), 0).unwrap();
        assert!(!resumed.manual);
        assert!(!resumed.cleared);
    }

    #[test]
    fn replacing_an_agent_does_not_adopt_its_old_prefix() {
        let cfg = Config::default();
        let old = plan_agent(&cfg, &json!({"agent":"codex","name":"review"}), None, 0).unwrap();
        let replacement = json!({"agent":"claude","name":"build","display_agent":"1:review"});
        assert_eq!(
            plan_agent(&cfg, &replacement, Some(&old), 1).unwrap().last,
            "2:build"
        );
    }

    #[test]
    fn agent_display_preserves_manual_changes_and_restarts_for_new_occupants() {
        let cfg = Config::default();
        let agent = json!({"agent":"codex", "name":"review", "display_agent":null});
        let owned = plan_agent(&cfg, &agent, None, 0).unwrap();
        assert_eq!(owned.last, "1:review");
        assert!(owned.previous.is_none());
        let changed = json!({"agent":"codex", "name":"review", "display_agent":"custom"});
        let manual = plan_agent(&cfg, &changed, Some(&owned), 1).unwrap();
        assert!(manual.manual);
        assert_eq!(
            plan_agent(&cfg, &changed, Some(&manual), 2).unwrap().last,
            "1:review"
        );
        let replaced = json!({"agent":"claude", "name":"build", "display_agent":null});
        assert_eq!(
            plan_agent(&cfg, &replaced, Some(&manual), 1).unwrap().last,
            "2:build"
        );
    }
}
