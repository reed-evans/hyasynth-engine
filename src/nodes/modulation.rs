// src/nodes/modulation.rs
//
// Modulation sources like LFOs.

use crate::audio_buffer::AudioBuffer;
use crate::node::{Node, ProcessContext};
use std::f32::consts::PI;

/// LFO waveform types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LfoWaveform {
    Sine,
    Triangle,
    Saw,
    Square,
    SampleAndHold,
}

/// Low Frequency Oscillator for modulation.
///
/// Outputs a control signal that can modulate other parameters.
/// The output range is -1.0 to 1.0, scaled by the depth parameter.
pub struct Lfo {
    rate: f32,        // Hz
    depth: f32,       // 0.0 - 1.0
    waveform: LfoWaveform,
    phase: f32,       // 0.0 - 1.0
    sync_to_transport: bool,

    // For sample & hold
    sh_value: f32,
    sh_last_phase: f32,
    rng_state: u32,
}

impl Lfo {
    /// Simple xorshift random number generator
    fn next_random(&mut self) -> f32 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng_state = x;
        // Convert to 0.0 - 1.0
        (x as f32) / (u32::MAX as f32)
    }
}

impl Lfo {
    pub fn new() -> Self {
        Self {
            rate: 1.0,
            depth: 1.0,
            waveform: LfoWaveform::Sine,
            phase: 0.0,
            sync_to_transport: false,
            sh_value: 0.0,
            sh_last_phase: 0.0,
            rng_state: 0x12345678,
        }
    }

    fn generate_sample(&mut self) -> f32 {
        let raw = match self.waveform {
            LfoWaveform::Sine => (self.phase * 2.0 * PI).sin(),
            LfoWaveform::Triangle => {
                if self.phase < 0.5 {
                    4.0 * self.phase - 1.0
                } else {
                    3.0 - 4.0 * self.phase
                }
            }
            LfoWaveform::Saw => 2.0 * self.phase - 1.0,
            LfoWaveform::Square => {
                if self.phase < 0.5 { 1.0 } else { -1.0 }
            }
            LfoWaveform::SampleAndHold => {
                // Update S&H value when phase wraps
                if self.phase < self.sh_last_phase {
                    self.sh_value = self.next_random() * 2.0 - 1.0;
                }
                self.sh_last_phase = self.phase;
                self.sh_value
            }
        };

        raw * self.depth
    }
}

impl Default for Lfo {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for Lfo {
    fn prepare(&mut self, _sample_rate: f64, _max_block: usize) {}

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            0 => self.rate = value.max(0.001),      // Rate in Hz
            1 => self.depth = value.clamp(0.0, 1.0), // Depth
            2 => {
                // Waveform (0=sine, 1=tri, 2=saw, 3=square, 4=s&h)
                self.waveform = match value as u32 {
                    0 => LfoWaveform::Sine,
                    1 => LfoWaveform::Triangle,
                    2 => LfoWaveform::Saw,
                    3 => LfoWaveform::Square,
                    _ => LfoWaveform::SampleAndHold,
                };
            }
            3 => self.phase = value.clamp(0.0, 1.0), // Initial phase
            4 => self.sync_to_transport = value > 0.5,
            _ => {}
        }
    }

    fn process(
        &mut self,
        ctx: &ProcessContext,
        _inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        let phase_inc = self.rate / ctx.sample_rate as f32;
        let out_ch = output.channel_mut(0);

        for i in 0..ctx.frames {
            out_ch[i] = self.generate_sample();
            self.phase += phase_inc;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }

        true
    }

    fn num_channels(&self) -> usize {
        1
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.sh_value = 0.0;
        self.sh_last_phase = 0.0;
    }
}

