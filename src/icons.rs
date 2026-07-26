/// Nerd Font glyphs for common shells, editors, runtimes, CLIs, and tools.
/// Users can override any entry through `[aliases]`.
pub fn for_program(program: &str) -> Option<&'static str> {
    Some(match program {
        "bash" => "",
        "zsh" => "",
        "fish" => "",
        "pwsh" | "powershell" => "",
        "nu" => "",
        "nvim" => "",
        "neovim" => "",
        "vim" => "",
        "emacs" => "",
        "helix" => "",
        "code" | "code-insiders" | "cursor" => "",
        "git" | "lazygit" | "hunk" => "",
        "gh" => "",
        "docker" | "podman" => "",
        "kubectl" | "k9s" | "helm" => "󱃾",
        "terraform" | "tofu" => "",
        "ansible" => "",
        "vagrant" => "",
        "cargo" | "rustc" | "rustup" => "",
        "npm" | "npx" => "",
        "node" => "",
        "yarn" => "",
        "pnpm" => "",
        "bun" => "",
        "deno" => "",
        "python" | "python3" | "pip" | "pip3" | "uv" | "poetry" => "",
        "go" => "",
        "java" | "javac" => "",
        "gradle" => "",
        "mvn" => "",
        "ruby" | "bundle" | "rails" => "",
        "php" | "composer" => "",
        "perl" => "",
        "lua" | "luarocks" => "",
        "swift" => "",
        "dotnet" => "",
        "zig" => "",
        "gcc" | "g++" | "clang" => "",
        "make" | "cmake" | "meson" => "",
        "nix" | "nix-shell" => "",
        "brew" => "",
        "atuin" => "󰒋",
        "bat" => "󰭟",
        "delta" => "",
        "direnv" => "󰆍",
        "doggo" | "gping" | "trippy" => "󰖟",
        "dust" => "󰋊",
        "eza" | "tree" | "zoxide" => "󰉋",
        "fd" | "procs" => "󰍛",
        "ffmpeg" => "",
        "hyperfine" => "󰔚",
        "llvm" => "",
        "shellcheck" => "󰅚",
        "starship" => "",
        "tree-sitter" => "󰙅",
        "ttyd" => "󰆍",
        "vhs" => "",
        "watchexec" => "󰁪",
        "ssh" | "scp" => "󰣀",
        "curl" | "wget" => "󰖟",
        "tmux" | "screen" | "zellij" | "herdr" => "",
        "yazi" | "lf" | "ranger" | "superfile" | "spf" => "󰉋",
        "fzf" | "rg" | "grep" | "ag" => "",
        "jq" | "yq" => "",
        "top" | "htop" | "btop" => "󰍛",
        "psql" | "mysql" | "sqlite3" | "mongosh" => "",
        "redis-cli" => "",
        "nginx" => "",
        "ollama" | "claude" | "codex" | "antigravity" | "agy" => "󰚩",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::for_program;

    #[test]
    fn covers_common_tools_and_leaves_unknown_programs_unchanged() {
        assert_eq!(for_program("nvim"), Some(""));
        assert_eq!(for_program("zsh"), Some(""));
        assert_eq!(for_program("kubectl"), Some("󱃾"));
        assert_eq!(for_program("eza"), Some("󰉋"));
        assert_eq!(for_program("spf"), Some("󰉋"));
        assert_eq!(for_program("nginx"), Some(""));
        assert_eq!(for_program("hunk"), Some(""));
        assert_eq!(for_program("agy"), Some("󰚩"));
        assert_eq!(for_program("herdr"), Some(""));
        assert_eq!(for_program("my-custom-tool"), None);
    }
}
