// Basic oscillator nodes.

use std::f32::consts::TAU;

use crate::audio_buffer::AudioBuffer;
use crate::node::{Node, ProcessContext};

use super::params;

const PHASE_START: f32 = std::f32::consts::PI / 2.0;

// ═══════════════════════════════════════════════════════════════════
// Sine Oscillator
// ═══════════════════════════════════════════════════════════════════

pub struct SineOsc {
    phase: f32,
    freq: f32,
    detune: f32,
    sample_rate: f32,
    was_silent: bool,
    last_note: Option<u8>,
}

impl SineOsc {
    pub fn new() -> Self {
        Self {
            phase: PHASE_START,
            freq: 440.0,
            detune: 0.0,
            sample_rate: 48_000.0,
            was_silent: true,
            last_note: None,
        }
    }

    #[inline]
    fn effective_freq(&self, voice_note: Option<u8>) -> f32 {
        let base = voice_note
            .map(|n| 440.0 * 2.0_f32.powf((n as f32 - 69.0) / 12.0))
            .unwrap_or(self.freq);
        base * 2.0_f32.powf(self.detune / 1200.0)
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
        _inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        let voice_note = ctx.voice.map(|v| v.note);
        let freq = self.effective_freq(voice_note);
        let inc = freq / self.sample_rate;

        // Check gate for per-voice operation
        if let Some(voice) = ctx.voice {
            if !voice.gate && !voice.release {
                self.was_silent = true;
                return true; // Silent if voice is fully released
            }
            if voice.trigger {
                let note_changed = self.last_note != Some(voice.note);
                // Reset phase if: previously silent, or voice was stolen for different note
                if self.was_silent || note_changed {
                    self.phase = PHASE_START;
                }
                self.last_note = Some(voice.note);
            }
            self.was_silent = false;
        }

        let buf = output.channel_mut(0);
        for sample in buf.iter_mut().take(ctx.frames) {
            *sample = (self.phase * TAU).sin();
            self.phase = (self.phase + inc).fract();
        }

        false
    }

    fn num_channels(&self) -> usize {
        1
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::FREQ => self.freq = value,
            params::DETUNE => self.detune = value,
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.phase = PHASE_START;
        self.was_silent = true;
        self.last_note = None;
    }
}

// ═══════════════════════════════════════════════════════════════════
// Saw Oscillator (naive, non-bandlimited)
// ═══════════════════════════════════════════════════════════════════

pub struct SawOsc {
    phase: f32,
    freq: f32,
    detune: f32,
    sample_rate: f32,
    was_silent: bool,
    last_note: Option<u8>,
}

impl SawOsc {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            freq: 440.0,
            detune: 0.0,
            sample_rate: 48_000.0,
            was_silent: true,
            last_note: None,
        }
    }

    #[inline]
    fn effective_freq(&self, voice_note: Option<u8>) -> f32 {
        let base = voice_note
            .map(|n| 440.0 * 2.0_f32.powf((n as f32 - 69.0) / 12.0))
            .unwrap_or(self.freq);
        base * 2.0_f32.powf(self.detune / 1200.0)
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
        _inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        let voice_note = ctx.voice.map(|v| v.note);
        let freq = self.effective_freq(voice_note);
        let inc = freq / self.sample_rate;

        if let Some(voice) = ctx.voice {
            if !voice.gate && !voice.release {
                self.was_silent = true;
                return true;
            }
            if voice.trigger {
                let note_changed = self.last_note != Some(voice.note);
                if self.was_silent || note_changed {
                    self.phase = PHASE_START;
                }
                self.last_note = Some(voice.note);
            }
            self.was_silent = false;
        }

        let buf = output.channel_mut(0);
        for sample in buf.iter_mut().take(ctx.frames) {
            *sample = 2.0 * self.phase - 1.0;
            self.phase = (self.phase + inc).fract();
        }

        false
    }

    fn num_channels(&self) -> usize {
        1
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::FREQ => self.freq = value,
            params::DETUNE => self.detune = value,
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.phase = PHASE_START;
        self.was_silent = true;
        self.last_note = None;
    }
}

// ═══════════════════════════════════════════════════════════════════
// Square Oscillator (with pulse width)
// ═══════════════════════════════════════════════════════════════════

pub struct SquareOsc {
    phase: f32,
    freq: f32,
    pulse_width: f32,
    sample_rate: f32,
    was_silent: bool,
    last_note: Option<u8>,
}

impl SquareOsc {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            freq: 440.0,
            pulse_width: 0.5,
            sample_rate: 48_000.0,
            was_silent: true,
            last_note: None,
        }
    }

    #[inline]
    fn effective_freq(&self, voice_note: Option<u8>) -> f32 {
        voice_note
            .map(|n| 440.0 * 2.0_f32.powf((n as f32 - 69.0) / 12.0))
            .unwrap_or(self.freq)
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
        _inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        let voice_note = ctx.voice.map(|v| v.note);
        let freq = self.effective_freq(voice_note);
        let inc = freq / self.sample_rate;

        if let Some(voice) = ctx.voice {
            if !voice.gate && !voice.release {
                self.was_silent = true;
                return true;
            }
            if voice.trigger {
                let note_changed = self.last_note != Some(voice.note);
                if self.was_silent || note_changed {
                    self.phase = PHASE_START;
                }
                self.last_note = Some(voice.note);
            }
            self.was_silent = false;
        }

        let buf = output.channel_mut(0);
        for sample in buf.iter_mut().take(ctx.frames) {
            *sample = if self.phase < self.pulse_width {
                1.0
            } else {
                -1.0
            };
            self.phase = (self.phase + inc).fract();
        }

        false
    }

    fn num_channels(&self) -> usize {
        1
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::FREQ => self.freq = value,
            params::PULSE_WIDTH => self.pulse_width = value.clamp(0.01, 0.99),
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.phase = PHASE_START;
        self.was_silent = true;
        self.last_note = None;
    }
}

// ═══════════════════════════════════════════════════════════════════
// Triangle Oscillator
// ═══════════════════════════════════════════════════════════════════

pub struct TriangleOsc {
    phase: f32,
    freq: f32,
    sample_rate: f32,
    was_silent: bool,
    last_note: Option<u8>,
}

impl TriangleOsc {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            freq: 440.0,
            sample_rate: 48_000.0,
            was_silent: true,
            last_note: None,
        }
    }

    #[inline]
    fn effective_freq(&self, voice_note: Option<u8>) -> f32 {
        voice_note
            .map(|n| 440.0 * 2.0_f32.powf((n as f32 - 69.0) / 12.0))
            .unwrap_or(self.freq)
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
        _inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        let voice_note = ctx.voice.map(|v| v.note);
        let freq = self.effective_freq(voice_note);
        let inc = freq / self.sample_rate;

        if let Some(voice) = ctx.voice {
            if !voice.gate && !voice.release {
                self.was_silent = true;
                return true;
            }
            if voice.trigger {
                let note_changed = self.last_note != Some(voice.note);
                if self.was_silent || note_changed {
                    self.phase = PHASE_START;
                }
                self.last_note = Some(voice.note);
            }
            self.was_silent = false;
        }

        let buf = output.channel_mut(0);
        for sample in buf.iter_mut().take(ctx.frames) {
            *sample = if self.phase < 0.5 {
                4.0 * self.phase - 1.0
            } else {
                3.0 - 4.0 * self.phase
            };
            self.phase = (self.phase + inc).fract();
        }

        false
    }

    fn num_channels(&self) -> usize {
        1
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::FREQ => self.freq = value,
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.phase = PHASE_START;
        self.was_silent = true;
        self.last_note = None;
    }
}
