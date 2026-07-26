function _herdr_pane_name_fish_run
    if not set -q HERDR_PLUGIN_CONFIG_DIR
        set -gx HERDR_PLUGIN_CONFIG_DIR (herdr plugin config-dir herdr.pane-name 2>/dev/null)
    end
    set -l bin herdr-pane-name
    if set -q HERDR_PANE_NAME_BIN
        set bin $HERDR_PANE_NAME_BIN
    end
    command $bin hook >/dev/null 2>&1 &
end
function _herdr_pane_name_fish_preexec --on-event fish_preexec
    begin
        sleep 0.2
        _herdr_pane_name_fish_run
    end &
end
function _herdr_pane_name_fish_hook --on-event fish_postexec
    _herdr_pane_name_fish_run
end
