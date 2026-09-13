//! Bounded lexical decomposition. No evaluation, expansion or shell execution.
use super::structure;

/// Split common shell compositions while leaving quoted strings opaque. Reject
/// substitutions/groups/heredocs rather than treating their bodies as commands.
pub(super) fn segments(input: &str) -> Option<Vec<(&str, &'static str)>> {
    let mut quote = None;
    let mut escaped = false;
    let mut start = 0;
    let mut result = Vec::new();
    let mut chars = input.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' && quote != Some('\'') {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else if q == '"' && matches!(c, '$' | '`') {
                return None;
            }
            continue;
        }
        match c {
            '\'' | '"' => quote = Some(c),
            '$' | '`' | '(' | ')' | '{' | '}' => return None,
            '<' if chars.peek().is_some_and(|(_, c)| *c == '<') => return None,
            '#' if at == 0 || input[..at].ends_with(char::is_whitespace) => {
                if !input[start..at].trim().is_empty() {
                    result.push((&input[start..at], ""));
                }
                for (offset, next) in chars.by_ref() {
                    start = offset + next.len_utf8();
                    if next == '\n' {
                        break;
                    }
                }
                if chars.peek().is_none() {
                    start = input.len();
                }
            }
            '&' if input[..at].ends_with(['>', '<']) => {}
            '&' | '|' | ';' | '\n' => {
                let op = match c {
                    '&' if chars.peek().is_some_and(|(_, c)| *c == '&') => {
                        chars.next();
                        " && "
                    }
                    '&' => return None,
                    '|' if chars.peek().is_some_and(|(_, c)| *c == '|') => {
                        chars.next();
                        " || "
                    }
                    '|' => " | ",
                    _ => " ; ",
                };
                if input[start..at].trim().is_empty() {
                    return None;
                }
                result.push((&input[start..at], op));
                start = chars.peek().map_or(input.len(), |(at, _)| *at);
            }
            // Redirections remain words in this segment; constituent extraction
            // stops at them, so a destination cannot become a flag/subcommand.
            _ => {}
        }
    }
    if quote.is_some() || escaped {
        return None;
    }
    if !input[start..].trim().is_empty() {
        result.push((&input[start..], ""));
    }
    (!result.is_empty()).then_some(result)
}

pub(super) struct Component {
    pub label: String,
    pub words: Vec<usize>,
}
fn subcommand(program: &str, word: &str) -> bool {
    let allowed = match program.rsplit('/').next().unwrap_or(program) {
        "git" => {
            "status diff log show add commit checkout switch branch restore reset fetch pull push worktree rev-parse ls-files grep"
        }
        "cargo" => "check test build run fmt clippy doc add bench",
        "npm" | "pnpm" | "yarn" => "run install add test build exec dlx list",
        "uv" => "run sync add pip venv tool python",
        "docker" => "run build ps images exec compose logs inspect",
        "kubectl" => "get describe logs apply delete exec",
        "gh" => "pr issue repo run workflow auth",
        _ => return false,
    };
    allowed.split_whitespace().any(|w| w == word)
}

// Skip known option values even when they begin with '-'. Unknown option
// spellings remain lexical parts, not a claim about their argument semantics.
fn takes_value(program: &str, flag: &str) -> bool {
    let flags = match program.rsplit('/').next().unwrap_or(program) {
        "rg" | "grep" => {
            "-e -f -g -t -T -m -A -B -C -r --regexp --file --glob --iglob --type --type-not --max-count --after-context --before-context --context --replace --encoding --threads --sort --sortr"
        }
        "git" => "-C -c --git-dir --work-tree --format --pretty --author --since --until",
        "sed" => "-e -f --expression --file",
        "ssh" => "-o -p -i -L -R -D -F -J -l",
        "find" => "-name -iname -path -ipath -type -maxdepth -mindepth",
        "curl" => "-H -d -o -X -u --header --data --output --request --user",
        "head" | "tail" => "-n -c --lines --bytes",
        _ => "",
    };
    flags.split_whitespace().any(|f| f == flag)
}

pub(super) fn components(words: &[String]) -> Vec<Component> {
    let Some(program) = words.first() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut context = program.clone();
    let mut prefix = vec![0];
    let mut from = 1;
    if words.get(1).is_some_and(|w| subcommand(program, w)) {
        context.push(' ');
        context.push_str(&words[1]);
        prefix.push(1);
        from = 2;
        out.push(Component {
            label: context.clone(),
            words: prefix.clone(),
        });
    }
    let opaque = structure::is_inline(words, &words.join(" "));
    let mut flags = Vec::new();
    let mut value_next = false;
    for (i, word) in words.iter().enumerate().skip(from).take(64) {
        if value_next {
            value_next = false;
            continue;
        }
        if word == "--" || word.contains(['<', '>']) {
            break;
        }
        if !word.starts_with('-') || word == "-" {
            if opaque {
                break;
            }
            continue;
        }
        let flag = word.split('=').next().unwrap();
        value_next = !word.contains('=') && takes_value(program, flag);
        if !flag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
        {
            continue;
        }
        let mut indices = prefix.clone();
        indices.push(i);
        out.push(Component {
            label: format!("{context} {flag}"),
            words: indices.clone(),
        });
        flags.push((flag.to_string(), i));
        if program.rsplit('/').next() == Some("ssh")
            && flag == "-o"
            && let Some(value) = words
                .get(i + 1)
                .filter(|w| w.contains('=') && !w.starts_with('-'))
        {
            indices.push(i + 1);
            out.push(Component {
                label: format!("{context} -o {value}"),
                words: indices,
            });
        }
        if opaque {
            break;
        }
    }
    if flags.len() > 1 {
        let mut indices = prefix;
        indices.extend(flags.iter().map(|(_, i)| *i));
        out.push(Component {
            label: format!(
                "{} {}",
                context,
                flags
                    .iter()
                    .map(|(f, _)| f.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            words: indices,
        });
    }
    out
}
