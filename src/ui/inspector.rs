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
    if let Some(i) = &mut app.inspector {
        i.content_width = (usize::from(area.width) * 70 / 100).saturating_sub(10);
    }
    app.refresh_inspector();
    let Some(i) = &app.inspector else { return };
    let palette = app.flow.theme.palette();
    let bg = Style::default().bg(palette.surface).fg(palette.text);
    let accent = bg.fg(palette.accent);
    let muted = bg.fg(palette.muted);
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
    frame.render_widget(Paragraph::new(format!(" Linger / {} / {} calls    {} events ahead\n Enter drill in · Esc back · ,/. step event · [/] prompt · g live · space pause", safe_text(&i.agent), count, buffered)).style(bg), heading);
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
            ListItem::new(vec![
                Line::from(format!("{} {}", glyph, safe_text(&c.name))),
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
                .title(if i.detail {
                    " Calls "
                } else {
                    " Calls · j/k "
                })
                .border_style(if i.detail { muted } else { accent }),
        )
        .highlight_style(bg.fg(palette.accent).add_modifier(Modifier::BOLD))
        .highlight_symbol("› ");
    let mut state = ListState::default().with_selected(position.map(|n| n - start));
    frame.render_stateful_widget(list, calls_area, &mut state);
    let title = [
        (Tab::Input, "1 Input"),
        (Tab::Output, "2 Output"),
        (Tab::Explain, "3 Reference"),
        (Tab::Interpret, "4 Interpretation"),
    ]
    .into_iter()
    .map(|(t, label)| {
        Span::styled(
            format!(" {} ", label),
            if t == i.tab {
                accent.add_modifier(Modifier::BOLD)
            } else {
                muted
            },
        )
    })
    .collect::<Vec<_>>();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(title))
        .border_style(if i.detail { accent } else { muted })
        .style(bg);
    let inner = block.inner(detail_area);
    frame.render_widget(block, detail_area);
    let scroll = i.scroll.min(i.lines.len().saturating_sub(1));
    let query = i.query.to_lowercase();
    let lines: Vec<_> = i
        .lines
        .iter()
        .enumerate()
        .skip(scroll)
        .take(inner.height as usize)
        .map(|(n, line)| {
            let style = if !query.is_empty() && line.to_lowercase().contains(&query) {
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
        Paragraph::new(lines).scroll((0, i.horizontal)).style(bg),
        inner,
    );
    let help = if i.searching {
        format!(" /{}▏  Enter find · Esc cancel", safe_text(&i.query))
    } else {
        format!(
            " 1–4 tabs · v raw/readable · i Mercury · / search · n next · h/l pan · PgUp/PgDn scroll   {}/{}",
            scroll + 1,
            i.lines.len()
        )
    };
    let status = i.notice.clone().unwrap_or_else(|| {
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
