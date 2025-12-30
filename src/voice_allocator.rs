//! Polyphonic voice allocation and management.
//!
//! The voice allocator maps MIDI notes to voices, manages voice lifecycles,
//! and exposes active voices for per-voice processing in the audio graph.

use crate::voice::{Voice, VoiceContext, VoiceId};

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
        let voices = (0..max_voices).map(Voice::new).collect();
        Self { voices }
    }

    /// Clear one-shot trigger flags at start of each block.
    pub fn clear_triggers(&mut self) {
        for voice in &mut self.voices {
            voice.clear_triggers();
        }
    }

    /// Allocate a voice for a note-on event.
    ///
    /// Returns the allocated voice id.
    pub fn note_on(&mut self, note: u8, velocity: f32) -> Option<VoiceId> {
        // First, try to find an inactive voice
        if let Some(v) = self.voices.iter_mut().find(|v| !v.active) {
            v.note_on(note, velocity);
            return Some(v.id);
        }

        // Voice stealing: when all voices are active, steal the oldest one.
        // A more sophisticated policy could steal voices in release phase first,
        // or prioritize stealing quieter voices.
        if let Some(v) = self.voices.first_mut() {
            v.note_on(note, velocity);
            return Some(v.id);
        }

        None
    }

    /// Release the voice associated with a note-off event.
    pub fn note_off(&mut self, note: u8) {
        if let Some(v) = self
            .voices
            .iter_mut()
            .find(|v| v.active && v.gate && v.note == note)
        {
            v.note_off();
        }
    }

    /// Deactivate a voice (called when envelope finishes release).
    pub fn deactivate(&mut self, voice_id: VoiceId) {
        if let Some(v) = self.voices.get_mut(voice_id) {
            v.deactivate();
        }
    }

    /// Iterate over active voices.
    pub fn active_voices(&self) -> impl Iterator<Item = VoiceContext> + '_ {
        self.voices
            .iter()
            .filter(|v| v.active)
            .map(VoiceContext::from)
    }

    /// Get a specific voice's context.
    pub fn get_voice(&self, id: VoiceId) -> Option<VoiceContext> {
        self.voices.get(id).map(VoiceContext::from)
    }

    /// Number of currently active voices.
    pub fn active_count(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }
}
