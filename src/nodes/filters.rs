// src/nodes/filters.rs
//
// Filter nodes using State Variable Filter (SVF) topology.
// This provides lowpass, highpass, and bandpass outputs.

use crate::audio_buffer::AudioBuffer;
use crate::node::{Node, ProcessContext};

/// Filter type for the SVF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    Lowpass,
    Highpass,
    Bandpass,
    Notch,
}

/// State Variable Filter implementation.
///
/// The SVF is a versatile, high-quality filter that can produce
/// multiple filter types simultaneously with excellent stability.
pub struct SvfFilter {
    filter_type: FilterType,
    cutoff: f32,
    resonance: f32,

    // Filter state
    ic1eq: f32,
    ic2eq: f32,

    // Cached coefficients
    g: f32,
    k: f32,
    a1: f32,
    a2: f32,
    a3: f32,

    last_sample_rate: f64,
}

impl SvfFilter {
    pub fn new(filter_type: FilterType) -> Self {
        Self {
            filter_type,
            cutoff: 1000.0,
            resonance: 0.5,
            ic1eq: 0.0,
            ic2eq: 0.0,
            g: 0.0,
            k: 0.0,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            last_sample_rate: 0.0,
        }
    }

    pub fn lowpass() -> Self {
        Self::new(FilterType::Lowpass)
    }

    pub fn highpass() -> Self {
        Self::new(FilterType::Highpass)
    }

    pub fn bandpass() -> Self {
        Self::new(FilterType::Bandpass)
    }

    pub fn notch() -> Self {
        Self::new(FilterType::Notch)
    }

    fn update_coefficients(&mut self, sample_rate: f64) {
        if (self.last_sample_rate - sample_rate).abs() < 0.1 {
            return;
        }

        self.last_sample_rate = sample_rate;
        self.recalc_coeffs();
    }

    fn recalc_coeffs(&mut self) {
        // Clamp cutoff to valid range
        let cutoff = self.cutoff.clamp(20.0, (self.last_sample_rate as f32 * 0.49).max(20.0));

        // Resonance clamped to prevent self-oscillation issues
        let resonance = self.resonance.clamp(0.0, 0.99);

        // SVF coefficient calculation
        self.g = (std::f32::consts::PI * cutoff / self.last_sample_rate as f32).tan();
        self.k = 2.0 - 2.0 * resonance;
        self.a1 = 1.0 / (1.0 + self.g * (self.g + self.k));
        self.a2 = self.g * self.a1;
        self.a3 = self.g * self.a2;
    }

    #[inline]
    fn process_sample(&mut self, input: f32) -> f32 {
        let v3 = input - self.ic2eq;
        let v1 = self.a1 * self.ic1eq + self.a2 * v3;
        let v2 = self.ic2eq + self.a2 * self.ic1eq + self.a3 * v3;

        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;

        match self.filter_type {
            FilterType::Lowpass => v2,
            FilterType::Highpass => input - self.k * v1 - v2,
            FilterType::Bandpass => v1,
            FilterType::Notch => input - self.k * v1,
        }
    }
}

impl Node for SvfFilter {
    fn prepare(&mut self, sample_rate: f64, _max_block: usize) {
        self.last_sample_rate = sample_rate;
        self.recalc_coeffs();
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            0 => {
                self.cutoff = value;
                if self.last_sample_rate > 0.0 {
                    self.recalc_coeffs();
                }
            }
            1 => {
                self.resonance = value;
                if self.last_sample_rate > 0.0 {
                    self.recalc_coeffs();
                }
            }
            _ => {}
        }
    }

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        self.update_coefficients(ctx.sample_rate);

        let input = match inputs.first() {
            Some(buf) => buf,
            None => {
                output.clear();
                return false;
            }
        };

        let in_ch = input.channel(0);
        let out_ch = output.channel_mut(0);

        for i in 0..ctx.frames {
            let sample = in_ch.get(i).copied().unwrap_or(0.0);
            out_ch[i] = self.process_sample(sample);
        }

        true
    }

    fn num_channels(&self) -> usize {
        1
    }

    fn reset(&mut self) {
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Convenience type aliases
// ═══════════════════════════════════════════════════════════════════════════

pub type LowpassFilter = SvfFilter;
pub type HighpassFilter = SvfFilter;
pub type BandpassFilter = SvfFilter;
pub type NotchFilter = SvfFilter;

