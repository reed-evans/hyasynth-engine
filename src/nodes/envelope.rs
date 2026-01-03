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

pub struct AdsrEnvelope {
    stage: EnvelopeStage,
    level: f32,
    smooth_level: f32,

    // Parameters (in seconds)
    attack: f32,
    decay: f32,
    sustain: f32, // 0-1 level
    release: f32,

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
            sample_rate: 48_000.0,
            release_level: 0.0,
            last_note: None,
        }
    }

    #[inline]
    fn process_sample(&mut self) -> f32 {
        match self.stage {
            EnvelopeStage::Idle => 0.0,

            EnvelopeStage::Attack => {
                let rate = 1.0 / (self.attack * self.sample_rate).max(1.0);
                self.level += rate;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = EnvelopeStage::Decay;
                }
                self.level
            }

            EnvelopeStage::Decay => {
                let rate = (1.0 - self.sustain) / (self.decay * self.sample_rate).max(1.0);
                self.level -= rate;
                if self.level <= self.sustain {
                    self.level = self.sustain;
                    self.stage = EnvelopeStage::Sustain;
                }
                self.level
            }

            EnvelopeStage::Sustain => self.sustain,

            EnvelopeStage::Release => {
                let rate = self.release_level / (self.release * self.sample_rate).max(1.0);
                self.level -= rate;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = EnvelopeStage::Idle;
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
                self.stage = EnvelopeStage::Attack;
                self.last_note = Some(voice.note);
            }
            if voice.release
                && self.stage != EnvelopeStage::Idle
                && self.stage != EnvelopeStage::Release
            {
                self.release_level = self.level;
                self.stage = EnvelopeStage::Release;
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
            self.smooth_level += (env - self.smooth_level) * coeff;
            let gain = self.smooth_level.sqrt();

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
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.stage = EnvelopeStage::Idle;
        self.level = 0.0;
        self.smooth_level = 0.0;
        self.last_note = None;
    }
}
