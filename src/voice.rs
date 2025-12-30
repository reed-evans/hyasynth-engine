pub type VoiceId = usize;

/// A voice represents one active note / execution context.
///
/// Voices do NOT own DSP state.
/// DSP state lives in per-voice node instances.
#[derive(Debug, Clone)]
pub struct Voice {
    pub id: VoiceId,
    pub active: bool,
    pub note: u8,
    pub velocity: f32,

    /// Gate is high while note is held
    pub gate: bool,

    /// Trigger is high for one block after note-on
    pub trigger: bool,

    /// Release is high for one block after note-off  
    pub release: bool,
}

impl Voice {
    #[inline]
    pub fn new(id: VoiceId) -> Self {
        Self {
            id,
            active: false,
            note: 0,
            velocity: 0.0,
            gate: false,
            trigger: false,
            release: false,
        }
    }

    /// Called at start of each block to clear one-shot flags
    #[inline]
    pub fn clear_triggers(&mut self) {
        self.trigger = false;
        self.release = false;
    }

    /// Trigger note on
    #[inline]
    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.active = true;
        self.note = note;
        self.velocity = velocity;
        self.gate = true;
        self.trigger = true;
        self.release = false;
    }

    /// Trigger note off (voice stays active for release phase)
    #[inline]
    pub fn note_off(&mut self) {
        self.gate = false;
        self.release = true;
    }

    /// Fully deactivate voice (after release complete)
    #[inline]
    pub fn deactivate(&mut self) {
        self.active = false;
        self.gate = false;
        self.trigger = false;
        self.release = false;
    }
}

/// Read-only voice context passed to nodes during processing.
///
/// This is a lightweight view of Voice state.
#[derive(Debug, Clone, Copy)]
pub struct VoiceContext {
    pub id: VoiceId,
    pub note: u8,
    pub velocity: f32,
    pub gate: bool,
    pub trigger: bool,
    pub release: bool,
}

impl From<&Voice> for VoiceContext {
    fn from(v: &Voice) -> Self {
        Self {
            id: v.id,
            note: v.note,
            velocity: v.velocity,
            gate: v.gate,
            trigger: v.trigger,
            release: v.release,
        }
    }
}
