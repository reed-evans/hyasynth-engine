// Envelope generators.

use crate::audio_buffer::AudioBuffer;
use crate::node::{Node, ProcessContext};

use super::params;

// ═══════════════════════════════════════════════════════════════════
// ADSR Envelope
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq)]
enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Evaluate a quadratic bezier for envelope shaping.
///
/// Given a linear phase `t` (0.0–1.0), start level, end level, and a curve
/// parameter (0.0–1.0), returns the shaped level.
///
/// The curve parameter controls the bezier control point's y-position:
/// - 0.5 = linear (control point at midpoint)
/// - 0.0 = concave (fast start, slow end)
/// - 1.0 = convex (slow start, fast end)
#[inline]
fn bezier_shape(t: f32, start: f32, end: f32, curve: f32) -> f32 {
    // Control point y-value: lerp between start and end based on curve
    let cp = start + (end - start) * curve;
    // Quadratic bezier: B(t) = (1-t)²·P0 + 2(1-t)t·P1 + t²·P2
    let inv = 1.0 - t;
    inv * inv * start + 2.0 * inv * t * cp + t * t * end
}

pub struct AdsrEnvelope {
    stage: EnvelopeStage,
    level: f32,
    smooth_level: f32,

    // Parameters (in seconds)
    attack: f32,
    decay: f32,
    sustain: f32, // 0-1 level
    release: f32,

    // Curve parameters (0.0-1.0, 0.5 = linear)
    attack_curve: f32,
    decay_curve: f32,
    release_curve: f32,

    // Phase tracking for bezier evaluation
    stage_phase: f32,   // 0.0→1.0 progress through current stage
    stage_start: f32,   // level at stage entry
    stage_end: f32,     // target level for current stage

    sample_rate: f32,
    release_level: f32,
    last_note: Option<u8>,
}

impl AdsrEnvelope {
    pub fn new() -> Self {
        Self {
            stage: EnvelopeStage::Idle,
            level: 0.0,
            smooth_level: 0.0,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.3,
            attack_curve: 0.5,
            decay_curve: 0.5,
            release_curve: 0.5,
            stage_phase: 0.0,
            stage_start: 0.0,
            stage_end: 0.0,
            sample_rate: 48_000.0,
            release_level: 0.0,
            last_note: None,
        }
    }

    /// Enter a new stage, capturing the start/end levels and resetting phase.
    #[inline]
    fn enter_stage(&mut self, stage: EnvelopeStage) {
        self.stage = stage;
        self.stage_phase = 0.0;
        self.stage_start = self.level;
        match stage {
            EnvelopeStage::Attack => self.stage_end = 1.0,
            EnvelopeStage::Decay => self.stage_end = self.sustain,
            EnvelopeStage::Release => {
                self.release_level = self.level;
                self.stage_end = 0.0;
            }
            _ => {}
        }
    }

    #[inline]
    fn process_sample(&mut self) -> f32 {
        match self.stage {
            EnvelopeStage::Idle => 0.0,

            EnvelopeStage::Attack => {
                let phase_inc = 1.0 / (self.attack * self.sample_rate).max(1.0);
                self.stage_phase += phase_inc;
                if self.stage_phase >= 1.0 {
                    self.stage_phase = 1.0;
                    self.level = self.stage_end;
                    self.enter_stage(EnvelopeStage::Decay);
                } else {
                    self.level = bezier_shape(
                        self.stage_phase,
                        self.stage_start,
                        self.stage_end,
                        self.attack_curve,
                    );
                }
                self.level
            }

            EnvelopeStage::Decay => {
                let phase_inc = 1.0 / (self.decay * self.sample_rate).max(1.0);
                self.stage_phase += phase_inc;
                if self.stage_phase >= 1.0 {
                    self.stage_phase = 1.0;
                    self.level = self.stage_end;
                    self.stage = EnvelopeStage::Sustain;
                } else {
                    self.level = bezier_shape(
                        self.stage_phase,
                        self.stage_start,
                        self.stage_end,
                        self.decay_curve,
                    );
                }
                self.level
            }

            EnvelopeStage::Sustain => self.sustain,

            EnvelopeStage::Release => {
                let phase_inc = 1.0 / (self.release * self.sample_rate).max(1.0);
                self.stage_phase += phase_inc;
                if self.stage_phase >= 1.0 {
                    self.stage_phase = 1.0;
                    self.level = 0.0;
                    self.stage = EnvelopeStage::Idle;
                } else {
                    self.level = bezier_shape(
                        self.stage_phase,
                        self.stage_start,
                        self.stage_end,
                        self.release_curve,
                    );
                }
                self.level
            }
        }
    }
}

impl Default for AdsrEnvelope {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for AdsrEnvelope {
    fn prepare(&mut self, sample_rate: f64, _max_block: usize) {
        self.sample_rate = sample_rate as f32;
    }

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        // Handle voice triggers
        if let Some(voice) = ctx.voice {
            if voice.trigger {
                // Check if this is a different note (voice stealing) or same note retriggering
                let note_changed = self.last_note != Some(voice.note);

                // Reset to 0 if: idle, or voice was stolen for a different note
                if self.stage == EnvelopeStage::Idle || note_changed {
                    self.level = 0.0;
                    self.smooth_level = 0.0;
                }
                self.enter_stage(EnvelopeStage::Attack);
                self.last_note = Some(voice.note);
            }
            if voice.release
                && self.stage != EnvelopeStage::Idle
                && self.stage != EnvelopeStage::Release
            {
                self.enter_stage(EnvelopeStage::Release);
            }
        }

        let has_input = !inputs.is_empty();
        let buf = output.channel_mut(0);

        // Track if we produce any sound during this block
        let mut produced_sound = false;

        let cutoff = 1000.0;
        let coeff = 1.0 - (-2.0 * std::f32::consts::PI * cutoff / self.sample_rate).exp();

        for i in 0..ctx.frames {
            let env = self.process_sample();
            let gain = if env > 0.0 {
                self.smooth_level += (env - self.smooth_level) * coeff;
                self.smooth_level.sqrt().min(1.0)
            } else {
                0.0
            };

            if gain > 0.0 {
                produced_sound = true;
            }

            // If we have input, multiply by envelope
            // Otherwise, output raw envelope value
            buf[i] = if has_input {
                inputs[0].channel(0).get(i).copied().unwrap_or(0.0) * gain
            } else {
                gain
            };
        }

        // Only report silent if we produced no sound during the entire block
        !produced_sound
    }

    fn num_channels(&self) -> usize {
        1
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::ATTACK => self.attack = value.max(0.001),
            params::DECAY => self.decay = value.max(0.001),
            params::SUSTAIN => self.sustain = value.clamp(0.0, 1.0),
            params::RELEASE => self.release = value.max(0.001),
            params::ATTACK_CURVE => self.attack_curve = value.clamp(0.0, 1.0),
            params::DECAY_CURVE => self.decay_curve = value.clamp(0.0, 1.0),
            params::RELEASE_CURVE => self.release_curve = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.stage = EnvelopeStage::Idle;
        self.level = 0.0;
        self.smooth_level = 0.0;
        self.stage_phase = 0.0;
        self.last_note = None;
    }

    fn needs_release(&self) -> bool {
        true
    }
}
