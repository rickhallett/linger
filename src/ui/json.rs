//! Cache JSON colours before rendering. Validate whole blocks, then map their
//! token offsets onto the already wrapped display without changing any text.
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

pub(crate) fn highlight(text: &str, lines: &[String]) -> Vec<Option<Vec<Span<'static>>>> {
    // Long transcripts remain readable without an unbounded styling pass.
    if text.len() > 1024 * 1024 {
        return Vec::new();
    }
    let mut tokens = Vec::new();
    let mut blocks = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let start = at + text[at..].len() - text[at..].trim_start_matches([' ', '\t']).len();
        if matches!(text.as_bytes().get(start), Some(b'{' | b'[')) {
            let mut stream =
                serde_json::Deserializer::from_str(&text[start..]).into_iter::<serde_json::Value>();
            if let Some(Ok(value)) = stream.next()
                && (value.is_object() || value.is_array())
            {
                let end = start + stream.byte_offset();
                blocks.push((start, end));
                lex(text, start, end, &mut tokens);
                at = end;
                continue;
            }
        }
        at = text[at..].find('\n').map_or(text.len(), |n| at + n + 1);
    }
    let mut cursor = 0;
    lines
        .iter()
        .map(|line| {
            let start = cursor;
            let end = start + line.len();
            cursor = end + usize::from(text.as_bytes().get(end) == Some(&b'\n'));
            let block = blocks.partition_point(|&(_, b)| b <= start);
            if !blocks
                .get(block)
                .is_some_and(|&(a, b)| a <= end && b > start)
            {
                return None;
            }
            let first = tokens.partition_point(|t: &Token| t.end <= start);
            let mut pos = start;
            let mut spans = Vec::new();
            for token in tokens[first..].iter().take_while(|t| t.start < end) {
                let a = token.start.max(start);
                let b = token.end.min(end);
                if pos < a {
                    spans.push(Span::raw(text[pos..a].to_owned()));
                }
                spans.push(Span::styled(text[a..b].to_owned(), token.style));
                pos = b;
            }
            if pos < end {
                spans.push(Span::raw(text[pos..end].to_owned()));
            }
            Some(spans)
        })
        .collect()
}

struct Token {
    start: usize,
    end: usize,
    style: Style,
}

fn lex(text: &str, start: usize, end: usize, tokens: &mut Vec<Token>) {
    use super::theme::{JSON_KEY, JSON_LITERAL, JSON_PUNCTUATION, JSON_STRING};
    let bytes = text.as_bytes();
    let mut at = start;
    while at < end {
        let begin = at;
        let (color, bold): (Color, bool) = match bytes[at] {
            b'"' => {
                at += 1;
                while at < end {
                    match bytes[at] {
                        b'\\' => at += 2,
                        b'"' => {
                            at += 1;
                            break;
                        }
                        _ => at += 1,
                    }
                }
                let key = text[at..end].trim_start().starts_with(':');
                (if key { JSON_KEY } else { JSON_STRING }, key)
            }
            b'{' | b'}' | b'[' | b']' | b',' | b':' => {
                at += 1;
                (JSON_PUNCTUATION, false)
            }
            b if b.is_ascii_whitespace() => {
                at += 1;
                continue;
            }
            _ => {
                while at < end && !bytes[at].is_ascii_whitespace() && !b",]}".contains(&bytes[at]) {
                    at += 1;
                }
                (JSON_LITERAL, false)
            }
        };
        let mut style = Style::default().fg(color);
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        tokens.push(Token {
            start: begin,
            end: at,
            style,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validated_json_keeps_colour_across_wrapped_unicode_strings() {
        let text = "[Structured content]\n\n{\n  \"message\": \"café 界\",\n  \"ready\": true\n}\nordinary prose";
        let lines = vec![
            "[Structured content]",
            "",
            "{",
            "  \"message\": \"café ",
            "界\",",
            "  \"ready\": true",
            "}",
            "ordinary prose",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        let styles = highlight(text, &lines);
        assert!(styles[0].is_none() && styles[1].is_none() && styles[7].is_none());
        for (line, spans) in lines.iter().zip(&styles) {
            if let Some(spans) = spans {
                assert_eq!(
                    &spans.iter().map(|s| s.content.as_ref()).collect::<String>(),
                    line
                );
            }
        }
        assert_eq!(
            styles[4].as_ref().unwrap()[0].style.fg,
            Some(super::super::theme::JSON_STRING)
        );
        assert!(
            styles[5]
                .as_ref()
                .unwrap()
                .iter()
                .any(|s| s.content == "true"
                    && s.style.fg == Some(super::super::theme::JSON_LITERAL))
        );
        let bad = "{not JSON}\n[Structured content]\n{\"unfinished\":";
        assert!(
            highlight(bad, &bad.lines().map(str::to_owned).collect::<Vec<_>>())
                .iter()
                .all(Option::is_none)
        );
    }
}
