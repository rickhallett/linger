//! Quiet key legends with a short, layout-stable acknowledgement of an action.
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use web_time::{Duration, Instant};

const HOLD: Duration = Duration::from_millis(650);
#[derive(Default)]
pub struct Feedback {
    recent: Option<(String, Instant)>,
}
impl Feedback {
    pub fn record(&mut self, key: String) {
        self.recent = Some((key, Instant::now()));
    }
    pub fn active(&self, key: &str) -> bool {
        self.recent
            .as_ref()
            .is_some_and(|(k, at)| k == key && at.elapsed() < HOLD)
    }
}

pub fn line(text: &str, base: Style, feedback: &Feedback) -> Line<'static> {
    let mut spans = Vec::new();
    for (n, chunk) in text.split(" · ").enumerate() {
        if n > 0 {
            spans.push(Span::styled(" · ", base));
        }
        let leading = chunk.len() - chunk.trim_start().len();
        let rest = &chunk[leading..];
        let end = rest.find(' ').unwrap_or(rest.len());
        let token = &rest[..end];
        let keys: Vec<_> = if token == "/" {
            vec!["/"]
        } else {
            token.split('/').collect()
        };
        let known = |s: &str| {
            s.chars().count() == 1
                || matches!(
                    s,
                    "Enter" | "Esc" | "Tab" | "PgUp" | "PgDn" | "Home" | "End" | "Space"
                )
        };
        if !keys.iter().all(|k| known(k)) || token.is_empty() {
            spans.push(Span::styled(chunk.to_owned(), base));
            continue;
        }
        spans.push(Span::styled(chunk[..leading].to_owned(), base));
        for (i, key) in keys.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("/", base));
            }
            let style = if feedback.active(key) {
                Style::default()
                    .bg(super::theme::PRACTISING)
                    .fg(ratatui::style::Color::Rgb(12, 21, 18))
                    .add_modifier(Modifier::BOLD)
            } else {
                base.fg(super::theme::BRIGHT_TEXT)
                    .add_modifier(Modifier::BOLD)
            };
            spans.push(Span::styled((*key).to_owned(), style));
        }
        spans.push(Span::styled(rest[end..].to_owned(), base));
    }
    Line::from(spans)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_activated_key_lights_up_without_changing_text() {
        let mut f = Feedback::default();
        f.record("k".into());
        let text = " j/k select · f level · / search";
        let l = line(text, Style::default(), &f);
        assert_eq!(l.to_string(), text);
        let lit: Vec<_> = l
            .spans
            .iter()
            .filter(|s| s.style.bg.is_some())
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(lit, ["k"]);
        f.recent = Some(("k".into(), Instant::now() - HOLD));
        assert!(!f.active("k"));
    }
}
