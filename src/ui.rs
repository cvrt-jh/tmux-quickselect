use crate::app::App;
use crate::config::parse_color;
use crate::history::format_relative;
use crate::scanner::GitStatus;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(f: &mut Frame, app: &App) {
    let area = centered_rect(app.config.ui.width as u16 + 40, app.projects.len() as u16 + 6, f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header (title + group counts / breadcrumb + filter)
            Constraint::Min(1),   // project list
            Constraint::Length(1), // footer
        ])
        .split(area);

    render_header(f, app, chunks[0]);
    render_list(f, app, chunks[1]);
    render_footer(f, chunks[2]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(" {} {} ", app.config.ui.icon, app.config.ui.title);

    let block = Block::default()
        .title(title)
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT);

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Build the info line: group counts or breadcrumb on left, filter on right
    let left_spans = if !app.nav_stack.is_empty() || app.browsing_path.is_some() {
        // Breadcrumb mode
        let path = app.browsing_path.as_deref().unwrap_or("root");
        vec![Span::styled(
            format!(" {} ", path),
            Style::default().fg(Color::Yellow),
        )]
    } else {
        // Group counts
        let groups = app.group_counts();
        let mut spans = Vec::new();
        for (i, (label, count, color)) in groups.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(
                format!(" {}:{}", label, count),
                Style::default().fg(parse_color(color)),
            ));
        }
        spans
    };

    let filter_text = if app.filter_input.is_empty() {
        String::new()
    } else {
        format!("Filter: {} ", app.filter_input)
    };

    let mut all_spans = left_spans;
    if !filter_text.is_empty() {
        // Calculate padding
        let left_len: usize = all_spans.iter().map(|s| s.content.len()).sum();
        let right_len = filter_text.len();
        let total_width = inner.width as usize;
        let padding = total_width.saturating_sub(left_len + right_len);
        all_spans.push(Span::raw(" ".repeat(padding)));
        all_spans.push(Span::styled(
            filter_text,
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ));
    }

    let info_line = Paragraph::new(Line::from(all_spans));
    f.render_widget(info_line, inner);
}

fn render_list(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible = app.visible_projects();
    let name_width = app.config.ui.width;

    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, project)| {
            let is_selected = i == app.selected;

            // Label
            let label_span = Span::styled(
                format!("{:<6}", project.label),
                Style::default().fg(parse_color(&project.color)),
            );

            // Name (padded)
            let name_display = if project.name.len() > name_width {
                project.name[..name_width].to_string()
            } else {
                format!("{:<width$}", project.name, width = name_width)
            };
            let name_span = Span::styled(
                name_display,
                Style::default().fg(Color::White),
            );

            // Git status indicator
            let git_span = match &project.git_status {
                Some(GitStatus::Dirty(_)) => Span::styled(" \u{25cf} ", Style::default().fg(Color::Red)),
                Some(GitStatus::Clean) => Span::styled(" \u{25cb} ", Style::default().fg(Color::Green)),
                None => Span::raw("   "),
            };

            // Drill-down indicator
            let drill_span = if project.has_children {
                Span::styled("\u{2192} ", Style::default().fg(Color::Cyan))
            } else {
                Span::raw("  ")
            };

            // Last used time
            let time_text = match &project.last_used {
                Some(dt) => format_relative(*dt),
                None => "-".to_string(),
            };
            let time_span = Span::styled(
                format!("{:>10}", time_text),
                Style::default().fg(Color::DarkGray),
            );

            // Cursor indicator
            let cursor = if is_selected { " > " } else { "   " };
            let cursor_span = Span::styled(
                cursor,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            );

            let line = Line::from(vec![
                cursor_span,
                label_span,
                name_span,
                git_span,
                drill_span,
                time_span,
            ]);

            let style = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, inner);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let help = Line::from(vec![
        Span::styled(" \u{2191}\u{2193}", Style::default().fg(Color::DarkGray)),
        Span::raw(" navigate  "),
        Span::styled("\u{23ce}", Style::default().fg(Color::DarkGray)),
        Span::raw(" select  "),
        Span::styled("q", Style::default().fg(Color::DarkGray)),
        Span::raw(" quit  "),
        Span::styled("e", Style::default().fg(Color::DarkGray)),
        Span::raw(" config  "),
        Span::styled("h", Style::default().fg(Color::DarkGray)),
        Span::raw(" hidden"),
    ]);

    let block = Block::default()
        .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let footer = Paragraph::new(help);
    f.render_widget(footer, inner);
}

/// Returns a centered rect that fits within `area`.
fn centered_rect(max_width: u16, max_height: u16, area: Rect) -> Rect {
    let width = max_width.min(area.width);
    let height = max_height.min(area.height);

    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Rect::new(x, y, width, height)
}
