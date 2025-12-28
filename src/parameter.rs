use crate::modulation::ModSignal;

/// One modulation input to a parameter
#[derive(Copy, Clone)]
pub struct ModulationInput<'a> {
    pub signal: ModSignal<'a>,
    pub depth: f32,
}

/// A strongly typed parameter with implicit modulation.
///
/// Parameters do not own their modulation sources.
/// They only read from them.
pub struct Parameter<'a> {
    base: f32,
    mods: [Option<ModulationInput<'a>>; 8],
}

impl<'a> Parameter<'a> {
    pub fn new(base: f32) -> Self {
        Self {
            base,
            mods: Default::default(),
        }
    }

    /// Set the base value (e.g. UI control)
    #[inline]
    pub fn set_base(&mut self, value: f32) {
        self.base = value;
    }

    /// Attach a modulation source to a slot
    #[inline]
    pub fn set_mod(
        &mut self,
        slot: usize,
        signal: ModSignal<'a>,
        depth: f32,
    ) {
        self.mods[slot] = Some(ModulationInput { signal, depth });
    }

    /// Evaluate at control rate (once per slice)
    #[inline]
    pub fn value_control(&self) -> f32 {
        let mut v = self.base;
        for m in self.mods.iter().flatten() {
            v += m.signal.value_control() * m.depth;
        }
        v
    }

    /// Evaluate at audio rate (per sample)
    #[inline]
    pub fn value_audio(&self, frame: usize) -> f32 {
        let mut v = self.base;
        for m in self.mods.iter().flatten() {
            v += m.signal.value_audio(frame) * m.depth;
        }
        v
    }
}
