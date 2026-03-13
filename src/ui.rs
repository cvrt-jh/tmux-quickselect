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

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header (title + group counts / breadcrumb + filter)
            Constraint::Min(1),   // project list
            Constraint::Length(2), // footer
        ])
        .split(area);

    render_header(f, app, chunks[0]);

    // Calculate visible height for scrolling (inner area minus borders)
    let list_block = Block::default().borders(Borders::LEFT | Borders::RIGHT);
    let list_inner = list_block.inner(chunks[1]);
    app.adjust_scroll((list_inner.height as usize).saturating_sub(1));

    render_list(f, app, chunks[1]);
    render_footer(f, app, chunks[2]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(" {} {} ", app.config.ui.icon, app.config.ui.title);

    let block = Block::default()
        .title(title)
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT);

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Build the info line
    let all_spans = if app.search_mode {
        // Search mode: show search input prominently
        let count = app.filtered_indices.len();
        let mut spans = vec![
            Span::styled(" Search: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                &app.filter_input,
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "\u{2588}", // cursor block
                Style::default().fg(Color::White),
            ),
        ];
        // Show result count on the right
        let count_text = format!(" {} results", count);
        let left_len: usize = spans.iter().map(|s| s.content.len()).sum();
        let total_width = inner.width as usize;
        let padding = total_width.saturating_sub(left_len + count_text.len());
        spans.push(Span::raw(" ".repeat(padding)));
        spans.push(Span::styled(count_text, Style::default().fg(Color::DarkGray)));
        spans
    } else if !app.nav_stack.is_empty() || app.browsing_path.is_some() {
        // Breadcrumb mode
        let path = app.browsing_path.as_deref().unwrap_or("root");
        let mut spans = vec![Span::styled(
            format!(" {} ", path),
            Style::default().fg(Color::Yellow),
        )];

        if !app.filter_input.is_empty() {
            let filter_text = format!("Filter: {} ", app.filter_input);
            let left_len: usize = spans.iter().map(|s| s.content.len()).sum();
            let total_width = inner.width as usize;
            let padding = total_width.saturating_sub(left_len + filter_text.len());
            spans.push(Span::raw(" ".repeat(padding)));
            spans.push(Span::styled(
                filter_text,
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ));
        }
        spans
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

        if !app.filter_input.is_empty() {
            let filter_text = format!("Filter: {} ", app.filter_input);
            let left_len: usize = spans.iter().map(|s| s.content.len()).sum();
            let total_width = inner.width as usize;
            let padding = total_width.saturating_sub(left_len + filter_text.len());
            spans.push(Span::raw(" ".repeat(padding)));
            spans.push(Span::styled(
                filter_text,
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ));
        }
        spans
    };

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
    // Leave 1 row padding at bottom before footer
    let visible_height = (inner.height as usize).saturating_sub(1);

    // Apply scroll offset — only render the visible window
    let end = (app.scroll_offset + visible_height).min(visible.len());
    let window = &visible[app.scroll_offset..end];

    let items: Vec<ListItem> = window
        .iter()
        .enumerate()
        .map(|(i, project)| {
            let absolute_i = i + app.scroll_offset;
            let is_selected = absolute_i == app.selected;

            // Label
            let label_span = Span::styled(
                format!("{:<6}", project.label),
                Style::default().fg(parse_color(&project.color)),
            );

            // Name (padded) — show relative path in search mode
            let display_name = if app.search_mode {
                // Show path relative to home
                let home = dirs::home_dir()
                    .map(|h| h.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let rel = project.path.strip_prefix(&home).unwrap_or(&project.path);
                format!("~{}", rel)
            } else {
                project.name.clone()
            };
            let display_width = if app.search_mode { name_width + 20 } else { name_width };
            let name_display = if display_name.len() > display_width {
                display_name[..display_width].to_string()
            } else {
                format!("{:<width$}", display_name, width = display_width)
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

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let key_style = Style::default().fg(Color::DarkGray);
    let label_style = Style::default().fg(Color::White);
    let sep = Span::styled("  ", Style::default());

    let spans = if app.search_mode {
        vec![
            Span::styled(" [ESC]", key_style),
            Span::styled(":Back", label_style),
            sep.clone(),
            Span::styled("[RET]", key_style),
            Span::styled(":Select", label_style),
            sep.clone(),
            Span::styled("[\u{2191}\u{2193}]", key_style),
            Span::styled(":Navigate", label_style),
        ]
    } else if app.browsing_path.is_some() {
        vec![
            Span::styled(" [TAB]", key_style),
            Span::styled(":Select folder", label_style),
            sep.clone(),
            Span::styled("[RET]", key_style),
            Span::styled(":Open", label_style),
            sep.clone(),
            Span::styled("[ESC]", key_style),
            Span::styled(":Back", label_style),
            sep,
            Span::styled("[Q]", key_style),
            Span::styled(":Quit", label_style),
        ]
    } else {
        let hidden_label = if app.config.show_hidden {
            ":Hidden \u{2713}"
        } else {
            ":Hidden"
        };
        vec![
            Span::styled(" [/]", key_style),
            Span::styled(":Search", label_style),
            sep.clone(),
            Span::styled("[H]", key_style),
            Span::styled(hidden_label, label_style),
            sep.clone(),
            Span::styled("[E]", key_style),
            Span::styled(":Config", label_style),
            sep.clone(),
            Span::styled("[RET]", key_style),
            Span::styled(":Select", label_style),
            sep.clone(),
            Span::styled("[ESC]", key_style),
            Span::styled(":Back", label_style),
            sep,
            Span::styled("[Q]", key_style),
            Span::styled(":Quit", label_style),
        ]
    };

    let help = Line::from(spans);
    let block = Block::default()
        .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let footer = Paragraph::new(help);
    f.render_widget(footer, inner);
}

