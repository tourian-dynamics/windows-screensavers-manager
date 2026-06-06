//! Ratatui-based rendering. Pure function of `App` -> `Frame`.
//!
//! # Model-Render Split
//! rSaver uses a strict Model-Render architectural split:
//!
//! * **Model (`app.rs`)**: Owns the state (selected saver, timer configuration, focus, etc.)
//!   and implements the business logic, key handlers, and state modifications.
//! * **Render (`ui.rs`)**: Takes a mutable reference to the `App` state and draws the layout,
//!   widgets, list view, borders, help texts, and active indicators to the screen.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::{App, FocusedSection, GlobalField};

pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let theme = app.theme;
    let (min_w, min_h) = crate::theme::recommended_min_size(96);

    if area.width < min_w || area.height < min_h {
        render_too_small(theme, frame, area);
        return;
    }

    // Split entire area vertically into bordered boxes
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 0: Header box
            Constraint::Length(7), // 1: Global Prefs & Help (side-by-side)
            Constraint::Min(10),   // 2: Screensaver Preferences list
            Constraint::Length(3), // 3: Status / Progress footer box
        ])
        .split(area);

    // 0. Render Header Info Box
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(
            " Rust Screensaver Manager ",
            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
        ));
    
    let username = std::env::var("USERNAME").unwrap_or_else(|_| std::env::var("USER").unwrap_or_else(|_| "user".to_string()));
    let hostname = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "localhost".to_string());
    let os_str = crate::win32::query_os_version();

    let header_line = Line::from(vec![
        Span::styled(
            format!(" rSaver v{} ", env!("CARGO_PKG_VERSION")),
            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(theme.border)),
        Span::styled(
            "Press ? for help",
            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(theme.border)),
        Span::styled(format!("{}@{}", username, hostname), Style::default().fg(Color::Rgb(255, 215, 0)).add_modifier(Modifier::BOLD)),
        Span::styled(" │ ", Style::default().fg(theme.border)),
        Span::styled(os_str, Style::default().fg(theme.text_main)),
    ]);
    let header_inner = header_block.inner(chunks[0]);
    frame.render_widget(header_block, chunks[0]);
    frame.render_widget(Paragraph::new(header_line), header_inner);

    // 1. Render Side-by-Side Global Prefs & Help
    let top_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[1]);

    render_prefs(app, frame, top_split[0]);
    render_help(theme, frame, top_split[1]);

    // 2. Render Screensaver Preferences List Table
    render_list(app, frame, chunks[2]);

    // 3. Render Footer Status Box
    let footer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(
            " Status ",
            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
        ));

    #[cfg(feature = "downloader")]
    let mut is_downloading = false;
    #[cfg(feature = "downloader")]
    let mut download_name = String::new();
    #[cfg(feature = "downloader")]
    {
        if let Some(ref state_mutex) = app.download_state {
            is_downloading = true;
            if let Ok(state) = state_mutex.lock() {
                download_name = state.name.clone();
            }
        }
    }

    #[cfg(feature = "downloader")]
    let footer_p = if is_downloading {
        let progress = app.visual_progress;
        let track_width = 30;
        let pacman_pos = ((progress * track_width as f64).round() as usize).min(track_width);
        
        let mut track = String::new();
        let is_mouth_open = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() / 150)
            .unwrap_or(0) % 2) == 0;
            
        let pacman_char = if progress >= 1.0 {
            "o"
        } else if is_mouth_open {
            "ᗧ"
        } else {
            "o"
        };

        if progress < 1.0 {
            for _ in 0..pacman_pos {
                track.push(' ');
            }
            track.push_str(pacman_char);
            for i in (pacman_pos + 1)..track_width {
                if i == track_width - 1 {
                    track.push('ᗣ');
                } else {
                    track.push('·');
                }
            }
        } else {
            for _ in 0..track_width.saturating_sub(1) {
                track.push(' ');
            }
            track.push('o');
        }

        Paragraph::new(Line::from(vec![
            Span::styled(format!("Downloading ({}): ", download_name), Style::default().fg(theme.text_main).add_modifier(Modifier::BOLD)),
            Span::styled(" [", Style::default().fg(theme.border)),
            Span::styled(track, Style::default().fg(theme.accent_primary)),
            Span::styled("]", Style::default().fg(theme.border)),
            Span::styled(format!(" {:>3.0}%", progress * 100.0), Style::default().fg(theme.accent_secondary)),
        ]))
    } else if let Some(ref status) = app.status {
        let (color, icon) = match status.kind {
            crate::app::StatusKind::Info => (theme.accent_primary, app.glyphs.info),
            crate::app::StatusKind::Error => (theme.missing, app.glyphs.status_err),
        };
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(color)),
            Span::styled(&status.text, Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled("Ready. Press Tab to cycle focus.", Style::default().fg(theme.text_dim)),
        ]))
    };

    #[cfg(not(feature = "downloader"))]
    let footer_p = if let Some(ref status) = app.status {
        let (color, icon) = match status.kind {
            crate::app::StatusKind::Info => (theme.accent_primary, app.glyphs.info),
            crate::app::StatusKind::Error => (theme.missing, app.glyphs.status_err),
        };
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(color)),
            Span::styled(&status.text, Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled("Ready. Press Tab to cycle focus.", Style::default().fg(theme.text_dim)),
        ]))
    };

    let footer_inner = footer_block.inner(chunks[3]);
    frame.render_widget(footer_block, chunks[3]);
    frame.render_widget(footer_p, footer_inner);

    // Handle Mouse Selection Highlights & Clipboard Copy
    if let (Some(start), Some(end)) = (app.selection_start, app.selection_end) {
        let buf = frame.buffer_mut();
        let width = buf.area.width;
        let height = buf.area.height;

        let (col1, row1) = start;
        let (col2, row2) = end;

        let is_selected = |x: u16, y: u16| -> bool {
            let (c1, r1) = (col1, row1);
            let (c2, r2) = (col2, row2);
            if r1 == r2 {
                y == r1 && x >= c1.min(c2) && x <= c1.max(c2)
            } else if r1 < r2 {
                (y == r1 && x >= c1) || (y > r1 && y < r2) || (y == r2 && x <= c2)
            } else {
                (y == r2 && x >= c2) || (y > r2 && y < r1) || (y == r1 && x <= c1)
            }
        };

        // 1. Draw Highlight
        for y in 0..height {
            for x in 0..width {
                if is_selected(x, y) {
                    let cell = &mut buf[(x, y)];
                    cell.set_bg(Color::Rgb(0, 120, 215));
                    cell.set_fg(Color::White);
                }
            }
        }

        // 2. Perform Copy on Release
        if app.selection_pending_copy {
            let mut selected_text = String::new();
            let mut current_row: Option<u16> = None;
            let mut current_line = String::new();

            for y in 0..height {
                for x in 0..width {
                    if is_selected(x, y) {
                        let cell = &buf[(x, y)];
                        if current_row != Some(y) {
                            if current_row.is_some() {
                                selected_text.push_str(current_line.trim_end());
                                selected_text.push('\n');
                                current_line.clear();
                            }
                            current_row = Some(y);
                        }
                        current_line.push_str(cell.symbol());
                    }
                }
            }
            if !current_line.is_empty() {
                selected_text.push_str(current_line.trim_end());
            }

            if !selected_text.is_empty() {
                let _ = crate::win32::copy_text_to_clipboard(&selected_text);
                let truncated = if selected_text.len() > 30 {
                    format!("{}...", &selected_text[..27].replace('\n', " "))
                } else {
                    selected_text.replace('\n', " ")
                };
                app.status = Some(crate::app::StatusMessage {
                    text: format!("{} Copied selection to clipboard: {}", app.glyphs.clipboard, truncated),
                    kind: crate::app::StatusKind::Info,
                });
            }

            app.selection_start = None;
            app.selection_end = None;
            app.selection_pending_copy = false;
        }
    }

    // 5. Scrollable Markdown Document Viewer Modal
    if let Some(ref filename) = app.show_markdown {
        let area = centered_rect(85, 80, frame.area());
        let popup_block = Block::default()
            .title(format!(
                " Document Viewer: {} (Press Esc/q to Close) ",
                filename
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent_primary));

        // Render text lines scrollable
        let paragraph = Paragraph::new(app.markdown_lines.clone())
            .block(popup_block)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .alignment(ratatui::layout::Alignment::Left)
            .scroll((app.markdown_scroll as u16, 0));

        frame.render_widget(Clear, area);
        frame.render_widget(paragraph, area);
    }

    // 6. Help Shortcuts Overlay Modal
    if app.show_help {
        let area = centered_rect(65, 75, frame.area());
        let popup_block = Block::default()
            .title(" Keyboard Shortcuts & TUI Commands ")
            .title_style(
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent_primary));

        let key_col_width = 18;
        let border_padding = 2;
        let total_inner_width = area.width.saturating_sub(border_padding);
        let max_desc_width = (total_inner_width as usize)
            .saturating_sub(key_col_width)
            .saturating_sub(2); // for ": "

        let mut help_text = Vec::new();
        help_text.push(Line::from(""));

        help_text.extend(format_help_row(
            "Tab / Shift-Tab",
            "Cycle active panel focus",
            max_desc_width,
            &theme,
        ));
        help_text.extend(format_help_row(
            "Up / Down",
            "Navigate lists and preference fields",
            max_desc_width,
            &theme,
        ));
        help_text.extend(format_help_row(
            "Left / Right",
            "Adjust settings and toggle flags",
            max_desc_width,
            &theme,
        ));
        help_text.extend(format_help_row(
            "Space / Enter",
            "Toggle screensaver selection / Apply settings",
            max_desc_width,
            &theme,
        ));
        help_text.extend(format_help_row(
            "p / t",
            "Preview highlighted screensaver",
            max_desc_width,
            &theme,
        ));
        help_text.extend(format_help_row(
            "c / C",
            "Configure highlighted screensaver",
            max_desc_width,
            &theme,
        ));
        help_text.extend(format_help_row(
            "d / D",
            "Delete downloaded screensaver from list",
            max_desc_width,
            &theme,
        ));
        help_text.extend(format_help_row(
            "r / F5",
            "Refresh screensavers list",
            max_desc_width,
            &theme,
        ));
        help_text.extend(format_help_row(
            "Esc / q",
            "Close dialogs / Help Overlay, or Quit application",
            max_desc_width,
            &theme,
        ));
        help_text.extend(format_help_row(
            "? / h",
            "Toggle this help shortcut overlay modal",
            max_desc_width,
            &theme,
        ));

        help_text.push(Line::from(""));
        help_text.extend(format_help_row(
            "F1",
            "View README.md document",
            max_desc_width,
            &theme,
        ));
        help_text.extend(format_help_row(
            "F2",
            "View SUPPORT.md document",
            max_desc_width,
            &theme,
        ));
        help_text.extend(format_help_row(
            "F3",
            "View LICENSE.md document",
            max_desc_width,
            &theme,
        ));
        help_text.extend(format_help_row(
            "F4",
            "View COPYRIGHT.md document",
            max_desc_width,
            &theme,
        ));

        help_text.push(Line::from(""));
        help_text.extend(format_help_row(
            "CLI Subcommands",
            "rsav.exe [tui | run | stop | toggle-active | lock | configure | preview | doctor]",
            max_desc_width,
            &theme,
        ));

        frame.render_widget(Clear, area);
        let paragraph = Paragraph::new(help_text).block(popup_block);
        frame.render_widget(paragraph, area);
    }
}

fn render_too_small(theme: crate::theme::TuiTheme, frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL);
    let (min_w, min_h) = crate::theme::recommended_min_size(96);
    let text = vec![
        Line::from(Span::styled(
            "Terminal too small",
            Style::default()
                .fg(theme.accent_secondary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "Need at least {min_w}x{min_h}, current {}x{}.",
            area.width, area.height
        )),
    ];
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

fn render_prefs(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme;
    let active = app.focused == FocusedSection::GlobalPrefs;
    let border_color = if active { theme.border_active } else { theme.border };

    let prefs_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            " Global Screensaver Preferences ",
            Style::default().fg(if active { theme.accent_primary } else { theme.header }).add_modifier(Modifier::BOLD),
        ));

    let active_status = if app.global.active { "ACTIVE" } else { "DISABLED" };
    let active_color = if app.global.active { theme.accent_secondary } else { theme.text_dim };
    
    let sleep_status = if app.local.prevent_sleep { "ACTIVE (SYSTEM AWAKE)" } else { "DISABLED (NORMAL)" };
    let sleep_color = if app.local.prevent_sleep { theme.accent_secondary } else { theme.text_dim };
    
    let hide_stock_status = if app.local.hide_stock { "YES" } else { "NO" };
    let hide_stock_color = if app.local.hide_stock { theme.accent_secondary } else { theme.text_dim };
    
    let timeout_value = format!("{} minutes", app.global.timeout / 60);
    let cycle_time_value = format!("{} seconds", app.local.random_cycle_secs);

    let mut lines = Vec::new();

    let mut add_field = |field: GlobalField, label: &str, value: String, value_color: Color| {
        let focused = active && app.global_field == field;
        let arrow_span = Span::styled(if focused { app.glyphs.play } else { "   " }, Style::default().fg(theme.accent_primary));
        let label_style = if focused {
            Style::default().fg(theme.accent_secondary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_main)
        };
        lines.push(Line::from(vec![
            arrow_span,
            Span::styled(label.to_string(), label_style),
            Span::styled(" ", Style::default()),
            Span::styled(value, Style::default().fg(value_color)),
        ]));
    };

    add_field(GlobalField::Active,       "Active:        ", active_status.to_string(), active_color);
    add_field(GlobalField::Timeout,      "Timeout:       ", timeout_value, theme.accent_primary);
    add_field(GlobalField::PreventSleep, "Prevent sleep: ", sleep_status.to_string(), sleep_color);
    add_field(GlobalField::CycleTime,    "Cycle time:    ", cycle_time_value, theme.accent_primary);
    add_field(GlobalField::HideStock,    "Hide stock:    ", hide_stock_status.to_string(), hide_stock_color);

    let prefs_inner = prefs_block.inner(area);
    frame.render_widget(prefs_block, area);
    frame.render_widget(Paragraph::new(lines), prefs_inner);
}

fn render_help(theme: crate::theme::TuiTheme, frame: &mut Frame, area: Rect) {
    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(
            " Help & Keyboard Shortcuts ",
            Style::default().fg(theme.header).add_modifier(Modifier::BOLD),
        ));

    let col1 = [
        ("Tab", "Focus"),
        ("↑/↓", "Move"),
        ("←/→", "Adjust"),
        ("Space/Enter", "Toggle/Apply"),
        ("? / H", "Help Info"),
    ];

    let col2 = [
        ("F5 / R", "Rescan"),
        ("P", "Preview"),
        ("C", "Config"),
        ("D", "Delete"),
        ("q/Esc", "Quit"),
    ];

    let mut lines = Vec::new();

    for i in 0..5 {
        let (k1, d1) = col1[i];
        let (k2, d2) = col2[i];
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<12}", k1), Style::default().fg(theme.accent_primary)),
            Span::raw(format!("{:<15}", d1)),
            Span::styled(format!("  {:<8}", k2), Style::default().fg(theme.accent_primary)),
            Span::raw(d2),
        ]));
    }

    let help_inner = help_block.inner(area);
    frame.render_widget(help_block, area);
    frame.render_widget(Paragraph::new(lines), help_inner);
}

fn render_list(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme;
    let active = app.focused == FocusedSection::SaverList;
    let border_color = if active { theme.border_active } else { theme.border };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            " Screensaver Preferences ",
            Style::default().fg(if active { theme.accent_primary } else { theme.header }).add_modifier(Modifier::BOLD),
        ));

    let list_inner = list_block.inner(area);
    frame.render_widget(list_block, area);

    let list_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Table Header
            Constraint::Min(1),    // List Items
        ])
        .split(list_inner);

    // Table Header Alignment to match the List items
    let header_line = Line::from(vec![
        Span::raw("   "),
        Span::styled("STATUS        ", if active { theme.accent_primary } else { theme.header }),
        Span::styled("LOCATION      ", Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD)),
        Span::styled("FRIENDLY NAME             ", Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD)),
        Span::styled("FILE NAME           ", Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD)),
        Span::styled("TYPE", Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD)),
    ]);
    frame.render_widget(Paragraph::new(header_line), list_chunks[0]);

    let indices = app.filtered_indices();

    if indices.is_empty() {
        let text = vec![
            Line::from("  No .scr files found."),
            Line::from(Span::styled(
                "  Drop one into %APPDATA%\\rSaver\\screensavers",
                Style::default().fg(theme.text_dim),
            )),
        ];
        frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), list_chunks[1]);
        return;
    }

    let total_items = indices.len();
    let visible_height = list_chunks[1].height as usize;
    let selected_pos = indices
        .iter()
        .position(|&i| i == app.highlighted)
        .unwrap_or(0);

    // Adjust list_offset to keep selected_pos in view
    if selected_pos < app.list_offset {
        app.list_offset = selected_pos;
    } else if selected_pos >= app.list_offset + visible_height {
        app.list_offset = selected_pos - visible_height + 1;
    }
    if app.list_offset + visible_height > total_items {
        app.list_offset = total_items.saturating_sub(visible_height);
    }

    let start = app.list_offset;
    let end = (start + visible_height).min(total_items);
    let visible_indices = &indices[start..end];

    let items: Vec<ListItem> = visible_indices
        .iter()
        .map(|&i| app.list_items[i].clone())
        .collect();

    let mut state = ListState::default().with_selected(Some(selected_pos.saturating_sub(start)));
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(theme.text_main)
                .bg(theme.bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(if active { app.glyphs.play } else { app.glyphs.play_empty });
    frame.render_stateful_widget(list, list_chunks[1], &mut state);
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    use ratatui::layout::{Constraint, Direction, Layout};
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn format_help_row(
    key: &str,
    description: &str,
    max_desc_width: usize,
    theme: &crate::theme::TuiTheme,
) -> Vec<Line<'static>> {
    let wrapped = wrap_text(description, max_desc_width);
    let mut lines = Vec::new();

    let key_col_width = 18;
    let key_str = format!("  {:<15} ", key);

    if wrapped.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                key_str,
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": ", Style::default().fg(theme.text_main)),
        ]));
    } else {
        for (i, chunk) in wrapped.into_iter().enumerate() {
            if i == 0 {
                lines.push(Line::from(vec![
                    Span::styled(
                        key_str.clone(),
                        Style::default()
                            .fg(theme.accent_primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(": ", Style::default().fg(theme.text_main)),
                    Span::styled(chunk, Style::default().fg(theme.text_main)),
                ]));
            } else {
                let padding = " ".repeat(key_col_width + 2);
                lines.push(Line::from(vec![
                    Span::styled(padding, Style::default().fg(theme.text_main)),
                    Span::styled(chunk, Style::default().fg(theme.text_main)),
                ]));
            }
        }
    }
    lines
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    if max_width == 0 {
        return vec![text.to_string()];
    }
    for paragraph in text.split('\n') {
        let mut current_line = String::new();
        for word in paragraph.split_whitespace() {
            if current_line.is_empty() {
                current_line.push_str(word);
            } else if current_line.len() + 1 + word.len() <= max_width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }
    lines
}
