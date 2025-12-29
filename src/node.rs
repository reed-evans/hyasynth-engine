// src/node.rs

use crate::audio_buffer::AudioBuffer;
use crate::voice::VoiceContext;

/// Node instancing strategy
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Polyphony {
    /// One shared instance (effects, output)
    Global,

    /// One instance per voice (oscillators, envelopes)
    PerVoice,
}

/// Context passed to nodes during processing.
#[derive(Debug, Clone, Copy)]
pub struct ProcessContext<'a> {
    /// Number of frames to process
    pub frames: usize,
    
    /// Sample rate
    pub sample_rate: f64,
    
    /// Current sample position
    pub sample_pos: u64,
    
    /// Voice context (only present for PerVoice nodes)
    pub voice: Option<VoiceContext>,
    
    /// Tempo in BPM
    pub bpm: f64,
    
    /// Marker for lifetime
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> ProcessContext<'a> {
    pub fn new(frames: usize, sample_rate: f64, sample_pos: u64, bpm: f64) -> Self {
        Self {
            frames,
            sample_rate,
            sample_pos,
            bpm,
            voice: None,
            _marker: std::marker::PhantomData,
        }
    }
    
    pub fn with_voice(mut self, voice: VoiceContext) -> Self {
        self.voice = Some(voice);
        self
    }
}

/// Core DSP node trait.
///
/// Nodes:
/// - do NOT know about scheduling
/// - do NOT allocate
/// - do NOT dispatch events
/// - ONLY process audio for the given context
pub trait Node: Send {
    /// Called once before playback starts or when the graph recompiles.
    fn prepare(&mut self, sample_rate: f64, max_block: usize);

    /// Process audio.
    ///
    /// The engine guarantees:
    /// - all events for this slice have already been dispatched
    /// - inputs contain valid data from upstream nodes
    ///
    /// Arguments:
    /// - `ctx`: Processing context (frames, sample rate, voice info)
    /// - `inputs`: Buffers from connected input nodes (may be empty for sources)
    /// - `output`: Buffer to write output to
    ///
    /// Returns `true` if the output is silent (optimization hint).
    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool;

    /// Number of output channels.
    fn num_channels(&self) -> usize;

    /// Polyphony behavior for this node.
    fn polyphony(&self) -> Polyphony {
        Polyphony::Global
    }

    /// Set a parameter value.
    fn set_param(&mut self, param_id: u32, value: f32);
    
    /// Reset node state (called on transport stop/seek).
    fn reset(&mut self) {}
}
