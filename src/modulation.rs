/// A read-only view of a modulation signal.
///
/// This is intentionally lightweight:
/// - no ownership
/// - no allocation
/// - no dynamic dispatch
#[derive(Copy, Clone)]
pub enum ModSignal<'a> {
    /// Fixed value (e.g. knob, constant offset)
    Constant(f32),

    /// One value per slice (control-rate)
    Control(&'a [f32]),

    /// One value per sample (audio-rate)
    Audio(&'a [f32]),
}

impl<'a> ModSignal<'a> {
    #[inline]
    pub fn value_control(&self) -> f32 {
        match *self {
            ModSignal::Constant(v) => v,
            ModSignal::Control(buf) => buf[0],
            ModSignal::Audio(buf) => buf[0],
        }
    }

    #[inline]
    pub fn value_audio(&self, frame: usize) -> f32 {
        match *self {
            ModSignal::Constant(v) => v,
            ModSignal::Control(buf) => buf[0],
            ModSignal::Audio(buf) => buf[frame],
        }
    }
}
