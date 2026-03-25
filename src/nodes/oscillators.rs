// Oscillator nodes.

use std::f32::consts::{PI, TAU};

use crate::audio_buffer::AudioBuffer;
use crate::node::{Node, ProcessContext};

use super::params;

const PHASE_START: f32 = std::f32::consts::PI / 2.0;

/// 2-point polyBLEP residual for anti-aliasing waveform discontinuities.
/// `t` is the phase (0..1), `dt` is the phase increment per sample.
#[inline]
fn poly_blep(t: f32, dt: f32) -> f32 {
    if dt <= 0.0 {
        return 0.0;
    }
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

// ═══════════════════════════════════════════════════════════════════
// Sine Oscillator
// ═══════════════════════════════════════════════════════════════════

pub struct SineOsc {
    phase_l: f32,
    phase_r: f32,
    freq: f32,
    detune: f32,
    pitch_offset: f32,
    key_tracking: bool,
    stereo_detune: bool,
    sample_rate: f32,
    last_note: Option<u8>,
    retrigger_last: f32,
}

impl SineOsc {
    pub fn new() -> Self {
        Self {
            phase_l: PHASE_START,
            phase_r: PHASE_START,
            freq: 440.0,
            detune: 0.0,
            pitch_offset: 0.0,
            key_tracking: true,
            stereo_detune: false,
            sample_rate: 48_000.0,
            last_note: None,
            retrigger_last: 0.0,
        }
    }

    #[inline]
    fn base_freq(&self, voice_note: Option<u8>) -> f32 {
        let base = if self.key_tracking {
            voice_note
                .map(|n| 440.0 * 2.0_f32.powf((n as f32 - 69.0) / 12.0))
                .unwrap_or(self.freq)
        } else {
            self.freq
        };
        base * 2.0_f32.powf(self.pitch_offset / 12.0)
    }
}

impl Default for SineOsc {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for SineOsc {
    fn prepare(&mut self, sample_rate: f64, _max_block: usize) {
        self.sample_rate = sample_rate as f32;
    }

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        let voice_note = ctx.voice.map(|v| v.note);
        let base = self.base_freq(voice_note);
        let detune_factor = 2.0f32.powf(self.detune / 1200.0);

        if let Some(voice) = ctx.voice {
            if voice.trigger {
                let note_changed = self.last_note != Some(voice.note);
                if note_changed {
                    self.phase_l = PHASE_START;
                    self.phase_r = PHASE_START;
                }
                self.last_note = Some(voice.note);
            }
        }

        let pitch_in = inputs.first();
        let phase_in = inputs.get(1);
        let retrigger_in = inputs.get(2);
        let frames = ctx.frames;

        // Left channel
        {
            let buf = output.channel_mut(0);
            for i in 0..frames {
                // Retrigger on rising edge
                if let Some(rt) = retrigger_in {
                    let s = rt.channel(0).get(i).copied().unwrap_or(0.0);
                    if s > 0.5 && self.retrigger_last <= 0.5 {
                        self.phase_l = PHASE_START;
                        self.phase_r = PHASE_START;
                    }
                    self.retrigger_last = s;
                }

                let pitch_mod = pitch_in
                    .and_then(|b| b.channel(0).get(i).copied())
                    .unwrap_or(0.0);
                let phase_offset = phase_in
                    .and_then(|b| b.channel(0).get(i).copied())
                    .unwrap_or(0.0);
                let freq = base * detune_factor * 2.0f32.powf(pitch_mod / 12.0);
                let inc = freq / self.sample_rate;
                let ep = (self.phase_l + phase_offset).fract().abs();
                buf[i] = (ep * TAU).sin();
                self.phase_l = (self.phase_l + inc).fract();
            }
        }

        // Right channel
        {
            let left_samples: Vec<f32> = output.channel(0)[..frames].to_vec();
            let buf_r = output.channel_mut(1);
            if self.stereo_detune {
                for i in 0..frames {
                    let pitch_mod = pitch_in
                        .and_then(|b| b.channel(0).get(i).copied())
                        .unwrap_or(0.0);
                    let phase_offset = phase_in
                        .and_then(|b| b.channel(0).get(i).copied())
                        .unwrap_or(0.0);
                    let freq = base / detune_factor * 2.0f32.powf(pitch_mod / 12.0);
                    let inc = freq / self.sample_rate;
                    let ep = (self.phase_r + phase_offset).fract().abs();
                    buf_r[i] = (ep * TAU).sin();
                    self.phase_r = (self.phase_r + inc).fract();
                }
            } else {
                buf_r[..frames].copy_from_slice(&left_samples);
            }
        }

        false
    }

    fn num_channels(&self) -> usize {
        2
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::FREQ => self.freq = value,
            params::DETUNE => self.detune = value,
            params::OSC_KEY_TRACKING => self.key_tracking = value > 0.5,
            params::PITCH_OFFSET => self.pitch_offset = value.clamp(-48.0, 48.0),
            params::STEREO_DETUNE => self.stereo_detune = value > 0.5,
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.phase_l = PHASE_START;
        self.phase_r = PHASE_START;
        self.last_note = None;
        self.retrigger_last = 0.0;
    }
}

// ═══════════════════════════════════════════════════════════════════
// Saw Oscillator (polyBLEP band-limited)
// ═══════════════════════════════════════════════════════════════════

pub struct SawOsc {
    phase_l: f32,
    phase_r: f32,
    freq: f32,
    detune: f32,
    pitch_offset: f32,
    key_tracking: bool,
    stereo_detune: bool,
    sample_rate: f32,
    last_note: Option<u8>,
    retrigger_last: f32,
}

impl SawOsc {
    pub fn new() -> Self {
        Self {
            phase_l: 0.0,
            phase_r: 0.0,
            freq: 440.0,
            detune: 0.0,
            pitch_offset: 0.0,
            key_tracking: true,
            stereo_detune: false,
            sample_rate: 48_000.0,
            last_note: None,
            retrigger_last: 0.0,
        }
    }

    #[inline]
    fn base_freq(&self, voice_note: Option<u8>) -> f32 {
        let base = if self.key_tracking {
            voice_note
                .map(|n| 440.0 * 2.0_f32.powf((n as f32 - 69.0) / 12.0))
                .unwrap_or(self.freq)
        } else {
            self.freq
        };
        base * 2.0_f32.powf(self.pitch_offset / 12.0)
    }
}

impl Default for SawOsc {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for SawOsc {
    fn prepare(&mut self, sample_rate: f64, _max_block: usize) {
        self.sample_rate = sample_rate as f32;
    }

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        let voice_note = ctx.voice.map(|v| v.note);
        let base = self.base_freq(voice_note);
        let detune_factor = 2.0f32.powf(self.detune / 1200.0);

        if let Some(voice) = ctx.voice {
            if voice.trigger {
                let note_changed = self.last_note != Some(voice.note);
                if note_changed {
                    self.phase_l = PHASE_START;
                    self.phase_r = PHASE_START;
                }
                self.last_note = Some(voice.note);
            }
        }

        let pitch_in = inputs.first();
        let phase_in = inputs.get(1);
        let retrigger_in = inputs.get(2);
        let frames = ctx.frames;

        // Left channel
        {
            let buf = output.channel_mut(0);
            for i in 0..frames {
                // Retrigger on rising edge
                if let Some(rt) = retrigger_in {
                    let s = rt.channel(0).get(i).copied().unwrap_or(0.0);
                    if s > 0.5 && self.retrigger_last <= 0.5 {
                        self.phase_l = PHASE_START;
                        self.phase_r = PHASE_START;
                    }
                    self.retrigger_last = s;
                }

                let pitch_mod = pitch_in
                    .and_then(|b| b.channel(0).get(i).copied())
                    .unwrap_or(0.0);
                let phase_offset = phase_in
                    .and_then(|b| b.channel(0).get(i).copied())
                    .unwrap_or(0.0);
                let freq = base * detune_factor * 2.0f32.powf(pitch_mod / 12.0);
                let inc = freq / self.sample_rate;
                let ep = (self.phase_l + phase_offset).fract().abs();
                buf[i] = 2.0 * ep - 1.0 - poly_blep(ep, inc);
                self.phase_l = (self.phase_l + inc).fract();
            }
        }

        // Right channel
        {
            let left_samples: Vec<f32> = output.channel(0)[..frames].to_vec();
            let buf_r = output.channel_mut(1);
            if self.stereo_detune {
                for i in 0..frames {
                    let pitch_mod = pitch_in
                        .and_then(|b| b.channel(0).get(i).copied())
                        .unwrap_or(0.0);
                    let phase_offset = phase_in
                        .and_then(|b| b.channel(0).get(i).copied())
                        .unwrap_or(0.0);
                    let freq = base / detune_factor * 2.0f32.powf(pitch_mod / 12.0);
                    let inc = freq / self.sample_rate;
                    let ep = (self.phase_r + phase_offset).fract().abs();
                    buf_r[i] = 2.0 * ep - 1.0 - poly_blep(ep, inc);
                    self.phase_r = (self.phase_r + inc).fract();
                }
            } else {
                buf_r[..frames].copy_from_slice(&left_samples);
            }
        }

        false
    }

    fn num_channels(&self) -> usize {
        2
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::FREQ => self.freq = value,
            params::DETUNE => self.detune = value,
            params::OSC_KEY_TRACKING => self.key_tracking = value > 0.5,
            params::PITCH_OFFSET => self.pitch_offset = value.clamp(-48.0, 48.0),
            params::STEREO_DETUNE => self.stereo_detune = value > 0.5,
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.phase_l = PHASE_START;
        self.phase_r = PHASE_START;
        self.last_note = None;
        self.retrigger_last = 0.0;
    }
}

// ═══════════════════════════════════════════════════════════════════
// Square Oscillator (polyBLEP band-limited, with pulse width)
// ═══════════════════════════════════════════════════════════════════

pub struct SquareOsc {
    phase_l: f32,
    phase_r: f32,
    freq: f32,
    detune: f32,
    pitch_offset: f32,
    pulse_width: f32,
    key_tracking: bool,
    stereo_detune: bool,
    sample_rate: f32,
    last_note: Option<u8>,
    retrigger_last: f32,
}

impl SquareOsc {
    pub fn new() -> Self {
        Self {
            phase_l: 0.0,
            phase_r: 0.0,
            freq: 440.0,
            detune: 0.0,
            pitch_offset: 0.0,
            pulse_width: 0.5,
            key_tracking: true,
            stereo_detune: false,
            sample_rate: 48_000.0,
            last_note: None,
            retrigger_last: 0.0,
        }
    }

    #[inline]
    fn base_freq(&self, voice_note: Option<u8>) -> f32 {
        let base = if self.key_tracking {
            voice_note
                .map(|n| 440.0 * 2.0_f32.powf((n as f32 - 69.0) / 12.0))
                .unwrap_or(self.freq)
        } else {
            self.freq
        };
        base * 2.0_f32.powf(self.pitch_offset / 12.0)
    }
}

impl Default for SquareOsc {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for SquareOsc {
    fn prepare(&mut self, sample_rate: f64, _max_block: usize) {
        self.sample_rate = sample_rate as f32;
    }

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        let voice_note = ctx.voice.map(|v| v.note);
        let base = self.base_freq(voice_note);
        let detune_factor = 2.0f32.powf(self.detune / 1200.0);

        if let Some(voice) = ctx.voice {
            if voice.trigger {
                let note_changed = self.last_note != Some(voice.note);
                if note_changed {
                    self.phase_l = PHASE_START;
                    self.phase_r = PHASE_START;
                }
                self.last_note = Some(voice.note);
            }
        }

        let pitch_in = inputs.first();
        let phase_in = inputs.get(1);
        let retrigger_in = inputs.get(2);
        let frames = ctx.frames;
        let pw = self.pulse_width;

        // Left channel
        {
            let buf = output.channel_mut(0);
            for i in 0..frames {
                // Retrigger on rising edge
                if let Some(rt) = retrigger_in {
                    let s = rt.channel(0).get(i).copied().unwrap_or(0.0);
                    if s > 0.5 && self.retrigger_last <= 0.5 {
                        self.phase_l = PHASE_START;
                        self.phase_r = PHASE_START;
                    }
                    self.retrigger_last = s;
                }

                let pitch_mod = pitch_in
                    .and_then(|b| b.channel(0).get(i).copied())
                    .unwrap_or(0.0);
                let phase_offset = phase_in
                    .and_then(|b| b.channel(0).get(i).copied())
                    .unwrap_or(0.0);
                let freq = base * detune_factor * 2.0f32.powf(pitch_mod / 12.0);
                let inc = freq / self.sample_rate;
                let ep = (self.phase_l + phase_offset).fract().abs();
                let mut s = if ep < pw { 1.0 } else { -1.0 };
                s += poly_blep(ep, inc);
                s -= poly_blep((ep - pw + 1.0) % 1.0, inc);
                buf[i] = s;
                self.phase_l = (self.phase_l + inc).fract();
            }
        }

        // Right channel
        {
            let left_samples: Vec<f32> = output.channel(0)[..frames].to_vec();
            let buf_r = output.channel_mut(1);
            if self.stereo_detune {
                for i in 0..frames {
                    let pitch_mod = pitch_in
                        .and_then(|b| b.channel(0).get(i).copied())
                        .unwrap_or(0.0);
                    let phase_offset = phase_in
                        .and_then(|b| b.channel(0).get(i).copied())
                        .unwrap_or(0.0);
                    let freq = base / detune_factor * 2.0f32.powf(pitch_mod / 12.0);
                    let inc = freq / self.sample_rate;
                    let ep = (self.phase_r + phase_offset).fract().abs();
                    let mut s = if ep < pw { 1.0 } else { -1.0 };
                    s += poly_blep(ep, inc);
                    s -= poly_blep((ep - pw + 1.0) % 1.0, inc);
                    buf_r[i] = s;
                    self.phase_r = (self.phase_r + inc).fract();
                }
            } else {
                buf_r[..frames].copy_from_slice(&left_samples);
            }
        }

        false
    }

    fn num_channels(&self) -> usize {
        2
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::FREQ => self.freq = value,
            params::DETUNE => self.detune = value,
            params::PULSE_WIDTH => self.pulse_width = value.clamp(0.01, 0.99),
            params::OSC_KEY_TRACKING => self.key_tracking = value > 0.5,
            params::PITCH_OFFSET => self.pitch_offset = value.clamp(-48.0, 48.0),
            params::STEREO_DETUNE => self.stereo_detune = value > 0.5,
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.phase_l = PHASE_START;
        self.phase_r = PHASE_START;
        self.last_note = None;
        self.retrigger_last = 0.0;
    }
}

// ═══════════════════════════════════════════════════════════════════
// Triangle Oscillator
// ═══════════════════════════════════════════════════════════════════

pub struct TriangleOsc {
    phase_l: f32,
    phase_r: f32,
    freq: f32,
    detune: f32,
    pitch_offset: f32,
    key_tracking: bool,
    stereo_detune: bool,
    sample_rate: f32,
    last_note: Option<u8>,
    retrigger_last: f32,
}

impl TriangleOsc {
    pub fn new() -> Self {
        Self {
            phase_l: 0.0,
            phase_r: 0.0,
            freq: 440.0,
            detune: 0.0,
            pitch_offset: 0.0,
            key_tracking: true,
            stereo_detune: false,
            sample_rate: 48_000.0,
            last_note: None,
            retrigger_last: 0.0,
        }
    }

    #[inline]
    fn base_freq(&self, voice_note: Option<u8>) -> f32 {
        let base = if self.key_tracking {
            voice_note
                .map(|n| 440.0 * 2.0_f32.powf((n as f32 - 69.0) / 12.0))
                .unwrap_or(self.freq)
        } else {
            self.freq
        };
        base * 2.0_f32.powf(self.pitch_offset / 12.0)
    }
}

impl Default for TriangleOsc {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for TriangleOsc {
    fn prepare(&mut self, sample_rate: f64, _max_block: usize) {
        self.sample_rate = sample_rate as f32;
    }

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        let voice_note = ctx.voice.map(|v| v.note);
        let base = self.base_freq(voice_note);
        let detune_factor = 2.0f32.powf(self.detune / 1200.0);

        if let Some(voice) = ctx.voice {
            if voice.trigger {
                let note_changed = self.last_note != Some(voice.note);
                if note_changed {
                    self.phase_l = PHASE_START;
                    self.phase_r = PHASE_START;
                }
                self.last_note = Some(voice.note);
            }
        }

        let pitch_in = inputs.first();
        let phase_in = inputs.get(1);
        let retrigger_in = inputs.get(2);
        let frames = ctx.frames;

        #[inline]
        fn tri_sample(phase: f32) -> f32 {
            if phase < 0.5 {
                4.0 * phase - 1.0
            } else {
                3.0 - 4.0 * phase
            }
        }

        // Left channel
        {
            let buf = output.channel_mut(0);
            for i in 0..frames {
                // Retrigger on rising edge
                if let Some(rt) = retrigger_in {
                    let s = rt.channel(0).get(i).copied().unwrap_or(0.0);
                    if s > 0.5 && self.retrigger_last <= 0.5 {
                        self.phase_l = PHASE_START;
                        self.phase_r = PHASE_START;
                    }
                    self.retrigger_last = s;
                }

                let pitch_mod = pitch_in
                    .and_then(|b| b.channel(0).get(i).copied())
                    .unwrap_or(0.0);
                let phase_offset = phase_in
                    .and_then(|b| b.channel(0).get(i).copied())
                    .unwrap_or(0.0);
                let freq = base * detune_factor * 2.0f32.powf(pitch_mod / 12.0);
                let inc = freq / self.sample_rate;
                let ep = (self.phase_l + phase_offset).fract().abs();
                buf[i] = tri_sample(ep);
                self.phase_l = (self.phase_l + inc).fract();
            }
        }

        // Right channel
        {
            let left_samples: Vec<f32> = output.channel(0)[..frames].to_vec();
            let buf_r = output.channel_mut(1);
            if self.stereo_detune {
                for i in 0..frames {
                    let pitch_mod = pitch_in
                        .and_then(|b| b.channel(0).get(i).copied())
                        .unwrap_or(0.0);
                    let phase_offset = phase_in
                        .and_then(|b| b.channel(0).get(i).copied())
                        .unwrap_or(0.0);
                    let freq = base / detune_factor * 2.0f32.powf(pitch_mod / 12.0);
                    let inc = freq / self.sample_rate;
                    let ep = (self.phase_r + phase_offset).fract().abs();
                    buf_r[i] = tri_sample(ep);
                    self.phase_r = (self.phase_r + inc).fract();
                }
            } else {
                buf_r[..frames].copy_from_slice(&left_samples);
            }
        }

        false
    }

    fn num_channels(&self) -> usize {
        2
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::FREQ => self.freq = value,
            params::DETUNE => self.detune = value,
            params::OSC_KEY_TRACKING => self.key_tracking = value > 0.5,
            params::PITCH_OFFSET => self.pitch_offset = value.clamp(-48.0, 48.0),
            params::STEREO_DETUNE => self.stereo_detune = value > 0.5,
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.phase_l = PHASE_START;
        self.phase_r = PHASE_START;
        self.last_note = None;
        self.retrigger_last = 0.0;
    }
}

// ═══════════════════════════════════════════════════════════════════
// Phase Distortion Oscillator
// ═══════════════════════════════════════════════════════════════════

/// Phase distortion oscillator with multiple algorithms, feedback,
/// formant control, pitch ratio, and stereo detune.
///
/// Inputs:
///   Port 0 - Pitch In: audio-rate pitch modulation in semitones
///   Port 1 - Phase In: audio-rate phase offset (0.0 to 1.0)
///   Port 2 - Retrigger In: rising edge (>0.5) resets phase
///
/// Output:
///   Stereo (2 channels). With stereo detune off, both channels are identical.
///   With stereo detune on, right channel uses inverse detune.
pub struct PhaseOsc {
    phase_l: f32,
    phase_r: f32,
    feedback_sample: f32,
    sample_rate: f32,
    last_note: Option<u8>,
    retrigger_last: f32,

    // Parameters
    shape: f32,
    algorithm: u32,
    feedback: f32,
    formant: u32,
    key_tracking: bool,
    pitch_ratio_num: f32,
    pitch_ratio_den: f32,
    pitch_offset: f32,
    stereo_detune: bool,
    detune: f32,
}

impl PhaseOsc {
    pub fn new() -> Self {
        Self {
            phase_l: 0.0,
            phase_r: 0.0,
            feedback_sample: 0.0,
            sample_rate: 48_000.0,
            last_note: None,
            retrigger_last: 0.0,
            shape: 0.0,
            algorithm: 0,
            feedback: 0.0,
            formant: 1,
            key_tracking: true,
            pitch_ratio_num: 1.0,
            pitch_ratio_den: 1.0,
            pitch_offset: 0.0,
            stereo_detune: false,
            detune: 0.0,
        }
    }

    /// Compute base frequency from voice note, pitch ratio, offset, and optional
    /// per-sample pitch modulation in semitones.
    #[inline]
    fn compute_freq(&self, voice_note: Option<u8>, pitch_mod_st: f32) -> f32 {
        let base_note = if self.key_tracking {
            voice_note.map(|n| n as f32).unwrap_or(69.0)
        } else {
            69.0 // A4
        };

        let total_semi = base_note + self.pitch_offset + pitch_mod_st;
        let base_hz = 440.0 * 2.0f32.powf((total_semi - 69.0) / 12.0);

        // Apply pitch ratio
        let ratio = if self.pitch_ratio_den > 0.0 {
            self.pitch_ratio_num / self.pitch_ratio_den
        } else {
            0.0
        };

        base_hz * ratio
    }

    /// CZ-style phase distortion.
    /// Compresses the first part of the cycle, stretching the second part.
    /// amount=0: linear (no distortion), amount→1: maximum brightness.
    #[inline]
    fn phase_distort(phase: f32, amount: f32) -> f32 {
        if amount < 0.001 {
            return phase;
        }
        let d = amount.min(0.99);
        let bp = 0.5 - 0.45 * d;
        if phase < bp {
            0.5 * phase / bp
        } else {
            0.5 + 0.5 * (phase - bp) / (1.0 - bp)
        }
    }

    /// Generate one sample from the selected algorithm.
    #[inline]
    fn algorithm_sample(phase: f32, algorithm: u32) -> f32 {
        match algorithm {
            0 => (phase * TAU).sin(),
            1 => 2.0 * phase - 1.0,
            2 => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            3 => {
                if phase < 0.5 {
                    4.0 * phase - 1.0
                } else {
                    3.0 - 4.0 * phase
                }
            }
            4 => {
                if phase < 0.5 {
                    (phase * TAU).sin()
                } else {
                    0.0
                }
            }
            5 => (phase * PI).sin().abs(),
            _ => (phase * TAU).sin(),
        }
    }

    /// Generate one sample with phase distortion, formant, and feedback.
    #[inline]
    fn generate(&mut self, phase: f32) -> f32 {
        // Apply feedback to phase (phase modulation)
        let fb_phase = (phase + self.feedback_sample * self.feedback).fract().abs();

        // Apply phase distortion (shape parameter)
        let pd_phase = Self::phase_distort(fb_phase, self.shape);

        // Apply formant (multiply phase, wrap to create extra cycles within one period)
        let formant_phase = (pd_phase * self.formant as f32).fract();

        let sample = Self::algorithm_sample(formant_phase, self.algorithm);
        self.feedback_sample = sample;
        sample
    }
}

impl Default for PhaseOsc {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for PhaseOsc {
    fn prepare(&mut self, sample_rate: f64, _max_block: usize) {
        self.sample_rate = sample_rate as f32;
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::SHAPE => self.shape = value.clamp(0.0, 1.0),
            params::ALGORITHM => self.algorithm = (value as u32).min(5),
            params::PO_FEEDBACK => self.feedback = value.clamp(0.0, 1.0),
            params::FORMANT => self.formant = (value as u32).clamp(1, 9),
            params::PO_KEY_TRACKING => self.key_tracking = value > 0.5,
            params::PITCH_RATIO_NUM => self.pitch_ratio_num = value.clamp(0.0, 99.0),
            params::PITCH_RATIO_DEN => self.pitch_ratio_den = value.clamp(1.0, 99.0),
            params::PITCH_OFFSET => self.pitch_offset = value.clamp(-48.0, 48.0),
            params::STEREO_DETUNE => self.stereo_detune = value > 0.5,
            params::PO_DETUNE => self.detune = value.clamp(-27.0, 27.0),
            _ => {}
        }
    }

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        let voice_note = ctx.voice.map(|v| v.note);

        // Handle retrigger from voice
        if let Some(voice) = ctx.voice {
            if voice.trigger {
                let note_changed = self.last_note != Some(voice.note);
                if note_changed {
                    self.phase_l = 0.0;
                    self.phase_r = 0.0;
                    self.feedback_sample = 0.0;
                }
                self.last_note = Some(voice.note);
            }
        }

        // Input buffers (ordered by dest_port from compile.rs)
        let pitch_in = inputs.first(); // Port 0: semitones
        let phase_in = inputs.get(1); // Port 1: phase offset (0-1)
        let retrigger_in = inputs.get(2); // Port 2: retrigger

        // Process left channel and store per-sample data for right channel
        // (avoids double mutable borrow of output)
        let frames = ctx.frames;

        // Left channel pass: generate samples, advance phase_l, update feedback
        {
            let out_l = output.channel_mut(0);
            for i in 0..frames {
                // Retrigger on rising edge
                if let Some(rt) = retrigger_in {
                    let s = rt.channel(0).get(i).copied().unwrap_or(0.0);
                    if s > 0.5 && self.retrigger_last <= 0.5 {
                        self.phase_l = 0.0;
                        self.phase_r = 0.0;
                        self.feedback_sample = 0.0;
                    }
                    self.retrigger_last = s;
                }

                let pitch_mod = pitch_in
                    .and_then(|buf| buf.channel(0).get(i).copied())
                    .unwrap_or(0.0);
                let phase_offset = phase_in
                    .and_then(|buf| buf.channel(0).get(i).copied())
                    .unwrap_or(0.0);

                let base_freq = self.compute_freq(voice_note, pitch_mod);
                let freq_l = (base_freq + self.detune).max(0.0);
                let inc_l = freq_l / self.sample_rate;

                let effective_phase = (self.phase_l + phase_offset).fract().abs();
                let sample = self.generate(effective_phase);
                out_l[i] = sample;

                self.phase_l = (self.phase_l + inc_l).fract();
            }
        }

        // Right channel pass
        {
            let out_l_ref = output.channel(0);
            // Copy left samples for the non-stereo-detune path
            let left_samples: Vec<f32> = out_l_ref[..frames].to_vec();

            let out_r = output.channel_mut(1);
            if self.stereo_detune {
                for i in 0..frames {
                    let pitch_mod = pitch_in
                        .and_then(|buf| buf.channel(0).get(i).copied())
                        .unwrap_or(0.0);
                    let phase_offset = phase_in
                        .and_then(|buf| buf.channel(0).get(i).copied())
                        .unwrap_or(0.0);

                    let base_freq = self.compute_freq(voice_note, pitch_mod);
                    let freq_r = (base_freq - self.detune).max(0.0);
                    let inc_r = freq_r / self.sample_rate;

                    let effective_phase = (self.phase_r + phase_offset).fract().abs();
                    // Generate directly (don't update feedback_sample again)
                    let fb_phase = (effective_phase + self.feedback_sample * self.feedback)
                        .fract()
                        .abs();
                    let pd_phase = Self::phase_distort(fb_phase, self.shape);
                    let formant_phase = (pd_phase * self.formant as f32).fract();
                    out_r[i] = Self::algorithm_sample(formant_phase, self.algorithm);

                    self.phase_r = (self.phase_r + inc_r).fract();
                }
            } else {
                out_r[..frames].copy_from_slice(&left_samples);
            }
        }

        false
    }

    fn num_channels(&self) -> usize {
        2
    }

    fn reset(&mut self) {
        self.phase_l = 0.0;
        self.phase_r = 0.0;
        self.feedback_sample = 0.0;
        self.retrigger_last = 0.0;
        self.last_note = None;
    }
}
