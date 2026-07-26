Set Shell "bash"
Set FontFamily "FiraCode Nerd Font"
Set FontSize 17
Set Width 1280
Set Height 560
Set Padding 18
Set TypingSpeed 40ms
Output .github/assets/demo/demo-light.gif
Set Theme "Github"

Env HERDR_DEMO_THEME "catppuccin-latte"
Env XDG_CONFIG_HOME "/private/tmp/herdr-pane-name-vhs"
Env HERDR_SOCKET_PATH "/private/tmp/herdr-pane-name-vhs.sock"

Hide
Type "source .github/assets/demo/setup.sh" Enter
Sleep 2s
Show
Type "herdr" Enter
Sleep 18s
