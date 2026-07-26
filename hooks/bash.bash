if [[ -z ${HERDR_PANE_NAME_BASH_HOOK:-} ]]; then
  HERDR_PANE_NAME_BASH_HOOK=1
  _herdr_pane_name_bash_run() {
    if [[ -z ${HERDR_PLUGIN_CONFIG_DIR:-} ]]; then
      HERDR_PLUGIN_CONFIG_DIR="$(herdr plugin config-dir herdr.pane-name 2>/dev/null)"
      export HERDR_PLUGIN_CONFIG_DIR
    fi
    if [[ -n ${HERDR_PANE_NAME_BIN:-} ]]; then
      "$HERDR_PANE_NAME_BIN" hook >/dev/null 2>&1 &
    elif command -v herdr-pane-name >/dev/null 2>&1; then
      herdr-pane-name hook >/dev/null 2>&1 &
    else
      herdr plugin action invoke herdr.pane-name.sync >/dev/null 2>&1 &
    fi
    disown "$!" 2>/dev/null || true
  }
  _herdr_pane_name_bash_preexec() {
    [[ $BASH_COMMAND == _herdr_pane_name_bash_* ]] && return 0
    (trap - DEBUG; sleep 0.2; _herdr_pane_name_bash_run) &
    disown "$!" 2>/dev/null || true
  }
  _herdr_pane_name_bash_postcmd() {
    _herdr_pane_name_bash_run
  }
  trap '_herdr_pane_name_bash_preexec' DEBUG
  PROMPT_COMMAND="_herdr_pane_name_bash_postcmd${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
fi
