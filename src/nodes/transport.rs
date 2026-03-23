// Transport node — outputs timing signals for sequencing and polymeters.
//
// Channel 0: Phase ramp (0.0→1.0 sawtooth, resets every cycle)
// Channel 1: Trigger pulse (1.0 on the reset sample, 0.0 otherwise)

use crate::audio_buffer::AudioBuffer;
use crate::node::{Node, ProcessContext};

/// Transport sync mode.
#[derive(Debug, Clone, Copy, PartialEq)]
enum TransportMode {
    /// Synced to global transport — resets on play, pauses on stop.
    Synced,
    /// Free-running — ignores global transport state.
    FreeRunning,
}

pub struct TransportNode {
    /// Beat rate relative to global BPM (1.0 = quarter note).
    rate: f64,
    /// Sync mode.
    mode: TransportMode,
    /// Phase accumulator (0.0–1.0), f64 for long-session precision.
    phase: f64,
    /// Whether transport was playing last block (for detecting play edge).
    was_playing: bool,
}

impl TransportNode {
    pub fn new() -> Self {
        Self {
            rate: 1.0,
            mode: TransportMode::Synced,
            phase: 0.0,
            was_playing: false,
        }
    }
}

impl Default for TransportNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for TransportNode {
    fn prepare(&mut self, _sample_rate: f64, _max_block: usize) {}

    fn process(
        &mut self,
        ctx: &ProcessContext,
        _inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        let playing = ctx.playing;

        // Synced mode: reset phase on play edge, output silence when stopped
        if self.mode == TransportMode::Synced {
            if playing && !self.was_playing {
                self.phase = 0.0;
            }
            self.was_playing = playing;

            if !playing {
                output.clear();
                return true;
            }
        }

        // Phase increment per sample: (bpm / 60) * rate / sample_rate
        // = beats_per_second * rate / sample_rate
        let phase_inc = (ctx.bpm * self.rate) / (60.0 * ctx.sample_rate);

        let frames = ctx.frames;
        let data = output.samples_mut();

        for i in 0..frames {
            // Planar layout: [phase_0..phase_n, trigger_0..trigger_n]
            data[i] = self.phase as f32;

            self.phase += phase_inc;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
                data[frames + i] = 1.0;
            } else {
                data[frames + i] = 0.0;
            }
        }

        false
    }

    fn num_channels(&self) -> usize {
        2
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            0 => self.rate = (value as f64).max(0.001), // Rate
            1 => {
                // Mode: 0 = synced, 1 = free-running
                self.mode = if value > 0.5 {
                    TransportMode::FreeRunning
                } else {
                    TransportMode::Synced
                };
            }
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.was_playing = false;
    }
}
