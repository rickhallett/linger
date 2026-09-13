//! Map structural forms back to their original recorded characters. Decoded
//! shell/JSON strings carry byte provenance; never locate them by text search.
use super::{Level, Pattern, normalize, structure};
use std::collections::BTreeMap;

type Ranges = Vec<(usize, usize)>;
#[derive(Clone)]
struct Mapped {
    text: String,
    // Each decoded UTF-8 byte points to the source bytes that produced it.
    origin: Ranges,
}
impl Mapped {
    fn raw(text: &str) -> Self {
        Self {
            text: text.into(),
            origin: (0..text.len()).map(|i| (i, i + 1)).collect(),
        }
    }
    fn slice(&self, start: usize, end: usize) -> Self {
        Self {
            text: self.text[start..end].into(),
            origin: self.origin[start..end].to_vec(),
        }
    }
    fn range(&self, start: usize, end: usize) -> Option<(usize, usize)> {
        Some((
            self.origin.get(start)?.0,
            self.origin.get(end.checked_sub(1)?)?.1,
        ))
    }
    fn push(&mut self, text: &str, origin: (usize, usize)) {
        self.text.push_str(text);
        self.origin.extend(std::iter::repeat_n(origin, text.len()));
    }
}
struct Word {
    value: Mapped,
    range: (usize, usize),
}

fn shell_words(input: &Mapped) -> Option<Vec<Word>> {
    let mut result = Vec::new();
    let mut chars = input.text.char_indices().peekable();
    while let Some(&(start, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        let mut decoded = Mapped {
            text: String::new(),
            origin: Vec::new(),
        };
        let mut quote = None;
        let mut end = start;
        while let Some(&(at, c)) = chars.peek() {
            if quote.is_none() && c.is_whitespace() {
                break;
            }
            chars.next();
            end = at + c.len_utf8();
            if matches!(c, '\'' | '"') && (quote.is_none() || quote == Some(c)) {
                quote = if quote.is_none() { Some(c) } else { None };
                continue;
            }
            if c == '\\' && quote != Some('\'') {
                let &(next_at, next) = chars.peek()?;
                if quote != Some('"') || matches!(next, '$' | '`' | '"' | '\\' | '\n') {
                    chars.next();
                    end = next_at + next.len_utf8();
                    if next != '\n' {
                        decoded.push(&next.to_string(), input.range(at, end)?);
                    }
                    continue;
                }
            }
            decoded.push(&c.to_string(), input.range(at, end)?);
        }
        if quote.is_some() {
            return None;
        }
        // Match the canonical decoder before trusting our offset mapping.
        let expected = shell_words::split(&input.text[start..end]).ok()?;
        if expected.len() != 1 || expected[0] != decoded.text {
            return None;
        }
        result.push(Word {
            value: decoded,
            range: input.range(start, end)?,
        });
    }
    Some(result)
}

fn json_string(source: &str, start: usize) -> Option<(Mapped, usize)> {
    if source.as_bytes().get(start) != Some(&b'"') {
        return None;
    }
    let mut at = start + 1;
    let mut value = Mapped {
        text: String::new(),
        origin: Vec::new(),
    };
    while at < source.len() {
        let c = source[at..].chars().next()?;
        if c == '"' {
            return Some((value, at + 1));
        }
        let mut end = at + c.len_utf8();
        if c == '\\' {
            end = at
                + if source.as_bytes().get(at + 1) == Some(&b'u') {
                    6
                } else {
                    2
                };
            let fragment = source.get(at..end)?;
            let mut decoded = serde_json::from_str::<String>(&format!("\"{fragment}\""));
            if decoded.is_err() && fragment.starts_with("\\u") {
                end += 6; // a UTF-16 surrogate pair
                decoded = serde_json::from_str::<String>(&format!("\"{}\"", source.get(at..end)?));
            }
            value.push(&decoded.ok()?, (at, end));
        } else {
            value.push(&c.to_string(), (at, end));
        }
        at = end;
    }
    None
}
fn argv(source: &str) -> Option<Vec<Word>> {
    let value: serde_json::Value = serde_json::from_str(source).ok()?;
    let key = if value.get("cmd").is_some() {
        "cmd"
    } else {
        "command"
    };
    value.get(key)?.as_array()?;
    let bytes = source.as_bytes();
    let mut depth = 0;
    let mut at = 0;
    let mut candidate = None;
    while at < bytes.len() {
        match bytes[at] {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            b'"' => {
                let (name, end) = json_string(source, at)?;
                at = end;
                if depth == 1 && name.text == key {
                    while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
                        at += 1;
                    }
                    if bytes.get(at) != Some(&b':') {
                        continue;
                    }
                    at += 1;
                    while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
                        at += 1;
                    }
                    if bytes.get(at) != Some(&b'[') {
                        return None;
                    }
                    at += 1;
                    let mut words = Vec::new();
                    loop {
                        while bytes
                            .get(at)
                            .is_some_and(|b| b.is_ascii_whitespace() || *b == b',')
                        {
                            at += 1;
                        }
                        if bytes.get(at) == Some(&b']') {
                            candidate = Some(words);
                            at += 1;
                            break;
                        }
                        let start = at;
                        let (value, end) = json_string(source, at)?;
                        words.push(Word {
                            value,
                            range: (start, end),
                        });
                        at = end;
                    }
                }
                continue;
            }
            _ => {}
        }
        at += 1;
    }
    let words = candidate?;
    let decoded: Vec<_> = words.iter().map(|w| w.value.text.as_str()).collect();
    let expected: Vec<_> = value
        .get(key)?
        .as_array()?
        .iter()
        .map(|v| v.as_str())
        .collect::<Option<_>>()?;
    (decoded == expected).then_some(words)
}
fn key(label: &str, level: Level) -> String {
    normalize::key(&serde_json::json!(["structure-v1", level.label(), label]))
}
fn add(
    out: &mut BTreeMap<String, Ranges>,
    key: String,
    ranges: impl IntoIterator<Item = (usize, usize)>,
) {
    out.entry(key).or_default().extend(ranges);
}
fn walk_words(words: &[Word], out: &mut BTreeMap<String, Ranges>, depth: usize) {
    let Some(first) = words.first() else { return };
    add(out, key(&first.value.text, Level::Programs), [first.range]);
    let decoded: Vec<_> = words.iter().map(|w| w.value.text.clone()).collect();
    if let Some((label, body)) = structure::wrapper(&decoded) {
        add(
            out,
            key(&label, Level::Wrappers),
            words[..words.len() - 1].iter().map(|w| w.range),
        );
        if let Some(inner) =
            normalize::pattern("Bash", &serde_json::json!({"command": body}).to_string())
            && !inner.label.starts_with("exact command")
        {
            add(
                out,
                key(&format!("{label} → {}", inner.label), Level::Combinations),
                words.iter().map(|w| w.range),
            );
        }
        walk(&words.last().unwrap().value, out, depth + 1);
    } else if let Some(pattern) = normalize::pattern(
        "Bash",
        &serde_json::json!({"command": shell_words::join(&decoded)}).to_string(),
    ) && !pattern.label.starts_with("exact command")
    {
        add(out, pattern.key, words.iter().map(|w| w.range));
    }
}
fn walk(input: &Mapped, out: &mut BTreeMap<String, Ranges>, depth: usize) {
    if depth > 4 || input.text.len() > 32 * 1024 {
        return;
    }
    if let Some(segments) = normalize::segments(&input.text) {
        for (segment, _) in segments {
            let start = segment.as_ptr() as usize - input.text.as_ptr() as usize;
            if let Some(words) = shell_words(&input.slice(start, start + segment.len())) {
                walk_words(&words, out, depth);
            }
        }
    } else {
        let line = input.text.lines().next().unwrap_or(&input.text);
        if let Some(words) = shell_words(&input.slice(0, line.len()))
            && let Some(first) = words.first()
        {
            add(out, key(&first.value.text, Level::Programs), [first.range]);
        }
    }
}

pub fn ranges(pattern: &Pattern, selected: &str) -> Ranges {
    if selected == pattern.key {
        return vec![(0, pattern.command.len())];
    }
    let mut out = BTreeMap::new();
    if let Some(words) = argv(&pattern.command) {
        walk_words(&words, &mut out, 0);
    } else {
        walk(&Mapped::raw(&pattern.command), &mut out, 0);
    }
    let mut ranges = out.remove(selected).unwrap_or_default();
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    fn selected(input: &str, label: &str, level: Level) -> (Pattern, String) {
        let p = super::super::pattern("Bash", input).unwrap();
        let key = super::super::projections(&p)
            .into_iter()
            .find(|p| p.label == label && p.level == level)
            .unwrap()
            .key;
        (p, key)
    }
    #[test]
    fn wrapper_and_inner_program_map_to_original_json_not_other_fields() {
        let input =
            r#"{"cwd":"/bin/zsh/rg","command":["/bin/zsh","-lc","rg -n \\\"café\\\" src"]}"#;
        let (p, k) = selected(input, "/bin/zsh -lc", Level::Wrappers);
        let parts: Vec<_> = ranges(&p, &k)
            .iter()
            .map(|&(s, e)| &p.command[s..e])
            .collect();
        assert_eq!(parts, ["\"/bin/zsh\"", "\"-lc\""]);
        let (p, k) = selected(input, "rg", Level::Programs);
        let spans = ranges(&p, &k);
        assert_eq!(spans.len(), 1);
        assert_eq!(&p.command[spans[0].0..spans[0].1], "rg");
        assert!(spans[0].0 > input.find("-lc").unwrap());
    }
    #[test]
    fn repeated_programs_highlight_commands_not_pattern_literals() {
        let (p, k) = selected(
            r#"{"command":"rg -n rg src && rg -n bar tests"}"#,
            "rg",
            Level::Programs,
        );
        assert_eq!(ranges(&p, &k), [(0, 2), (16, 18)]);
    }
    #[test]
    fn escaped_unicode_remains_aligned_inside_a_shell_string() {
        let input = r#"{"command":["/bin/zsh","-lc","rg -n \"caf\u00e9\" src"]}"#;
        let (p, k) = selected(input, "rg -n <pattern> <path>", Level::Combinations);
        let spans = ranges(&p, &k);
        assert!(
            spans
                .iter()
                .any(|&(s, e)| &p.command[s..e] == r#"\"caf\u00e9\""#)
        );
        assert!(
            spans
                .iter()
                .all(|&(s, e)| p.command.is_char_boundary(s) && p.command.is_char_boundary(e))
        );
    }
}
