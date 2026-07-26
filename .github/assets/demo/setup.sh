#!/usr/bin/env bash
set -euo pipefail

demo_root=/private/tmp/herdr-pane-name-vhs
demo_home=$demo_root/home
demo_project=$demo_home/project
socket_path=$demo_root/run/herdr.sock
repo_root=$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.."
    pwd
)
herdr_theme=${HERDR_DEMO_THEME:-catppuccin}

rm -rf "$demo_root"
mkdir -p "$demo_root/herdr" "$demo_root/run" "$demo_root/tmp" "$demo_project"
sed "s/__HERDR_THEME__/$herdr_theme/" \
    "$repo_root/.github/assets/demo/herdr-demo-config.toml" \
    >"$demo_root/herdr/config.toml"
sed "s#__HERDR_PLUGIN_ROOT__#$repo_root#" \
    "$repo_root/.github/assets/demo/bashrc" \
    >"$demo_home/.bashrc"

export HOME="$demo_home"
export USER=demo
export LOGNAME=demo
export XDG_CONFIG_HOME="$demo_root"
export XDG_DATA_HOME="$demo_root/data"
export XDG_CACHE_HOME="$demo_root/cache"
export XDG_STATE_HOME="$demo_root/state"
export TMPDIR="$demo_root/tmp"
export HERDR_SOCKET_PATH="$socket_path"
export PS1='demo ❯ '
export PROMPT_COMMAND=

cd "$demo_project"

herdr server >"$demo_root/server.log" 2>&1 &
sleep 1
herdr plugin link "$repo_root" --enabled >/dev/null

(
    sleep 4
    herdr tab create --workspace w1 --cwd "$demo_project" --focus >/dev/null
    herdr plugin action invoke herdr.pane-name.sync >/dev/null
    sleep 1
    herdr pane send-text w1:p2 $'vim -Nu NONE -n\n' >/dev/null
    sleep 2
    herdr plugin action invoke herdr.pane-name.sync >/dev/null
    sleep 1
    herdr pane split w1:p2 --direction right --focus >/dev/null
    herdr plugin action invoke herdr.pane-name.sync >/dev/null
    sleep 1
    herdr pane split w1:p3 --direction down --focus >/dev/null
    herdr plugin action invoke herdr.pane-name.sync >/dev/null
    herdr tab rename w1:t2 '1:custom' >/dev/null
    herdr plugin action invoke herdr.pane-name.sync >/dev/null
    sleep 0.5
    herdr pane send-text w1:p4 $'pico\n' >/dev/null
    sleep 2
    herdr plugin action invoke herdr.pane-name.sync >/dev/null
    sleep 1
    herdr pane focus --pane w1:p4 --direction up >/dev/null
    herdr plugin action invoke herdr.pane-name.sync >/dev/null
    sleep 1
    herdr pane focus --pane w1:p3 --direction left >/dev/null
    herdr plugin action invoke herdr.pane-name.sync >/dev/null
    sleep 1
    herdr pane focus --pane w1:p2 --direction right >/dev/null
    herdr plugin action invoke herdr.pane-name.sync >/dev/null
    sleep 1
    herdr pane focus --pane w1:p3 --direction down >/dev/null
    herdr plugin action invoke herdr.pane-name.sync >/dev/null
    sleep 1
    herdr tab close w1:t1 >/dev/null
    herdr plugin action invoke herdr.pane-name.sync >/dev/null
) >"$demo_root/controller.log" 2>&1 &
