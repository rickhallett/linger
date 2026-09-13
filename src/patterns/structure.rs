//! Structural frequency projections over retained inputs, never execution.
use super::{Pattern, normalize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Level {
    #[default]
    Combinations,
    Programs,
    Wrappers,
}
impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Self::Combinations => "Combinations",
            Self::Programs => "Programs",
            Self::Wrappers => "Wrappers",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Combinations => Self::Programs,
            Self::Programs => Self::Wrappers,
            Self::Wrappers => Self::Combinations,
        }
    }
}

#[derive(Clone)]
pub struct Projection {
    pub key: String,
    pub label: String,
    pub level: Level,
    pub inline: bool,
}
fn projected(label: String, level: Level, inline: bool) -> Projection {
    Projection {
        key: normalize::key(&serde_json::json!(["structure-v1", level.label(), label])),
        label,
        level,
        inline,
    }
}
fn executable(word: &str) -> bool {
    !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-+".contains(c))
        && !word.starts_with('-')
}
fn shell(word: &str) -> bool {
    matches!(
        word.rsplit('/').next().unwrap_or(word),
        "sh" | "bash" | "zsh" | "dash" | "ksh"
    )
}
pub(crate) fn wrapper(words: &[String]) -> Option<(String, &str)> {
    if !shell(words.first()?) || words.len() < 3 {
        return None;
    }
    let flags = &words[1..words.len() - 1];
    if !matches!(
        flags.last()?.as_str(),
        "-c" | "-lc" | "-ic" | "-lic" | "-ilc"
    ) || !flags[..flags.len() - 1]
        .iter()
        .all(|f| matches!(f.as_str(), "-l" | "-i"))
    {
        return None;
    }
    Some((words[..words.len() - 1].join(" "), words.last()?.as_str()))
}
fn inline(words: &[String], command: &str) -> bool {
    let Some(program) = words.first() else {
        return false;
    };
    let name = program.rsplit('/').next().unwrap_or(program);
    let python = name == "python"
        || name
            .strip_prefix("python")
            .is_some_and(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit() || c == '.'));
    (python
        && words[1..]
            .iter()
            .any(|w| w == "-c" || w.starts_with("-c") || w == "-"))
        || (python && command.contains("<<"))
        || (matches!(name, "node" | "nodejs" | "ruby" | "perl" | "Rscript")
            && words[1..]
                .iter()
                .any(|w| matches!(w.as_str(), "-e" | "--eval" | "-p") || w.starts_with("--eval=")))
}

/// Count each structural form at most once per tool call. A shell wrapper and
/// its inner program can coexist; these counts must not be summed as calls.
pub fn projections(pattern: &Pattern) -> Vec<Projection> {
    let mut out = vec![Projection {
        key: pattern.key.clone(),
        label: pattern.label.clone(),
        level: Level::Combinations,
        inline: matches!(pattern.tool.as_str(), "functions.exec" | "exec" | "python"),
    }];
    if out[0].inline {
        return out;
    }
    // Structured argv is retained as JSON by the original exact-input index.
    let value = serde_json::from_str::<serde_json::Value>(&pattern.command).ok();
    let argv = value
        .as_ref()
        .and_then(|v| v.get("cmd").or_else(|| v.get("command")))
        .and_then(|v| v.as_array())
        .and_then(|a| {
            a.iter()
                .map(|v| v.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        });
    if let Some(argv) = argv {
        analyze_words(&argv, &pattern.command, &mut out, 0);
    } else {
        analyze_command(&pattern.command, &mut out, 0);
    }
    let hidden = out.iter().any(|p| p.inline);
    out[0].inline |= hidden;
    // Unique inline bodies are hidden, but reusable interpreter/wrapper rows
    // remain useful even when their occurrences contain unique scripts.
    let mut unique = BTreeMap::new();
    for p in out {
        unique.entry(p.key.clone()).or_insert(p);
    }
    unique.into_values().collect()
}
fn analyze_command(command: &str, out: &mut Vec<Projection>, depth: usize) {
    if depth > 4 || command.len() > 32 * 1024 {
        return;
    }
    if let Some(segments) = normalize::segments(command) {
        let segment_depth = depth + usize::from(segments.len() > 1);
        for (segment, _) in segments {
            if let Ok(words) = shell_words::split(segment) {
                analyze_words(&words, segment, out, segment_depth);
            }
        }
    } else if let Ok(words) = shell_words::split(command.lines().next().unwrap_or(command)) {
        // Unsupported shell grammar: only classify an explicit leading
        // interpreter, never mine words from a heredoc/program body.
        if let Some(first) = words.first().filter(|w| executable(w)) {
            out.push(projected(first.clone(), Level::Programs, false));
            if inline(&words, command) {
                out[0].inline = true;
            }
        }
    }
}
fn analyze_words(words: &[String], command: &str, out: &mut Vec<Projection>, depth: usize) {
    let Some(program) = words.first().filter(|w| executable(w)) else {
        return;
    };
    out.push(projected(program.clone(), Level::Programs, false));
    if let Some((label, body)) = wrapper(words) {
        out.push(projected(label.clone(), Level::Wrappers, false));
        // Preserve both the full wrapper+command form and inner command forms.
        if let Some(inner) =
            normalize::pattern("Bash", &serde_json::json!({"command": body}).to_string())
            && !inner.label.starts_with("exact command")
        {
            out.push(projected(
                format!("{label} → {}", inner.label),
                Level::Combinations,
                false,
            ));
            // The opaque outer argv isn't a useful duplicate ranking row.
            if depth == 0 {
                out[0] = projected(
                    format!("{label} → {}", inner.label),
                    Level::Combinations,
                    false,
                );
            }
        }
        analyze_command(body, out, depth + 1);
    } else if inline(words, command) {
        out[0].inline = true;
    } else if let Some(shape) = normalize::usage(&shell_words::join(words)) {
        let p = normalize::pattern(
            "Bash",
            &serde_json::json!({"command": shell_words::join(words)}).to_string(),
        )
        .unwrap();
        out.push(Projection {
            key: p.key,
            label: shape,
            level: Level::Combinations,
            inline: false,
        });
    }
}
