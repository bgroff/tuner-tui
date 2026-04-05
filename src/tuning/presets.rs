use super::note::Note;

/// Represents a string on an instrument with its display label and MIDI number.
#[derive(Debug, Clone)]
pub struct InstrumentString {
    pub label: &'static str,
    pub midi: i32,
}

/// Returns the standard tuning strings for mandolin, ordered low to high.
/// G3 (196 Hz), D4 (294 Hz), A4 (440 Hz), E5 (659 Hz)
pub fn mandolin_strings() -> Vec<InstrumentString> {
    vec![
        InstrumentString { label: "G", midi: 55 }, // G3
        InstrumentString { label: "D", midi: 62 }, // D4
        InstrumentString { label: "A", midi: 69 }, // A4
        InstrumentString { label: "E", midi: 76 }, // E5
    ]
}

/// Returns the standard tuning strings for guitar, ordered low to high.
pub fn guitar_strings() -> Vec<InstrumentString> {
    vec![
        InstrumentString { label: "E", midi: 40 }, // E2
        InstrumentString { label: "A", midi: 45 }, // A2
        InstrumentString { label: "D", midi: 50 }, // D3
        InstrumentString { label: "G", midi: 55 }, // G3
        InstrumentString { label: "B", midi: 59 }, // B3
        InstrumentString { label: "E", midi: 64 }, // E4
    ]
}

/// Find the closest string to a given frequency for an instrument's strings.
/// Returns (string_index, note, cents_offset).
pub fn closest_string(
    strings: &[InstrumentString],
    frequency: f64,
    reference_pitch: f64,
) -> Option<(usize, Note, f64)> {
    let mut best: Option<(usize, Note, f64)> = None;
    let mut best_distance = f64::MAX;

    for (i, s) in strings.iter().enumerate() {
        let note = Note::from_midi(s.midi, reference_pitch);
        let cents = 1200.0 * (frequency / note.frequency).log2();
        let distance = cents.abs();

        if distance < best_distance {
            best_distance = distance;
            best = Some((i, note, cents));
        }
    }

    best
}
