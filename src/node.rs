// src/node.rs

use crate::{audio_buffer::AudioBuffer, execution_plan::SlicePlan};

/// Node instancing strategy
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Polyphony {
    /// One shared instance
    Global,

    /// One instance per voice
    PerVoice,
}

/// Core DSP node trait.
///
/// Nodes:
/// - do NOT know about scheduling
/// - do NOT allocate
/// - do NOT dispatch events
/// - ONLY process audio for the given slice
pub trait Node {
    /// Called once before playback starts or when the graph recompiles.
    fn prepare(&mut self, sample_rate: f64, max_block: usize);

    /// Process one scheduled slice of audio.
    ///
    /// The engine guarantees:
    /// - all events for this slice have already been dispatched
    /// - transport reflects the correct sample position
    ///
    /// Returns `true` if the output is silent.
    fn process_slice(&mut self, slice: &SlicePlan, output: &mut AudioBuffer) -> bool;

    /// Number of output channels.
    fn num_channels(&self) -> usize;

    /// Polyphony behavior for this node.
    ///
    /// Defaults to Global.
    fn polyphony(&self) -> Polyphony {
        Polyphony::Global
    }

    fn set_param(&mut self, param_id: u32, value: f32);
}
