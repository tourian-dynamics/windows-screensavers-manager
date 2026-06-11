//! Markdown text parsing utilities for rendering embedded documents.
//!
//! **Taxonomy Classification**: Interface (Document Parsing).

use crate::theme::TuiTheme;

/// Parse a raw markdown string into styled Ratatui Line objects.
pub fn parse_markdown_to_lines(content: &str, theme: &TuiTheme) -> Vec<ratatui::text::Line<'static>> {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};

    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut current_paragraph = String::new();

    let flush_paragraph = |para: &mut String, lines: &mut Vec<Line<'static>>| {
        if !para.is_empty() {
            lines.push(Line::from(Span::styled(
                para.clone(),
                Style::default().fg(theme.text_main),
            )));
            para.clear();
        }
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            flush_paragraph(&mut current_paragraph, &mut lines);
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(Color::Rgb(150, 240, 150)),
            )));
            continue;
        }

        if trimmed.is_empty() {
            flush_paragraph(&mut current_paragraph, &mut lines);
            lines.push(Line::from(""));
            continue;
        }

        if let Some(header) = trimmed.strip_prefix("# ") {
            flush_paragraph(&mut current_paragraph, &mut lines);
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("=== {} ===", header.to_uppercase()),
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
        } else if let Some(header) = trimmed.strip_prefix("## ") {
            flush_paragraph(&mut current_paragraph, &mut lines);
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("--- {} ---", header),
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
        } else if let Some(header) = trimmed.strip_prefix("### ") {
            flush_paragraph(&mut current_paragraph, &mut lines);
            lines.push(Line::from(Span::styled(
                header.to_string(),
                Style::default().fg(theme.accent_primary),
            )));
        } else if let Some(item) = trimmed.strip_prefix("* ").or_else(|| trimmed.strip_prefix("- ")) {
            flush_paragraph(&mut current_paragraph, &mut lines);
            lines.push(Line::from(vec![
                Span::styled(" • ", Style::default().fg(theme.accent_primary)),
                Span::styled(item.to_string(), Style::default().fg(theme.text_main)),
            ]));
        } else if trimmed.starts_with("1. ")
            || trimmed.starts_with("2. ")
            || trimmed.starts_with("3. ")
            || trimmed.starts_with("4. ")
            || trimmed.starts_with("5. ")
        {
            flush_paragraph(&mut current_paragraph, &mut lines);
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", &trimmed[..3]),
                    Style::default().fg(theme.accent_primary),
                ),
                Span::styled(trimmed[3..].to_string(), Style::default().fg(theme.text_main)),
            ]));
        } else {
            if !current_paragraph.is_empty() {
                current_paragraph.push(' ');
            }
            current_paragraph.push_str(trimmed);
        }
    }

    flush_paragraph(&mut current_paragraph, &mut lines);
    lines
}
