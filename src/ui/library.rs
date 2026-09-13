use super::{theme, truncate, wrap};
use crate::{inspector::safe_text, patterns::Learning, state::App};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    app.library.refresh_view();
    let width = (usize::from(area.width) * 55 / 100) as u16;
    let view = app.library.view.as_ref().unwrap();
    let stamp = (
        app.library.revision,
        view.selected.clone(),
        view.example,
        width,
    );
    if view.preview_stamp.as_ref() != Some(&stamp) {
        let lines = preview_lines(&app.library, width, &app.flow.theme.palette());
        let view = app.library.view.as_mut().unwrap();
        view.preview = lines;
        view.preview_stamp = Some(stamp);
    }
    let library = &app.library;
    let Some(v) = &library.view else { return };
    let palette = app.flow.theme.palette();
    let bg = Style::default().bg(palette.surface).fg(palette.text);
    let dim = bg.fg(palette.subtle);
    let accent = bg.fg(palette.accent);
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Fill(1),
        Constraint::Length(3),
    ])
    .areas(area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(" linger / patterns ", accent.add_modifier(Modifier::BOLD)),
                Span::styled(
                    if v.all {
                        "All cached sessions"
                    } else {
                        "This recording"
                    },
                    bg,
                ),
                Span::styled(
                    format!(
                        " · {} · {} rows · scripts {}{}",
                        v.level.label(),
                        v.rows.len(),
                        if v.show_scripts || !v.query.is_empty() { "included" } else { "hidden" },
                        if v.query.is_empty() {
                            String::new()
                        } else {
                            format!(" · /{}", safe_text(&v.query))
                        }
                    ),
                    dim,
                ),
            ]),
            Line::styled(
                " G guide · Tab scope · f level · c parts · S scripts · j/k select · h/l occurrence · Enter inspect · ? help · b/Esc return",
                dim,
            ),
        ])
        .style(bg),
        header,
    );
    let [list_area, preview] =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(body);
    let index = v
        .rows
        .iter()
        .position(|r| Some(&r.key) == v.selected.as_ref());
    let start = index
        .unwrap_or(0)
        .saturating_sub(list_area.height.saturating_sub(3) as usize / 4);
    let max = v
        .rows
        .iter()
        .map(|r| if v.all { r.total } else { r.here })
        .max()
        .unwrap_or(1)
        .max(1);
    let rows: Vec<_> = v
        .rows
        .iter()
        .skip(start)
        .take(list_area.height as usize / 2)
        .map(|r| {
            let state = library.learning(&r.key);
            let count = if v.all { r.total } else { r.here };
            let bar = "▰".repeat((count * 8).div_ceil(max).min(8));
            let style = if state == Learning::Practising {
                bg.fg(theme::PRACTISING).add_modifier(Modifier::BOLD)
            } else {
                bg
            };
            ListItem::new(vec![
                Line::styled(
                    format!(
                        "{} {}",
                        state.mark(),
                        truncate(
                            &safe_text(&r.label),
                            list_area.width.saturating_sub(7) as usize
                        )
                    ),
                    style,
                ),
                Line::styled(
                    format!(
                        "  {bar:<8} {:>4} here · {:>4} all · {} sessions",
                        r.here, r.total, r.spread
                    ),
                    dim,
                ),
            ])
        })
        .collect();
    let list = List::new(rows)
        .style(bg)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(theme::BORDER)
                .title(" Frequency ")
                .border_style(accent),
        )
        .highlight_symbol(theme::SELECTION_RAIL)
        .highlight_style(theme::selected());
    frame.render_stateful_widget(
        list,
        list_area,
        &mut ListState::default().with_selected(index.map(|n| n - start)),
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme::BORDER)
        .title(" Pattern / recorded example ")
        .border_style(dim)
        .style(bg);
    let inner = block.inner(preview);
    frame.render_widget(block, preview);
    let max_scroll = v.preview.len().saturating_sub(inner.height as usize);
    let scroll = v.scroll.min(max_scroll);
    frame.render_widget(
        Paragraph::new(
            v.preview
                .iter()
                .skip(scroll)
                .take(inner.height as usize)
                .cloned()
                .collect::<Vec<_>>(),
        )
        .style(bg),
        inner,
    );
    let help = if v.searching {
        format!(" /{}▏  Enter apply · Esc cancel", safe_text(&v.query))
    } else {
        " w Want · p Practising · L Learned · u Unmarked · H show/hide Learned · / search · PgUp/PgDn preview".into()
    };
    let status = library.notice.as_deref().unwrap_or(if library.ready {
        "Library saved locally · r refresh cache"
    } else {
        "Loading local library…"
    });
    frame.render_widget(Paragraph::new(vec![Line::styled(help,accent),Line::styled(" Counts are calls containing each form; rows overlap. Whole opened recordings, independent of playhead.",dim),Line::styled(format!(" {status}"),dim)]).style(bg),footer);
}

fn preview_lines(
    library: &crate::patterns::Library,
    width: u16,
    palette: &rataflow::Palette,
) -> Vec<Line<'static>> {
    let v = library.view.as_ref().unwrap();
    let bg = Style::default().bg(palette.surface).fg(palette.text);
    let dim = bg.fg(palette.subtle);
    let accent = bg.fg(palette.accent);
    let mut lines = Vec::new();
    if let Some(row) = library.selected() {
        let state = library.learning(&row.key);
        lines.push(Line::styled(
            format!("{} {}", state.mark(), state.label()),
            if state == Learning::Practising {
                bg.fg(theme::PRACTISING)
            } else {
                accent
            },
        ));
        for line in wrap(
            &safe_text(&row.label),
            width.saturating_sub(4) as usize,
            usize::MAX,
        ) {
            lines.push(Line::styled(line, bg.add_modifier(Modifier::BOLD)));
        }
        lines.push(Line::styled(
            format!(
                "{} in this recording · {} cached · {} sessions",
                row.here, row.total, row.spread
            ),
            dim,
        ));
        lines.push(Line::raw(""));
        let examples = library.examples(&row.key);
        let n = v.example.min(examples.len().saturating_sub(1));
        let (command, tool, pattern) = if let Some(o) = examples.get(n) {
            lines.push(Line::styled(
                format!(
                    "Occurrence {}/{} · {} · {}",
                    n + 1,
                    examples.len(),
                    safe_text(&o.agent),
                    safe_text(&o.call)
                ),
                accent,
            ));
            (
                o.pattern.command.as_str(),
                o.pattern.tool.as_str(),
                o.pattern.clone(),
            )
        } else {
            lines.push(Line::styled(
                "Cached example · open its session for output",
                accent,
            ));
            (
                row.example.as_str(),
                row.tool.as_str(),
                crate::patterns::pattern(&row.tool, &row.example)
                    .or_else(|| {
                        crate::patterns::pattern(
                            &row.tool,
                            &serde_json::json!({"command": &row.example}).to_string(),
                        )
                    })
                    .unwrap_or_else(|| crate::patterns::Pattern {
                        key: String::new(),
                        label: String::new(),
                        command: row.example.clone(),
                        tool: row.tool.clone(),
                    }),
            )
        };
        let ranges = crate::patterns::highlight_ranges(&pattern, &row.key);
        lines.push(Line::styled(
            if ranges.is_empty() {
                "Recorded input · exact location not mapped"
            } else {
                "Recorded input · matched form highlighted"
            },
            dim,
        ));
        let (input_lines, _) =
            super::inspector::highlighted_lines(command, &ranges, width.saturating_sub(4) as usize);
        lines.extend(input_lines);
        lines.push(Line::raw(""));
        let input = if serde_json::from_str::<serde_json::Value>(command)
            .ok()
            .is_some_and(|v| v.get("command").is_some() || v.get("cmd").is_some())
        {
            command.to_owned()
        } else {
            serde_json::json!({"command":command}).to_string()
        };
        let reference = if library
            .view
            .as_ref()
            .is_some_and(|v| v.level == crate::patterns::Level::Parts)
        {
            "Constituent tokens are highlighted in their recorded context.\nEnter inspects this call; 3 opens its command documentation.".into()
        } else {
            crate::inspector::reference_notes(tool, &input)
        };
        for line in reference.lines() {
            for part in wrap(line, width.saturating_sub(4) as usize, usize::MAX) {
                lines.push(Line::styled(part, dim));
            }
        }
    } else {
        lines.push(Line::styled("No patterns in this view.", bg));
        lines.push(Line::styled(
            "Try f for levels, S for scripts, H for Learned,",
            dim,
        ));
        lines.push(Line::styled("or / to change the search.", dim));
    }
    lines
}
