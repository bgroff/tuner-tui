use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, Selector};

use super::meter::TunerMeter;

fn selector_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    }
}

fn selector_brackets(label: &str, value: &str, active: bool) -> Line<'static> {
    let style = selector_style(active);
    if active {
        Line::from(vec![
            Span::styled(format!("{}: ", label), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("\u{25c0} {} \u{25b6}", value), style),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!("{}: ", label), Style::default().fg(Color::DarkGray)),
            Span::styled(value.to_string(), style),
        ])
    }
}

/// Render the string note selector as individual note buttons.
/// Selected note gets a green box, closest detected string gets a dim highlight.
fn string_notes_line(app: &App, active: bool) -> Line<'static> {
    let strings = match app.mode.strings() {
        Some(s) => s,
        None => return Line::from(""),
    };

    let mut spans = Vec::new();
    spans.push(Span::styled("  ", Style::default()));

    for (i, s) in strings.iter().enumerate() {
        let is_selected = i == app.selected_string;
        let is_closest = app.closest_string_index == Some(i);

        if is_selected {
            // Green box around the selected note
            spans.push(Span::styled(
                format!(" {} ", s.label),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));
        } else if is_closest && app.detected_frequency.is_some() {
            // Dim highlight for the closest detected string
            spans.push(Span::styled(
                format!(" {} ", s.label),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray),
            ));
        } else {
            let fg = if active { Color::White } else { Color::DarkGray };
            spans.push(Span::styled(
                format!(" {} ", s.label),
                Style::default().fg(fg),
            ));
        }

        spans.push(Span::styled(" ", Style::default()));
    }

    if active {
        // Show arrow hints when this row is focused
        spans.insert(0, Span::styled("\u{25c0} ", Style::default().fg(Color::Cyan)));
        spans.push(Span::styled(" \u{25b6}", Style::default().fg(Color::Cyan)));
    }

    Line::from(spans)
}

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let outer_block = Block::default()
        .title(" Tuner ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    let has_strings = app.mode.strings().is_some();

    let chunks = Layout::vertical([
        Constraint::Length(1), // Input device
        Constraint::Length(1), // Channel
        Constraint::Length(1), // Algorithm + Mode
        Constraint::Length(if has_strings { 1 } else { 0 }), // String notes (conditional)
        Constraint::Length(1), // Spacer
        Constraint::Length(3), // Meter
        Constraint::Length(1), // Spacer
        Constraint::Length(1), // Note display
        Constraint::Length(1), // Frequency display
        Constraint::Length(1), // Closest string info
        Constraint::Min(0),   // Help
    ])
    .split(inner);

    // Row 0: Input device
    let device_name = app
        .devices
        .get(app.selected_device)
        .map(|d| d.name.clone())
        .unwrap_or_else(|| "No device".to_string());
    let input_line = selector_brackets("Input", &device_name, app.active_selector == Selector::Input);
    frame.render_widget(Paragraph::new(input_line), chunks[0]);

    // Row 1: Channel
    let channel_str = app.channel_mode.display(app.device_channels);
    let channel_line = selector_brackets("Channel", &channel_str, app.active_selector == Selector::Channel);
    frame.render_widget(Paragraph::new(channel_line), chunks[1]);

    // Row 2: Algorithm + Mode
    let row2 = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    let algo_line = selector_brackets(
        "Algorithm",
        &app.algorithm.to_string(),
        app.active_selector == Selector::Algorithm,
    );
    frame.render_widget(Paragraph::new(algo_line), row2[0]);

    let mode_line = selector_brackets(
        "Mode",
        &app.mode.to_string(),
        app.active_selector == Selector::Mode,
    );
    frame.render_widget(Paragraph::new(mode_line), row2[1]);

    // Row 3: String notes (only shown in instrument modes)
    if has_strings {
        let notes_line = string_notes_line(app, app.active_selector == Selector::StringNote);
        frame.render_widget(
            Paragraph::new(notes_line).alignment(Alignment::Center),
            chunks[3],
        );
    }

    // Row 5: Meter
    let cents = app.display_cents();
    let meter = TunerMeter {
        cents,
        active: app.detected_frequency.is_some(),
    };
    frame.render_widget(meter, chunks[5]);

    // Row 7: Note display
    let note_text = if let Some(ref note) = app.detected_note {
        let color = if cents.abs() < 5.0 {
            Color::Green
        } else if cents.abs() < 15.0 {
            Color::Yellow
        } else {
            Color::Red
        };
        Line::from(vec![
            Span::styled(
                note.display_name(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {:+.1}\u{00a2}", cents),
                Style::default().fg(color),
            ),
        ])
    } else {
        Line::from(Span::styled(
            "---",
            Style::default().fg(Color::DarkGray),
        ))
    };
    frame.render_widget(
        Paragraph::new(note_text).alignment(Alignment::Center),
        chunks[7],
    );

    // Row 8: Frequency display
    let freq_text = if let Some(freq) = app.detected_frequency {
        Line::from(Span::styled(
            format!("{:.1} Hz", freq),
            Style::default().fg(Color::White),
        ))
    } else {
        Line::from(Span::styled(
            "--- Hz",
            Style::default().fg(Color::DarkGray),
        ))
    };
    frame.render_widget(
        Paragraph::new(freq_text).alignment(Alignment::Center),
        chunks[8],
    );

    // Row 9: Closest string info (in instrument modes)
    if has_strings {
        if let Some(strings) = app.mode.strings() {
            if let Some(idx) = app.closest_string_index {
                if let Some(s) = strings.get(idx) {
                    let closest_note = crate::tuning::Note::from_midi(s.midi, app.reference_pitch);
                    let string_text = Line::from(Span::styled(
                        format!("Nearest string: {} ({})", s.label, closest_note.display_name()),
                        Style::default().fg(Color::DarkGray),
                    ));
                    frame.render_widget(
                        Paragraph::new(string_text).alignment(Alignment::Center),
                        chunks[9],
                    );
                }
            }
        }
    }

    // Help line at bottom
    if chunks[10].height > 0 {
        let help = Line::from(Span::styled(
            "Tab/Shift+Tab: select  \u{2190}/\u{2192}: change  q: quit",
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(
            Paragraph::new(help).alignment(Alignment::Center),
            chunks[10],
        );
    }
}
