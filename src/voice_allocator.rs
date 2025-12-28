use crate::voice::{Voice, VoiceId};

/// Allocates and manages polyphonic voices.
///
/// Responsibilities:
/// - map notes to voices
/// - manage voice lifetime
/// - expose active voices for graph execution
///
/// Does NOT:
/// - own DSP state
/// - allocate during processing
pub struct VoiceAllocator {
    voices: Vec<Voice>,
}

impl VoiceAllocator {
    pub fn new(max_voices: usize) -> Self {
        let voices = (0..max_voices).map(|id| Voice::new(id)).collect();

        Self { voices }
    }

    /// Allocate a voice for a note-on event.
    ///
    /// Returns the allocated voice id.
    pub fn note_on(&mut self, note: u8, velocity: f32) -> Option<VoiceId> {
        // Find a free voice
        if let Some(v) = self.voices.iter_mut().find(|v| !v.active) {
            v.active = true;
            v.note = note;
            v.velocity = velocity;
            Some(v.id)
        } else {
            // Voice stealing policy will go here
            None
        }
    }

    /// Release the voice associated with a note-off event.
    pub fn note_off(&mut self, note: u8) {
        if let Some(v) = self.voices.iter_mut().find(|v| v.active && v.note == note) {
            v.active = false;
        }
    }

    /// Iterate over active voices (immutable).
    pub fn active_voices(&self) -> impl Iterator<Item = &Voice> {
        self.voices.iter().filter(|v| v.active)
    }

    /// Iterate over active voices (mutable).
    ///
    /// Needed for per-voice DSP state updates.
    pub fn active_voices_mut(&mut self) -> impl Iterator<Item = &mut Voice> {
        self.voices.iter_mut().filter(|v| v.active)
    }
}
