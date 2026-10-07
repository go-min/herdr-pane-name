use crate::{config::Config, icons};

pub fn for_pane(
    cfg: &Config,
    pane: &serde_json::Value,
    process: &str,
    argv: &[String],
    position: usize,
) -> String {
    if cfg.terminal_titles {
        if let Some(title) = pane
            .get("terminal_title_stripped")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|title| !title.is_empty())
        {
            return prefixed(cfg, title, position);
        }
    }
    automatic(cfg, process, argv, position)
}

pub fn automatic(cfg: &Config, process: &str, argv: &[String], position: usize) -> String {
    let icon = if cfg.icons {
        icons::for_program(process)
    } else {
        None
    };
    let base = match icon {
        Some(icon) => format!("{icon} {process}"),
        None => process.to_owned(),
    };
    let args = if cfg.show_args {
        argv.iter()
            .skip(1)
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        String::new()
    };
    let text = if args.is_empty() {
        base
    } else {
        format!("{base} {args}")
    };
    prefixed(cfg, &text, position)
}

pub fn prefixed(cfg: &Config, base: &str, position: usize) -> String {
    let prefix = if cfg.prefixes && position < 9 {
        format!("{}:", position + 1)
    } else {
        String::new()
    };
    truncate(&format!("{prefix}{base}"), cfg.max_length)
}

pub fn truncate(value: &str, max_length: usize) -> String {
    value.chars().take(max_length.max(1)).collect()
}

pub fn has_prefix(value: &str) -> bool {
    value.len() > 2 && value.as_bytes()[0].is_ascii_digit() && value.as_bytes()[1] == b':'
}

pub fn without_prefix(value: &str) -> &str {
    let mut value = value;
    while has_prefix(value) {
        value = &value[2..];
    }
    value
}

pub fn is_default_label(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('[') else {
        return false;
    };
    let Some((number, _)) = rest.split_once("] ") else {
        return false;
    };
    !number.is_empty() && number.chars().all(|character| character.is_ascii_digit())
}

pub fn is_numeric_label(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
}

pub fn is_ignored(cfg: &Config, process: &str) -> bool {
    cfg.ignored_programs
        .iter()
        .any(|ignored| ignored == process)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_icons_and_arguments_are_applied_before_prefixing() {
        let cfg = Config {
            show_args: true,
            icons: true,
            ..Config::default()
        };

        assert_eq!(
            automatic(&cfg, "nvim", &["nvim".into(), "file.rs".into()], 0),
            "1: nvim file.rs"
        );
    }

    #[test]
    fn icons_are_disabled_when_icons_are_disabled() {
        let cfg = Config {
            icons: false,
            ..Config::default()
        };

        assert_eq!(automatic(&cfg, "nvim", &[], 0), "1:nvim");
    }

    #[test]
    fn built_in_icons_work_without_configuration_overrides() {
        let cfg = Config::default();

        assert_eq!(automatic(&cfg, "docker", &[], 0), "1: docker");
    }

    #[test]
    fn prefix_helpers_handle_multiple_prefixes() {
        assert!(has_prefix("1:review"));
        assert_eq!(without_prefix("2:1:review"), "review");
        assert!(!has_prefix("10:review"));
    }

    #[test]
    fn truncation_is_unicode_safe_and_has_a_minimum_length() {
        assert_eq!(truncate(" review", 3), " r");
        assert_eq!(truncate("", 0), "");
    }
}

#[cfg(test)]
mod title_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn titles_are_opt_in_and_empty_titles_fall_back_to_process() {
        let pane = json!({"terminal_title_stripped":"  auth 🔧  "});
        let cfg = Config {
            icons: false,
            ..Config::default()
        };
        assert_eq!(for_pane(&cfg, &pane, "nvim", &[], 0), "1:nvim");
        let cfg = Config {
            terminal_titles: true,
            ..cfg
        };
        assert_eq!(for_pane(&cfg, &pane, "nvim", &[], 0), "1:auth 🔧");
        assert_eq!(
            for_pane(
                &cfg,
                &json!({"terminal_title_stripped":" "}),
                "nvim",
                &[],
                0
            ),
            "1:nvim"
        );
    }
}
