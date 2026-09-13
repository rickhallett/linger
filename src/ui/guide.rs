use super::{theme, truncate, wrap};
use crate::{guide::description, inspector::safe_text, patterns::Learning, state::App};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    app.guide.refresh(&app.library);
    let palette = app.flow.theme.palette();
    let bg = Style::default().bg(palette.surface).fg(palette.text);
    frame.render_widget(Block::default().style(bg), area);
    let dim = bg.fg(palette.subtle);
    let accent = bg.fg(palette.accent);
    let [heading, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(2),
    ])
    .areas(area);
    let guide = &app.guide;
    let heading_text = if guide.detail {
        guide
            .entry()
            .map(|e| format!(" linger / field guide / {}", safe_text(&e.program.label)))
            .unwrap_or_else(|| " linger / field guide".into())
    } else {
        " linger / field guide".into()
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(heading_text, accent.add_modifier(Modifier::BOLD)),
            Line::styled(
                format!(
                    " {} entries · {}{}",
                    guide.visible().len(),
                    if guide.here_only {
                        "This recording"
                    } else {
                        "All collected encounters"
                    },
                    if guide.query.is_empty() {
                        String::new()
                    } else {
                        format!(" · /{}", safe_text(&guide.query))
                    }
                ),
                dim,
            ),
        ])
        .style(bg),
        heading,
    );
    if guide.detail {
        detail(frame, body, app);
    } else {
        cards(frame, body, app);
    }
    let guide = &app.guide;
    let help = if guide.editing.is_some() {
        " Field note · Enter save · Esc cancel · arrows move cursor".into()
    } else if guide.searching {
        format!(" /{}▏ · Enter apply · Esc finish", safe_text(&guide.query))
    } else if guide.detail {
        " j/k forms · h/l specimens · Enter inspect · n note · p Practising · Esc entries · G return".into()
    } else {
        " h/j/k/l browse · Enter open · / search · Tab scope · G/Esc return".into()
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(help, accent),
            Line::styled(
                format!(
                    " {}",
                    guide
                        .notice
                        .as_deref()
                        .or(app.library.storage_error.as_deref())
                        .or(app.library.notice.as_deref())
                        .unwrap_or("Drill into a call to collect it · notes saved locally")
                ),
                dim,
            ),
        ])
        .style(bg),
        footer,
    );
}
fn cards(frame: &mut Frame, area: Rect, app: &mut App) {
    let palette = app.flow.theme.palette();
    let bg = Style::default().bg(palette.surface).fg(palette.text);
    let dim = bg.fg(palette.subtle);
    let columns = (usize::from(area.width) / 36).clamp(1, 4);
    app.guide.columns = columns;
    let entries = app.guide.visible();
    if entries.is_empty() {
        frame.render_widget(Paragraph::new("Drill into a tool call to collect your first specimen.\nEntries grow from calls you inspect. Clear / or change Tab scope if filtered.").style(dim), area);
        return;
    }
    let position = entries
        .iter()
        .position(|e| Some(&e.program.key) == app.guide.selected.as_ref())
        .unwrap_or(0);
    let height = 9u16;
    let rows = (area.height / height).max(1) as usize;
    let first_row = (position / columns).saturating_sub(rows - 1);
    let cell_width = area.width / columns as u16;
    for (n, entry) in entries
        .iter()
        .enumerate()
        .skip(first_row * columns)
        .take(rows * columns)
    {
        let x = (n % columns) as u16 * cell_width;
        let y = ((n / columns) - first_row) as u16 * height;
        let rect = Rect::new(
            area.x + x,
            area.y + y,
            cell_width.saturating_sub(1),
            height.min(area.height.saturating_sub(y)),
        );
        let selected = Some(&entry.program.key) == app.guide.selected.as_ref();
        let learning = app.library.learning(&entry.program.key);
        let border = if selected {
            bg.fg(palette.accent)
        } else {
            bg.fg(palette.muted)
        };
        let title = format!(
            " {}{} ",
            if selected { "▌ " } else { "" },
            safe_text(&entry.program.label)
        );
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(theme::BORDER)
            .border_style(border)
            .title(Span::styled(
                title,
                if selected {
                    theme::selected()
                } else {
                    bg.add_modifier(Modifier::BOLD)
                },
            ))
            .style(bg);
        let inner = block.inner(rect);
        frame.render_widget(block, rect);
        let (description, emblem) = description(&entry.program.label);
        let mut lines: Vec<Line> = emblem
            .iter()
            .map(|s| Line::styled(*s, if selected { bg.fg(palette.accent) } else { dim }))
            .collect();
        lines.push(Line::styled(
            truncate(description, inner.width as usize),
            bg,
        ));
        lines.push(Line::styled(
            format!(
                "{} here · {} collected · {} session{}",
                entry.program.here,
                entry.program.total,
                entry.program.spread,
                if entry.program.spread == 1 { "" } else { "s" }
            ),
            dim,
        ));
        lines.push(Line::styled(
            format!(
                "{} form{}{}",
                entry.forms.len().saturating_sub(1),
                if entry.forms.len() == 2 { "" } else { "s" },
                if app
                    .guide
                    .notes
                    .get(&entry.program.key)
                    .is_some_and(|n| !n.is_empty())
                {
                    " · field note"
                } else {
                    ""
                }
            ),
            dim,
        ));
        if learning != Learning::Unmarked {
            lines.push(Line::styled(
                format!("{} {}", learning.mark(), learning.label()),
                bg.fg(if learning == Learning::Practising {
                    theme::PRACTISING
                } else {
                    palette.accent
                }),
            ));
        }
        frame.render_widget(Paragraph::new(lines).style(bg), inner);
    }
}
fn detail(frame: &mut Frame, area: Rect, app: &App) {
    let Some(entry) = app.guide.entry() else {
        return;
    };
    let palette = app.flow.theme.palette();
    let bg = Style::default().bg(palette.surface).fg(palette.text);
    let dim = bg.fg(palette.subtle);
    let accent = bg.fg(palette.accent);
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(32), Constraint::Percentage(68)]).areas(area);
    let selected = entry
        .forms
        .iter()
        .position(|r| Some(&r.key) == app.guide.form.as_ref());
    let rows: Vec<_> = entry
        .forms
        .iter()
        .enumerate()
        .map(|(n, r)| {
            ListItem::new(vec![
                Line::styled(
                    if n == 0 {
                        "Collected specimens".into()
                    } else {
                        safe_text(&r.label)
                    },
                    bg,
                ),
                Line::styled(format!("{} here · {} collected", r.here, r.total), dim),
            ])
        })
        .collect();
    frame.render_stateful_widget(
        List::new(rows)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(theme::BORDER)
                    .title(" Collected forms ")
                    .border_style(accent),
            )
            .style(bg)
            .highlight_style(theme::selected())
            .highlight_symbol(theme::SELECTION_RAIL),
        left,
        &mut ListState::default().with_selected(selected),
    );
    let [specimen, note_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(6)]).areas(right);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme::BORDER)
        .title(" Specimen ")
        .border_style(dim)
        .style(bg);
    let inner = block.inner(specimen);
    frame.render_widget(block, specimen);
    let mut lines = vec![Line::styled(
        description(&entry.program.label).0,
        bg.add_modifier(Modifier::BOLD),
    )];
    if let Some(first) = &entry.program.first_seen {
        let date = chrono::DateTime::parse_from_rfc3339(first)
            .map(|d| d.format("%d %b %Y").to_string())
            .unwrap_or_default();
        lines.push(Line::styled(format!("Earliest record: {date}"), dim));
    }
    if let Some(row) = app.guide.row() {
        let examples = app.guide.examples(&row.key, &app.library.session);
        let n = app.guide.specimen.min(examples.len().saturating_sub(1));
        let pattern = if let Some(o) = examples.get(n) {
            lines.push(Line::styled(
                format!(
                    "Specimen {}/{} · {} / {}",
                    n + 1,
                    examples.len(),
                    safe_text(&o.agent),
                    safe_text(&o.call)
                ),
                accent,
            ));
            if o.session != app.library.session {
                lines.push(Line::styled(
                    "Collected in another recording · original needed for output",
                    dim,
                ));
            }
            o.pattern.clone()
        } else {
            lines.push(Line::styled(
                "Cached specimen · original session needed for output",
                dim,
            ));
            crate::guide::original(row)
        };
        lines.push(Line::raw(""));
        let ranges = crate::patterns::highlight_ranges(&pattern, &row.key);
        lines.extend(
            super::inspector::highlighted_lines(&pattern.command, &ranges, inner.width as usize).0,
        );
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "Enter opens recorded output · then 3 explores the command",
            dim,
        ));
    }
    let scroll = app
        .guide
        .scroll
        .min(lines.len().saturating_sub(inner.height as usize));
    frame.render_widget(
        Paragraph::new(lines.into_iter().skip(scroll).collect::<Vec<_>>()).style(bg),
        inner,
    );
    let editing = app.guide.editing.as_ref();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme::BORDER)
        .title(if editing.is_some() {
            " Field note · editing "
        } else {
            " Field note · n "
        })
        .border_style(if editing.is_some() { accent } else { dim })
        .style(bg);
    let inner = block.inner(note_area);
    frame.render_widget(block, note_area);
    let text = if let Some((_, draft, cursor)) = editing {
        format!(
            "{}▏{}",
            safe_text(&draft[..*cursor]),
            safe_text(&draft[*cursor..])
        )
    } else {
        app.guide
            .notes
            .get(&entry.program.key)
            .filter(|n| !n.is_empty())
            .map(|n| safe_text(n))
            .unwrap_or_else(|| "A detail worth keeping. Press n to add a note.".into())
    };
    let lines = wrap(&text, inner.width as usize, usize::MAX);
    let skip = if let Some((_, draft, cursor)) = editing {
        wrap(
            &format!("{}▏", safe_text(&draft[..*cursor])),
            inner.width as usize,
            usize::MAX,
        )
        .len()
        .saturating_sub(inner.height as usize)
    } else {
        0
    };
    frame.render_widget(
        Paragraph::new(
            lines
                .into_iter()
                .skip(skip)
                .map(Line::raw)
                .collect::<Vec<_>>(),
        )
        .style(bg),
        inner,
    );
}
