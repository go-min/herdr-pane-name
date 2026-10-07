#![cfg(unix)]
use serde_json::{json, Value};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn sync_move_manual_edit_clear_and_reset() {
    let dir = std::env::temp_dir().join(format!("pane-name-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let fake = dir.join("herdr");
    fs::write(&fake, r#"#!/bin/sh
printf '%s\n' "$*" >> "$HERDR_PLUGIN_CONFIG_DIR/calls"
case "$1 $2" in
  'api snapshot') cat "$HERDR_PLUGIN_CONFIG_DIR/snapshot" ;;
  'pane report-metadata') exit 0 ;;
  'pane process-info') printf '%s\n' '{"result":{"process_info":{"foreground_processes":[{"name":"zsh","argv":["zsh"]}]}}}' ;;
  *) if [ -f "$HERDR_PLUGIN_CONFIG_DIR/fail" ] && [ "$2" != "report-metadata" ]; then
       printf '%s\n' '{"error":{"code":"test_failure","message":"rename rejected"}}'
       exit 0
     fi
     printf '%s\n' '{"result":{}}' ;;
esac
"#).unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(dir.join("config.toml"), "icons = false\n").unwrap();
    let mut snapshot = json!({"result":{"type":"session_snapshot", "snapshot":{
        "workspaces":[{"workspace_id":"w1", "name":"project"}],
        "tabs":[{"tab_id":"w1:t1", "workspace_id":"w1", "label":"1"}],
        "panes":[{"pane_id":"w1:p1", "terminal_id":"term1", "tab_id":"w1:t1", "workspace_id":"w1", "focused":true, "label":null}],
        "agents":[{"pane_id":"w1:p1", "terminal_id":"term1", "workspace_id":"w1", "agent":"codex", "name":"review", "display_agent":"Custom agent"}]
    }}});
    let invoke = |command: &str, snapshot: &Value| {
        fs::write(dir.join("snapshot"), serde_json::to_vec(snapshot).unwrap()).unwrap();
        fs::write(dir.join("calls"), "").unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_herdr-pane-name"))
            .arg(command)
            .env("HERDR_BIN_PATH", &fake)
            .env("HERDR_PLUGIN_CONFIG_DIR", &dir)
            .env("HERDR_PANE_NAME_STATE_FILE", dir.join("labels.json"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        fs::read_to_string(dir.join("calls")).unwrap()
    };
    let calls = invoke("sync", &snapshot);
    assert_eq!(
        calls.lines().filter(|line| *line == "api snapshot").count(),
        1
    );
    assert!(calls.contains("pane rename w1:p1 1:zsh"));
    assert!(calls.contains("--display-agent 1:Custom agent"));
    let records = &mut snapshot["result"]["snapshot"];
    records["workspaces"][0]["name"] = json!("1:project");
    records["tabs"][0]["label"] = json!("1:zsh");
    records["panes"][0]["pane_id"] = json!("w2:p4");
    records["panes"][0]["label"] = json!("1:zsh");
    records["agents"][0]["pane_id"] = json!("w2:p4");
    records["agents"][0]["display_agent"] = json!("1:Custom agent");
    invoke("sync", &snapshot);
    let state: Value = serde_json::from_slice(&fs::read(dir.join("labels.json")).unwrap()).unwrap();
    assert_eq!(state["labels"]["pane:w2:p4"], "1:zsh");
    assert!(state["labels"].get("pane:w1:p1").is_none());
    fs::write(dir.join("fail"), "").unwrap();
    let failed_clear = Command::new(env!("CARGO_BIN_EXE_herdr-pane-name"))
        .arg("clear")
        .env("HERDR_BIN_PATH", &fake)
        .env("HERDR_PLUGIN_CONFIG_DIR", &dir)
        .env("HERDR_PANE_NAME_STATE_FILE", dir.join("labels.json"))
        .output()
        .unwrap();
    assert!(!failed_clear.status.success());
    let retained: Value =
        serde_json::from_slice(&fs::read(dir.join("labels.json")).unwrap()).unwrap();
    assert_eq!(retained["labels"]["pane:w2:p4"], "1:zsh");
    fs::remove_file(dir.join("fail")).unwrap();
    let calls = invoke("clear", &snapshot);
    assert!(calls.contains("pane rename w2:p4 --clear"));
    assert!(calls.contains("--display-agent Custom agent"));
    let state: Value = serde_json::from_slice(&fs::read(dir.join("labels.json")).unwrap()).unwrap();
    assert_eq!(state["labels"]["pane:w2:p4"], "1:zsh");
    snapshot["result"]["snapshot"]["panes"][0]["label"] = Value::Null;
    snapshot["result"]["snapshot"]["agents"][0]["display_agent"] = json!("Custom agent");
    assert!(invoke("sync", &snapshot).contains("--display-agent 1:Custom agent"));
    snapshot["result"]["snapshot"]["panes"][0]["label"] = json!("manual pane");
    snapshot["result"]["snapshot"]["tabs"][0]["label"] = json!("manual tab");
    snapshot["result"]["snapshot"]["agents"][0]["display_agent"] = json!("manual agent");
    let calls = invoke("reset", &snapshot);
    assert!(!calls.contains("pane rename"));
    assert!(!calls.contains("tab rename"));
    assert!(!calls.contains("report-metadata"));
    let state: Value = serde_json::from_slice(&fs::read(dir.join("labels.json")).unwrap()).unwrap();
    assert_eq!(state["labels"], json!({}));
    assert_eq!(state["agent_labels"], json!({}));
    snapshot["result"]["snapshot"]["workspaces"][0]["name"] = json!("project");
    fs::write(dir.join("snapshot"), serde_json::to_vec(&snapshot).unwrap()).unwrap();
    fs::write(dir.join("fail"), "").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_herdr-pane-name"))
        .arg("sync")
        .env("HERDR_BIN_PATH", &fake)
        .env("HERDR_PLUGIN_CONFIG_DIR", &dir)
        .env("HERDR_PANE_NAME_STATE_FILE", dir.join("labels.json"))
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "API errors must not be treated as successful mutations"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn concurrent_syncs_do_not_adopt_another_syncs_prefix() {
    let dir = std::env::temp_dir().join(format!("pane-name-concurrent-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let fake = dir.join("herdr");
    fs::write(&fake, r#"#!/bin/sh
case "$1 $2" in
 'api snapshot')
  printf '{"result":{"snapshot":{"workspaces":[],"tabs":[],"panes":[],"agents":[{"terminal_id":"term1","pane_id":"p1","agent":"codex","display_agent":"%s"}]}}}' "$(cat "$HERDR_PLUGIN_CONFIG_DIR/display")" ;;
 'pane report-metadata')
  printf '%s' "$9" > "$HERDR_PLUGIN_CONFIG_DIR/display"
  sleep 0.3 ;;
esac
"#).unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(dir.join("display"), "codex").unwrap();
    let start = || {
        Command::new(env!("CARGO_BIN_EXE_herdr-pane-name"))
            .arg("sync")
            .env("HERDR_BIN_PATH", &fake)
            .env("HERDR_PLUGIN_CONFIG_DIR", &dir)
            .env("HERDR_PANE_NAME_STATE_FILE", dir.join("labels.json"))
            .spawn()
            .unwrap()
    };
    let mut first = start();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while fs::read_to_string(dir.join("display")).unwrap() != "1:codex" {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let mut second = start();
    assert!(first.wait().unwrap().success());
    assert!(second.wait().unwrap().success());
    assert_eq!(fs::read_to_string(dir.join("display")).unwrap(), "1:codex");
    fs::remove_dir_all(dir).unwrap();
}
