// Filter nodes using State Variable Filter (SVF) topology.
// This provides lowpass, highpass, and bandpass outputs.

use crate::audio_buffer::AudioBuffer;
use crate::node::{Node, ProcessContext};
use crate::nodes::params;

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
///
/// Supports:
/// - Base cutoff frequency (param)
/// - Key tracking: voice MIDI note offsets cutoff (0% = fixed, 100% = 1:1, 200% = 2x)
/// - Cutoff In: audio-rate modulation input interpreted as semitones, added to cutoff
pub struct SvfFilter {
    filter_type: FilterType,
    cutoff: f32,
    resonance: f32,
    key_tracking: f32,

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
            key_tracking: 0.0,
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
        self.recalc_coeffs_for_cutoff(self.cutoff);
    }

    fn recalc_coeffs_for_cutoff(&mut self, cutoff_hz: f32) {
        let cutoff = cutoff_hz.clamp(20.0, (self.last_sample_rate as f32 * 0.49).max(20.0));
        let resonance = self.resonance.clamp(0.0, 0.99);

        self.g = (std::f32::consts::PI * cutoff / self.last_sample_rate as f32).tan();
        self.k = 2.0 - 2.0 * resonance;
        self.a1 = 1.0 / (1.0 + self.g * (self.g + self.k));
        self.a2 = self.g * self.a1;
        self.a3 = self.g * self.a2;
    }

    /// Efficiently update only g-dependent coefficients for per-sample cutoff modulation.
    /// k is cached since resonance doesn't change per-sample.
    #[inline]
    fn set_effective_cutoff(&mut self, cutoff_hz: f32) {
        let cutoff = cutoff_hz.clamp(20.0, (self.last_sample_rate as f32 * 0.49).max(20.0));
        self.g = (std::f32::consts::PI * cutoff / self.last_sample_rate as f32).tan();
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

    /// Convert a MIDI note number to Hz.
    #[inline]
    fn midi_note_to_hz(note: f32) -> f32 {
        440.0 * 2.0f32.powf((note - 69.0) / 12.0)
    }

    /// Compute effective cutoff in Hz from base cutoff + key tracking + modulation in semitones.
    #[inline]
    fn effective_cutoff_hz(&self, base_cutoff: f32, key_semitones: f32, mod_semitones: f32) -> f32 {
        // Convert base cutoff to semitones relative to A4
        let base_semi = 69.0 + 12.0 * (base_cutoff / 440.0).log2();
        // Add key tracking offset and modulation
        let total_semi = base_semi + key_semitones + mod_semitones;
        Self::midi_note_to_hz(total_semi)
    }
}

impl Node for SvfFilter {
    fn prepare(&mut self, sample_rate: f64, _max_block: usize) {
        self.last_sample_rate = sample_rate;
        self.recalc_coeffs();
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::CUTOFF => {
                self.cutoff = value;
                if self.last_sample_rate > 0.0 {
                    self.recalc_coeffs();
                }
            }
            params::RESONANCE => {
                self.resonance = value;
                if self.last_sample_rate > 0.0 {
                    self.recalc_coeffs();
                }
            }
            params::KEY_TRACKING => {
                self.key_tracking = value;
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
                return true;
            }
        };

        // Cutoff modulation input (port 1) — values interpreted as semitones
        let cutoff_mod = inputs.get(1);

        // Key tracking: compute semitone offset from voice note
        // key_tracking: 0.0 = no tracking, 1.0 = 1:1, 2.0 = 2x
        // Reference note is C3 (MIDI 60) — offset is relative to middle C
        let key_semitones = if self.key_tracking != 0.0 {
            if let Some(voice) = &ctx.voice {
                (voice.note as f32 - 60.0) * self.key_tracking
            } else {
                0.0
            }
        } else {
            0.0
        };

        let in_ch = input.channel(0);
        let out_ch = output.channel_mut(0);

        let has_mod = cutoff_mod.is_some();

        if has_mod {
            // Per-sample coefficient update: cutoff-in modulation is audio-rate
            let mod_ch = cutoff_mod.unwrap().channel(0);
            for i in 0..ctx.frames {
                let mod_semi = mod_ch.get(i).copied().unwrap_or(0.0);
                let eff_cutoff = self.effective_cutoff_hz(self.cutoff, key_semitones, mod_semi);
                self.set_effective_cutoff(eff_cutoff);
                let sample = in_ch.get(i).copied().unwrap_or(0.0);
                out_ch[i] = self.process_sample(sample);
            }
        } else if key_semitones != 0.0 {
            // Key tracking only — constant per block, compute once
            let eff_cutoff = self.effective_cutoff_hz(self.cutoff, key_semitones, 0.0);
            self.set_effective_cutoff(eff_cutoff);
            for i in 0..ctx.frames {
                let sample = in_ch.get(i).copied().unwrap_or(0.0);
                out_ch[i] = self.process_sample(sample);
            }
        } else {
            // No modulation — use base cutoff (coefficients already set)
            for i in 0..ctx.frames {
                let sample = in_ch.get(i).copied().unwrap_or(0.0);
                out_ch[i] = self.process_sample(sample);
            }
        }

        false
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
