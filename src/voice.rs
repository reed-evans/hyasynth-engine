pub type VoiceId = usize;

/// A voice represents one active note / execution context.
///
/// Voices do NOT own DSP state.
/// DSP state lives in per-voice node instances.
#[derive(Debug)]
pub struct Voice {
    pub id: VoiceId,
    pub active: bool,
    pub note: u8,
    pub velocity: f32,
}

impl Voice {
    #[inline]
    pub fn new(id: VoiceId) -> Self {
        Self {
            id,
            active: false,
            note: 0,
            velocity: 0.0,
        }
    }
}
