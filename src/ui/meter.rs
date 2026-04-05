use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

pub struct TunerMeter {
    /// Cents offset from target: -50 to +50
    pub cents: f64,
    /// Whether a pitch is currently detected
    pub active: bool,
}

impl Widget for TunerMeter {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 3 {
            return;
        }

        let meter_width = (area.width - 2) as usize; // leave padding
        let center = meter_width / 2;

        // Draw scale labels
        let label_y = area.y;
        let flat_label = "\u{266d} flat";
        let sharp_label = "sharp \u{266f}";
        let center_label = "\u{2502}";

        buf.set_string(area.x + 1, label_y, flat_label, Style::default().fg(Color::DarkGray));
        buf.set_string(
            area.x + area.width - 1 - sharp_label.len() as u16,
            label_y,
            sharp_label,
            Style::default().fg(Color::DarkGray),
        );
        buf.set_string(
            area.x + 1 + center as u16,
            label_y,
            center_label,
            Style::default().fg(Color::White),
        );

        // Draw the meter bar
        let bar_y = area.y + 1;

        // Draw track
        for x in 0..meter_width {
            let ch = if x == center { '\u{2502}' } else { '\u{2500}' };
            let color = if x == center {
                Color::White
            } else {
                Color::DarkGray
            };
            buf.set_string(
                area.x + 1 + x as u16,
                bar_y,
                ch.to_string(),
                Style::default().fg(color),
            );
        }

        // Draw needle if active
        if self.active {
            let clamped = self.cents.clamp(-50.0, 50.0);
            let needle_pos = center as f64 + (clamped / 50.0) * center as f64;
            let needle_x = (needle_pos as usize).clamp(0, meter_width - 1);

            let color = if self.cents.abs() < 5.0 {
                Color::Green
            } else if self.cents.abs() < 15.0 {
                Color::Yellow
            } else {
                Color::Red
            };

            buf.set_string(
                area.x + 1 + needle_x as u16,
                bar_y,
                "\u{2588}",
                Style::default().fg(color),
            );
        }

        // Draw cents markers
        let marker_y = area.y + 2;
        let markers = [("-50", 0), ("-25", center / 2), ("0", center), ("+25", center + center / 2), ("+50", meter_width - 3)];
        for (label, pos) in markers {
            let x = (pos as u16).min(area.width - 1 - label.len() as u16);
            buf.set_string(
                area.x + 1 + x,
                marker_y,
                label,
                Style::default().fg(Color::DarkGray),
            );
        }
    }
}
