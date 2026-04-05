const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub name: String,
    pub octave: i32,
    pub frequency: f64,
    pub midi_number: i32,
}

impl Note {
    pub fn from_midi(midi: i32, reference_pitch: f64) -> Self {
        let semitones_from_a4 = midi - 69;
        let frequency = reference_pitch * 2.0f64.powf(semitones_from_a4 as f64 / 12.0);
        let note_index = ((midi % 12) + 12) % 12;
        let octave = (midi / 12) - 1;
        let name = NOTE_NAMES[note_index as usize].to_string();

        Self {
            name,
            octave,
            frequency,
            midi_number: midi,
        }
    }

    pub fn display_name(&self) -> String {
        format!("{}{}", self.name, self.octave)
    }
}

impl std::fmt::Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

pub struct NoteDetection;

impl NoteDetection {
    /// Find the nearest note to a given frequency.
    /// Returns (note, cents_offset) where cents_offset is negative if flat, positive if sharp.
    pub fn detect(frequency: f64, reference_pitch: f64) -> (Note, f64) {
        // Calculate MIDI number from frequency
        let midi_float = 69.0 + 12.0 * (frequency / reference_pitch).log2();
        let midi_rounded = midi_float.round() as i32;
        let cents = (midi_float - midi_rounded as f64) * 100.0;
        let note = Note::from_midi(midi_rounded, reference_pitch);

        (note, cents)
    }
}
