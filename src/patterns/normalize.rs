use super::Pattern;
use sha2::{Digest, Sha256};

pub fn pattern(tool: &str, input: &str) -> Option<Pattern> {
    let shell = matches!(
        tool,
        "Bash"
            | "shell"
            | "exec_command"
            | "shell_command"
            | "functions.exec_command"
            | "functions.shell_command"
    );
    let value = serde_json::from_str::<serde_json::Value>(input).ok();
    let command = if shell {
        let v = value.as_ref()?;
        let command = v.get("cmd").or_else(|| v.get("command"))?;
        if let Some(s) = command.as_str() {
            s.to_string()
        } else if command.is_array() {
            return Some(exact(tool, input, "structured shell input"));
        } else {
            return None;
        }
    } else if matches!(tool, "functions.exec" | "exec" | "python") {
        return Some(exact(tool, input, "inline code"));
    } else {
        return None;
    };
    // Never erase heredoc bodies, code, expansions, redirects or unsupported grammar.
    if let Some(parts) = segments(&command) {
        let mut label = String::new();
        for (segment, separator) in parts {
            if let Some(shape) = usage(segment) {
                label.push_str(&shape);
                label.push_str(separator);
            } else {
                return Some(exact(tool, &command, "exact command"));
            }
        }
        let key = key(&serde_json::json!(["usage-v1", label]));
        return Some(Pattern {
            key,
            label,
            command,
            tool: tool.into(),
        });
    }
    Some(exact(tool, &command, "exact command"))
}
fn exact(tool: &str, command: &str, kind: &str) -> Pattern {
    let first = command
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(tool);
    Pattern {
        key: key(&serde_json::json!(["exact-v1", tool, command])),
        label: format!("{kind} · {}", first.chars().take(100).collect::<String>()),
        command: command.into(),
        tool: tool.into(),
    }
}

// Split only unquoted composition operators. Reject grammar we don't model;
// quote removal itself is delegated to shell-words. Quoted literals stay opaque.
fn segments(s: &str) -> Option<Vec<(&str, &'static str)>> {
    let mut quote = None;
    let mut escape = false;
    let mut start = 0;
    let mut out = Vec::new();
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' && quote != Some('\'') {
            escape = true;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None
            } else if q == '"' && matches!(c, '$' | '`') {
                return None;
            }
            continue;
        }
        match c {
            '\'' | '"' => quote = Some(c),
            '$' | '`' | '<' | '>' | '(' | ')' | '{' | '}' | '[' | ']' | '#' | '*' | '?' | '~' => {
                return None;
            }
            '&' | '|' | ';' | '\n' => {
                let separator = match c {
                    '&' => {
                        if chars.peek().map(|(_, c)| *c) != Some('&') {
                            return None;
                        }
                        chars.next();
                        " && "
                    }
                    '|' => {
                        if chars.peek().map(|(_, c)| *c) == Some('|') {
                            chars.next();
                            " || "
                        } else {
                            " | "
                        }
                    }
                    ';' => " ; ",
                    _ => " ; ",
                };
                if s[start..i].trim().is_empty() {
                    return None;
                }
                out.push((&s[start..i], separator));
                start = chars.peek().map_or(s.len(), |(i, _)| *i);
            }
            _ => {}
        }
    }
    if quote.is_some() || escape || s[start..].trim().is_empty() {
        return None;
    }
    out.push((&s[start..], ""));
    Some(out)
}
fn usage(s: &str) -> Option<String> {
    let words = shell_words::split(s).ok()?;
    let exe = words.first()?.as_str();
    let args = &words[1..];
    match exe {
        "git" => {
            if args.first().map(String::as_str) != Some("status") {
                return None;
            }
            if args[1..].iter().all(|a| {
                matches!(
                    a.as_str(),
                    "--short"
                        | "-s"
                        | "--branch"
                        | "-b"
                        | "--porcelain"
                        | "--porcelain=v1"
                        | "--porcelain=v2"
                        | "--untracked-files"
                        | "--untracked-files=no"
                        | "--untracked-files=normal"
                        | "--untracked-files=all"
                        | "-sb"
                        | "-bs"
                )
            }) {
                Some(words.join(" "))
            } else {
                None
            }
        }
        "rg" => {
            let mut out = vec!["rg".into()];
            let mut operands = 0;
            let mut files = false;
            let mut options = true;
            for a in args {
                if options && a == "--" {
                    options = false;
                    out.push(a.clone());
                } else if options && a.starts_with('-') {
                    if !matches!(
                        a.as_str(),
                        "-n" | "--line-number"
                            | "-i"
                            | "--ignore-case"
                            | "-l"
                            | "--files-with-matches"
                            | "-F"
                            | "--fixed-strings"
                            | "--files"
                            | "--hidden"
                            | "--no-ignore"
                            | "-c"
                            | "--count"
                            | "-q"
                            | "--quiet"
                            | "-w"
                            | "--word-regexp"
                    ) {
                        return None;
                    }
                    files |= a == "--files";
                    out.push(a.clone());
                } else {
                    out.push(
                        if operands == 0 && !files {
                            "<pattern>"
                        } else {
                            "<path>"
                        }
                        .into(),
                    );
                    operands += 1;
                }
            }
            if operands == 0 && !files {
                None
            } else {
                Some(out.join(" "))
            }
        }
        "sed" => {
            if args.len() < 2 || args[0] != "-n" {
                return None;
            }
            let address = args[1].strip_suffix('p')?;
            let numbers: Vec<_> = address.split(',').collect();
            if numbers.is_empty()
                || numbers.len() > 2
                || !numbers
                    .iter()
                    .all(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                || args[2..].iter().any(|a| a.starts_with('-'))
            {
                return None;
            }
            Some(format!(
                "sed -n '{}p'{}",
                if numbers.len() == 1 {
                    "<line>"
                } else {
                    "<start>,<end>"
                },
                " <path>".repeat(args.len() - 2)
            ))
        }
        "ssh" => {
            let mut out = vec!["ssh".into()];
            let mut i = 0;
            let mut host = false;
            while i < args.len() {
                let a = &args[i];
                if !host && a == "-o" {
                    let value = args.get(i + 1)?;
                    if !value.contains('=') {
                        return None;
                    }
                    out.push("-o".into());
                    out.push(value.clone());
                    i += 2;
                } else if !host && matches!(a.as_str(), "-T" | "-t" | "-N" | "-v" | "-vv" | "-vvv")
                {
                    out.push(a.clone());
                    i += 1;
                } else if !host && !a.starts_with('-') {
                    out.push("<host>".into());
                    host = true;
                    i += 1;
                } else {
                    return None;
                } // Remote commands retain their exact syntax.
            }
            host.then(|| out.join(" "))
        }
        _ => None,
    }
}

fn key(value: &serde_json::Value) -> String {
    format!("v1:{:x}", Sha256::digest(value.to_string().as_bytes()))
}
