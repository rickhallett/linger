//! Bounded static extraction of literal shell arguments from orchestration code.
//! This is lexical inspection, not JavaScript evaluation or shell detection.
#[derive(Clone, Debug)]
pub struct Literal {
    pub text: String,
    /// Decoded UTF-8 bytes mapped to the original JavaScript source.
    pub origin: Vec<(usize, usize)>,
}
#[derive(Debug)]
struct Token {
    text: String,
    literal: Option<Literal>,
}

pub fn is_orchestration(tool: &str) -> bool {
    matches!(tool, "exec" | "functions.exec")
}

fn string(source: &str, start: usize) -> Option<(Literal, usize, bool)> {
    let quote = source.as_bytes()[start] as char;
    let mut at = start + 1;
    let mut out = Literal {
        text: String::new(),
        origin: Vec::new(),
    };
    let mut constant = true;
    while at < source.len() {
        let c = source[at..].chars().next()?;
        if c == quote {
            return Some((out, at + 1, constant));
        }
        if quote == '`' && source[at..].starts_with("${") {
            // Nested template expressions need a real JS parser. Fail closed.
            return None;
        }
        let begin = at;
        at += c.len_utf8();
        let decoded = if c == '\\' {
            let escape = source[at..].chars().next()?;
            at += escape.len_utf8();
            match escape {
                'n' => "\n".into(),
                'r' => "\r".into(),
                't' => "\t".into(),
                'b' => "\u{8}".into(),
                'f' => "\u{c}".into(),
                'v' => "\u{b}".into(),
                '\n' => String::new(),
                '\r' => {
                    if source.as_bytes().get(at) == Some(&b'\n') {
                        at += 1;
                    }
                    String::new()
                }
                '\\' | '\'' | '"' | '`' | '$' => escape.to_string(),
                'x' | 'u' => {
                    let digits = if escape == 'x' { 2 } else { 4 };
                    let hex = source.get(at..at + digits)?;
                    let value = u32::from_str_radix(hex, 16).ok()?;
                    at += digits;
                    char::from_u32(value)?.to_string()
                }
                _ => {
                    constant = false;
                    escape.to_string()
                }
            }
        } else {
            if quote != '`' && matches!(c, '\n' | '\r') {
                return None;
            }
            c.to_string()
        };
        out.origin
            .extend(std::iter::repeat_n((begin, at), decoded.len()));
        out.text.push_str(&decoded);
    }
    None
}
fn tokens(source: &str) -> Option<Vec<Token>> {
    if source.len() > 128 * 1024 {
        return None;
    }
    let mut out = Vec::new();
    let mut at = 0;
    while at < source.len() {
        let c = source[at..].chars().next()?;
        if c.is_whitespace() {
            at += c.len_utf8();
            continue;
        }
        if source[at..].starts_with("//") {
            at += source[at..].find('\n').unwrap_or(source.len() - at);
            continue;
        }
        if source[at..].starts_with("/*") {
            at += 2 + source[at + 2..].find("*/")? + 2;
            continue;
        }
        if matches!(c, '\'' | '"' | '`') {
            let (literal, end, constant) = string(source, at)?;
            out.push(Token {
                text: literal.text.clone(),
                literal: constant.then_some(literal),
            });
            at = end;
            continue;
        }
        // A slash could introduce a regexp with fake call text. Do not guess.
        if c == '/' {
            return None;
        }
        let start = at;
        at += c.len_utf8();
        if c.is_alphanumeric() || c == '_' || c == '$' {
            while let Some(next) = source[at..].chars().next() {
                if !next.is_alphanumeric() && !matches!(next, '_' | '$') {
                    break;
                }
                at += next.len_utf8();
            }
        }
        out.push(Token {
            text: source[start..at].into(),
            literal: None,
        });
    }
    Some(out)
}
fn punct(t: &Token, text: &str) -> bool {
    t.literal.is_none() && t.text == text
}

pub fn commands(tool: &str, source: &str) -> Vec<Literal> {
    if !is_orchestration(tool) {
        return Vec::new();
    }
    let Some(tokens) = tokens(source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for n in 0..tokens.len().saturating_sub(5) {
        let prefix = &tokens[n..n + 6];
        if !punct(&prefix[0], "tools")
            || !punct(&prefix[1], ".")
            || !["exec_command", "shell_command"]
                .iter()
                .any(|s| punct(&prefix[2], s))
            || !punct(&prefix[3], "(")
            || !punct(&prefix[4], "{")
            || n > 0 && (punct(&tokens[n - 1], ".") || punct(&tokens[n - 1], "new"))
        {
            continue;
        }
        let mut i = n + 5;
        let mut command = None;
        let mut valid = true;
        let mut closed = false;
        // Only ordinary named object properties. Spreads/computed keys could
        // replace cmd at runtime, so reject the entire argument in that case.
        while i < tokens.len() {
            if punct(&tokens[i], "}") {
                closed = true;
                break;
            }
            let key = &tokens[i];
            if tokens.get(i + 1).is_none_or(|t| !punct(t, ":")) {
                valid = false;
                break;
            }
            let start = i + 2;
            i = start;
            let mut stack: Vec<&str> = Vec::new();
            while i < tokens.len() {
                let t = &tokens[i];
                if t.literal.is_none() {
                    let s = t.text.as_str();
                    if stack.is_empty() && matches!(s, "," | "}") {
                        break;
                    }
                    match s {
                        "(" => stack.push(")"),
                        "[" => stack.push("]"),
                        "{" => stack.push("}"),
                        ")" | "]" | "}" if stack.pop() != Some(s) => {
                            valid = false;
                            break;
                        }
                        _ => {}
                    }
                }
                i += 1;
            }
            if !valid {
                break;
            }
            if matches!(key.text.as_str(), "cmd" | "command") {
                if command.is_some() || i != start + 1 {
                    valid = false;
                    break;
                }
                command = tokens[start].literal.clone();
                if command.is_none() {
                    valid = false;
                    break;
                }
            }
            if tokens.get(i).is_some_and(|t| punct(t, ",")) {
                i += 1;
            }
        }
        if valid
            && closed
            && tokens.get(i + 1).is_some_and(|t| punct(t, ")"))
            && let Some(command) = command
        {
            out.push(command);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn literal_calls_preserve_escapes_offsets_and_source_order() {
        let source = r#"text(await tools.exec_command({cmd: 'rg -n "café" src', workdir: dir})); await tools.exec_command({"cmd": `git status --short\nrg -l x .`})"#;
        let c = commands("functions.exec", source);
        assert_eq!(c.len(), 2);
        assert_eq!(c[1].text, "git status --short\nrg -l x .");
        let pos = c[0].text.find("café").unwrap();
        assert_eq!(&source[c[0].origin[pos].0..c[0].origin[pos + 4].1], "café");
    }
    #[test]
    fn dynamic_arguments_comments_and_strings_are_not_shell_evidence() {
        for source in [
            r#"// tools.exec_command({cmd:'fake'})"#,
            r#"const s = "tools.exec_command({cmd:'fake'})""#,
            "tools.exec_command({cmd: command})",
            "tools.exec_command({cmd: 'rg ' + target})",
            "tools.exec_command({cmd: `rg ${target}`})",
            "tools.exec_command({cmd:'rg', ...opts})",
            "tools.exec_command({cmd:'rg', cmd:'git'})",
            "other.tools.exec_command({cmd:'rg'})",
            "tools.exec_command({['cmd']:'rg'})",
            "let re = /tools.exec_command({cmd:'fake'})/",
        ] {
            assert!(commands("exec", source).is_empty(), "{source}");
        }
        assert!(commands("python", "tools.exec_command({cmd:'rg'})").is_empty());
    }
}
