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

/// Accumulated per-sample modulation data for the node currently being processed.
///
/// Before processing each node, the graph scans compiled mod routes targeting it,
/// reads source output buffers, scales by depth, and accumulates into this buffer.
/// Nodes query per-sample modulation offsets by parameter ID during `process()`.
///
/// # Usage in nodes
///
/// ```ignore
/// fn process(&mut self, ctx: &ProcessContext, inputs: &[&AudioBuffer], output: &mut AudioBuffer) -> bool {
///     if let Some(mod_ctx) = ctx.modulation {
///         if let Some(freq_mod) = mod_ctx.get(PARAM_FREQ) {
///             // freq_mod[i] is the per-sample modulation offset for frame i
///             let modulated_freq = self.freq + freq_mod[i];
///         }
///     }
///     // ...
/// }
/// ```
#[derive(Debug)]
pub struct ModContext {
    /// (param_id, start_index_in_data) for each modulated parameter.
    entries: Vec<(u32, usize)>,
    /// Flat storage of per-sample modulation values.
    data: Vec<f32>,
    /// Frame count for the current block.
    frames: usize,
}

impl Default for ModContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ModContext {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            data: Vec::new(),
            frames: 0,
        }
    }

    /// Clear all modulation data for a new node.
    #[inline]
    pub fn clear(&mut self, frames: usize) {
        self.entries.clear();
        self.data.clear();
        self.frames = frames;
    }

    /// Accumulate a scaled modulation signal for a parameter.
    ///
    /// Multiple routes targeting the same parameter are summed.
    pub fn accumulate(&mut self, param_id: u32, source: &[f32], depth: f32) {
        let frames = self.frames;

        // Find existing entry or create one
        let offset = match self.entries.iter().find(|(id, _)| *id == param_id) {
            Some(&(_, offset)) => offset,
            None => {
                let offset = self.data.len();
                self.data.resize(offset + frames, 0.0);
                self.entries.push((param_id, offset));
                offset
            }
        };

        let slot = &mut self.data[offset..offset + frames];
        let len = slot.len().min(source.len());
        for i in 0..len {
            slot[i] += source[i] * depth;
        }
    }

    /// Get the per-sample modulation offset for a parameter.
    ///
    /// Returns `None` if no modulation targets this parameter.
    #[inline]
    pub fn get(&self, param_id: u32) -> Option<&[f32]> {
        self.entries
            .iter()
            .find(|(id, _)| *id == param_id)
            .map(|&(_, offset)| &self.data[offset..offset + self.frames])
    }

    /// Check if any modulation data exists.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
