use crate::{
    inspector::{Tab, safe_text},
    state::App,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let [_, content_area] =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(area);
    if let Some(i) = &mut app.inspector {
        i.content_width = usize::from(content_area.width).saturating_sub(9).max(1);
    }
    app.refresh_inspector();
    let Some(i) = &app.inspector else { return };
    let palette = app.flow.theme.palette();
    let bg = Style::default().bg(palette.surface).fg(palette.text);
    let accent = bg.fg(palette.accent);
    let muted = bg.fg(palette.subtle);
    let border = bg.fg(palette.muted);
    let [heading, body, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Fill(1),
        Constraint::Length(2),
    ])
    .areas(area);
    let count = app
        .session
        .agent(&i.agent)
        .map_or(0, |a| a.tool_calls().len());
    let buffered = app.timeline.items.len().saturating_sub(app.timeline.folded);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(" linger / ", accent.add_modifier(Modifier::BOLD)),
                Span::styled(format!("{} / {} calls", safe_text(&i.agent), count), bg),
                Span::styled(format!("    {} events ahead", buffered), muted),
            ]),
            Line::styled(
                " Enter drill in · Esc back · G guide · b patterns · p Practising",
                muted,
            ),
        ])
        .style(bg),
        heading,
    );
    let [calls_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(body);
    let calls: Vec<_> = app
        .session
        .agent(&i.agent)
        .map(|a| a.tool_calls().collect())
        .unwrap_or_default();
    let position = calls.iter().position(|c| Some(&c.id) == i.call.as_ref());
    let start = position
        .unwrap_or(0)
        .saturating_sub(calls_area.height.saturating_sub(4) as usize / 2);
    let rows: Vec<_> = calls
        .iter()
        .skip(start)
        .take(calls_area.height as usize)
        .map(|c| {
            let glyph = match c.state {
                crate::state::session::ToolState::Pending => "·",
                crate::state::session::ToolState::Ok => "✓",
                crate::state::session::ToolState::Err => "✗",
            };
            let learning = app.visible_learning(&i.agent, &c.id);
            ListItem::new(vec![
                Line::styled(
                    format!(
                        "{} {}{}",
                        glyph,
                        if learning == crate::patterns::Learning::Practising {
                            "◎ "
                        } else {
                            ""
                        },
                        safe_text(&c.name)
                    ),
                    if learning == crate::patterns::Learning::Practising {
                        bg.fg(super::theme::PRACTISING).add_modifier(Modifier::BOLD)
                    } else {
                        bg
                    },
                ),
                Line::styled(
                    format!("  {}", safe_text(c.summary.as_deref().unwrap_or(&c.id))),
                    muted,
                ),
            ])
        })
        .collect();
    let list = List::new(rows)
        .style(bg)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(super::theme::BORDER)
                .title(if i.detail {
                    " Calls "
                } else {
                    " Calls · j/k "
                })
                .border_style(if i.detail { border } else { accent }),
        )
        .highlight_style(super::theme::selected())
        .highlight_symbol(super::theme::SELECTION_RAIL);
    let mut state = ListState::default().with_selected(position.map(|n| n - start));
    frame.render_stateful_widget(list, calls_area, &mut state);
    let title = [
        (Tab::Input, "1 Input"),
        (Tab::Output, "2 Output"),
        (Tab::Explain, "3 Command"),
        (Tab::Interpret, "4 Interpretation"),
    ]
    .into_iter()
    .map(|(t, label)| {
        Span::styled(
            format!(" {} ", label),
            if t == i.tab {
                super::theme::selected()
            } else {
                muted
            },
        )
    })
    .collect::<Vec<_>>();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(super::theme::BORDER)
        .title(Line::from(title))
        .border_style(if i.detail { accent } else { border })
        .style(bg);
    let mut inner = block.inner(detail_area);
    frame.render_widget(block, detail_area);
    if i.tab == Tab::Explain && !i.command_lines.is_empty() {
        let height = (i.command_lines.len() as u16)
            .min(6)
            .min(inner.height / 3)
            .max(1);
        let [command_area, hint_area, reading_area] = Layout::vertical([
            Constraint::Length(height),
            Constraint::Length(2),
            Constraint::Fill(1),
        ])
        .areas(inner);
        let first = i
            .command_row
            .saturating_sub(height as usize / 2)
            .min(i.command_lines.len().saturating_sub(height as usize));
        frame.render_widget(
            Paragraph::new(
                i.command_lines
                    .iter()
                    .skip(first)
                    .take(height as usize)
                    .cloned()
                    .collect::<Vec<_>>(),
            )
            .style(bg),
            command_area,
        );
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!(
                    "h/l ←/→ parts · j/k scroll{}{}",
                    if i.command_from_argv {
                        " · shell string from argv"
                    } else {
                        ""
                    },
                    if i.command_lines.len() > height as usize {
                        " · command excerpt"
                    } else {
                        ""
                    }
                )),
                Line::from(
                    [
                        ("command", "command"),
                        ("flag", "option"),
                        ("argument", "argument"),
                        ("syntax", "shell"),
                        ("unknown", "unknown"),
                    ]
                    .into_iter()
                    .map(|(label, kind)| {
                        Span::styled(
                            format!("{label}  "),
                            super::theme::syntax(kind, kind != "unknown"),
                        )
                    })
                    .collect::<Vec<_>>(),
                ),
            ])
            .style(muted),
            hint_area,
        );
        inner = reading_area;
    }
    let scroll = i.scroll.min(i.lines.len().saturating_sub(1));
    let query = i.query.to_lowercase();
    let lines: Vec<_> = i
        .lines
        .iter()
        .enumerate()
        .skip(scroll)
        .take(inner.height as usize)
        .map(|(n, line)| {
            let style = if n == 0 && i.tab == Tab::Explain {
                i.command
                    .as_ref()
                    .and_then(|c| app.explorer.cache.get(c))
                    .and_then(|e| e.spans.get(i.part))
                    .map(|p| super::theme::syntax(&p.kind, p.known).add_modifier(Modifier::BOLD))
                    .unwrap_or(bg)
            } else if !query.is_empty() && line.to_lowercase().contains(&query) {
                accent.add_modifier(Modifier::BOLD)
            } else {
                bg
            };
            Line::from(vec![
                Span::styled(format!("{:>4} │ ", n + 1), muted),
                Span::styled(line.clone(), style),
            ])
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((0, if i.nowrap { i.horizontal } else { 0 }))
            .style(bg),
        inner,
    );
    let help = if i.searching {
        format!(" /{}▏  Enter find · Esc cancel", safe_text(&i.query))
    } else if i.tab == Tab::Explain {
        " 1–4 tabs · h/l parts · j/k scroll · W wrap · i Mercury · / search · Esc calls".into()
    } else {
        format!(
            " 1–4 tabs · v raw/readable · i Mercury · / search · n next · W {} · PgUp/PgDn   {}/{}",
            if i.nowrap { "wrap (h/l pan)" } else { "unwrap" },
            scroll + 1,
            i.lines.len()
        )
    };
    let learning = i
        .call
        .as_ref()
        .map(|call| app.visible_learning(&i.agent, call))
        .unwrap_or_default();
    let status = app.library.storage_error.clone().or_else(|| i.notice.clone()).unwrap_or_else(|| {
        if learning != crate::patterns::Learning::Unmarked {
            return format!(
                " {} {} · w Want · p Practising · L Learned · u Unmarked",
                learning.mark(),
                learning.label()
            );
        }
        if i.tab == Tab::Interpret {
            " Interpretation uses selected input/output; raw evidence stays in the other tabs."
                .into()
        } else {
            " Recorded content at the playhead · control characters escaped · no commands executed"
                .into()
        }
    });
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(help, accent),
            Line::styled(status, muted),
        ])
        .style(bg),
        footer,
    );
}

/// Wrap the exact command once per changed selection/width. Highlight its byte
/// range before sanitizing display characters, so Unicode/control bytes cannot
/// shift the selected documentation away from its evidence.
pub(crate) fn command_lines(
    command: &str,
    part: Option<&crate::exploration::Part>,
    parts: &[crate::exploration::Part],
    width: usize,
) -> (Vec<Line<'static>>, usize) {
    styled_lines(
        command,
        &part.map(|p| vec![(p.start, p.end)]).unwrap_or_default(),
        parts,
        width,
    )
}

pub(crate) fn highlighted_lines(
    command: &str,
    ranges: &[(usize, usize)],
    width: usize,
) -> (Vec<Line<'static>>, usize) {
    styled_lines(command, ranges, &[], width)
}
fn styled_lines(
    command: &str,
    ranges: &[(usize, usize)],
    parts: &[crate::exploration::Part],
    width: usize,
) -> (Vec<Line<'static>>, usize) {
    use unicode_width::UnicodeWidthChar;
    let normal = Style::default().fg(super::theme::theme().palette().text);
    let selected = super::theme::selected().add_modifier(Modifier::BOLD);
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut columns = 0;
    let mut focus = None;
    for (offset, c) in command.char_indices() {
        let active = ranges
            .iter()
            .any(|&(start, end)| start <= offset && offset < end);
        if c == '\n' {
            lines.push(Line::from(std::mem::take(&mut spans)));
            columns = 0;
            continue;
        }
        let syntax = parts
            .iter()
            .filter(|p| p.start <= offset && offset < p.end)
            .min_by_key(|p| p.end - p.start)
            .map(|p| super::theme::syntax(&p.kind, p.known))
            .unwrap_or(normal);
        for display in safe_text(&c.to_string()).chars() {
            let size = display.width().unwrap_or(0);
            if columns > 0 && columns + size > width {
                lines.push(Line::from(std::mem::take(&mut spans)));
                columns = 0;
            }
            if active && focus.is_none() {
                focus = Some(lines.len());
            }
            spans.push(Span::styled(
                display.to_string(),
                if active { selected } else { syntax },
            ));
            columns += size;
        }
    }
    lines.push(Line::from(spans));
    (lines, focus.unwrap_or(0))
}

#[cfg(test)]
mod syntax_tests {
    use super::*;
    #[test]
    fn syntax_roles_keep_colours_and_selected_range_across_wrapping() {
        let command = "rg -n 界 | cat";
        let parts = vec![
            (0, 2, "synopsis", true),
            (3, 5, "option", true),
            (6, 9, "argument", true),
            (10, 11, "shell", true),
            (12, 15, "unknown", false),
        ]
        .into_iter()
        .map(|(start, end, kind, known)| crate::exploration::Part {
            start,
            end,
            kind: kind.into(),
            known,
            text: String::new(),
            source: String::new(),
            extractor: String::new(),
        })
        .collect::<Vec<_>>();
        let (lines, _) = command_lines(command, Some(&parts[1]), &parts, 8);
        let spans = lines.iter().flat_map(|l| &l.spans).collect::<Vec<_>>();
        assert_eq!(
            spans.iter().map(|s| s.content.as_ref()).collect::<String>(),
            command
        );
        assert!(
            spans.iter().any(|s| s.content == "界"
                && s.style.fg == super::super::theme::syntax("argument", true).fg)
        );
        assert!(
            spans
                .iter()
                .any(|s| s.content == "-" && s.style.bg == Some(super::super::theme::SELECTION))
        );
        assert!(
            spans
                .iter()
                .any(|s| s.content == "c" && s.style.add_modifier.contains(Modifier::UNDERLINED))
        );
        assert_ne!(
            super::super::theme::syntax("command", true).fg,
            super::super::theme::syntax("option", true).fg
        );
    }
}
