use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    config::Config,
    herdr::{self, Client},
    naming,
    state::{self, State},
};

pub fn run(cfg: &Config, reset: bool) -> Result<()> {
    let client = Client::default();
    let mut state = state::load();
    let workspaces = client.list("workspace")?;
    let tabs = client.list("tab")?;
    let panes = client.list("pane")?;

    sync_workspaces(cfg, reset, &client, &mut state, &workspaces)?;
    sync_panes(cfg, reset, &client, &mut state, &panes)?;
    sync_tabs(cfg, reset, &client, &mut state, &tabs, &panes)?;

    if reset {
        reset_owned(&client, &mut state)?;
    }
    state::save(&state)
}

pub fn clear(_cfg: &Config) -> Result<()> {
    let client = Client::default();
    let state = state::load();
    for (key, old) in &state.labels {
        let (kind, id) = split_key(key);
        let label = if kind == "workspace" {
            naming::without_prefix(old)
        } else {
            ""
        };
        let _ = client.rename(kind, id, label);
    }
    state::save(&state)
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
        let generated = naming::automatic(cfg, &process, &argv, position);
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
        let generated = naming::automatic(cfg, &process, &argv, position);
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

fn reset_owned(client: &Client, state: &mut State) -> Result<()> {
    for (key, old) in state.labels.clone() {
        let (kind, id) = split_key(&key);
        let label = if kind == "workspace" {
            naming::without_prefix(&old)
        } else {
            ""
        };
        let _ = client.rename(kind, id, label);
    }
    state.labels.clear();
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
