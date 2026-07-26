autoload -Uz add-zsh-hook
_herdr_pane_name_run() {
  if [[ -z ${HERDR_PLUGIN_CONFIG_DIR:-} ]]; then
    HERDR_PLUGIN_CONFIG_DIR="$(herdr plugin config-dir herdr.pane-name 2>/dev/null)"
    export HERDR_PLUGIN_CONFIG_DIR
  fi
  "${HERDR_PANE_NAME_BIN:-herdr-pane-name}" hook >/dev/null 2>&1 &!
}
_herdr_pane_name_preexec() {
  (sleep 0.2; _herdr_pane_name_run) &!
}
_herdr_pane_name_precmd() {
  _herdr_pane_name_run
}
add-zsh-hook preexec _herdr_pane_name_preexec
add-zsh-hook precmd _herdr_pane_name_precmd
