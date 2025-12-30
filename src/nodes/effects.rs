// Audio effect nodes.

use crate::audio_buffer::AudioBuffer;
use crate::node::{Node, ProcessContext};

use super::params;

// ═══════════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════════

const MAX_DELAY_SAMPLES: usize = 192_000 * 2; // 2 seconds at 192kHz

// ═══════════════════════════════════════════════════════════════════
// Gain Node
// ═══════════════════════════════════════════════════════════════════

pub struct GainNode {
    gain_db: f32,
    gain_linear: f32,
}

impl GainNode {
    pub fn new() -> Self {
        Self {
            gain_db: 0.0,
            gain_linear: 1.0,
        }
    }

    fn update_linear(&mut self) {
        self.gain_linear = 10.0_f32.powf(self.gain_db / 20.0);
    }
}

impl Default for GainNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for GainNode {
    fn prepare(&mut self, _sample_rate: f64, _max_block: usize) {}

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        if self.gain_linear < 0.0001 {
            output.clear();
            return true;
        }

        // Copy and scale input to output
        for ch in 0..output.channels.min(inputs.get(0).map_or(0, |i| i.channels)) {
            let input = inputs[0].channel(ch);
            let out = output.channel_mut(ch);
            for i in 0..ctx.frames {
                out[i] = input.get(i).copied().unwrap_or(0.0) * self.gain_linear;
            }
        }

        false
    }

    fn num_channels(&self) -> usize {
        2
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::GAIN => {
                self.gain_db = value;
                self.update_linear();
            }
            _ => {}
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Pan Node (constant power panning)
// ═══════════════════════════════════════════════════════════════════

pub struct PanNode {
    pan: f32, // -1 (left) to +1 (right)
    left_gain: f32,
    right_gain: f32,
}

impl PanNode {
    pub fn new() -> Self {
        let mut node = Self {
            pan: 0.0,
            left_gain: 1.0,
            right_gain: 1.0,
        };
        node.update_gains();
        node
    }

    fn update_gains(&mut self) {
        // Constant power panning
        let angle = (self.pan + 1.0) * 0.25 * std::f32::consts::PI;
        self.left_gain = angle.cos();
        self.right_gain = angle.sin();
    }
}

impl Default for PanNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for PanNode {
    fn prepare(&mut self, _sample_rate: f64, _max_block: usize) {}

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        if inputs.is_empty() || output.channels < 2 {
            return true;
        }

        let input = inputs[0];
        // Treat first channel as mono input
        let mono = input.channel(0);

        let left = output.channel_mut(0);
        for i in 0..ctx.frames {
            left[i] = mono.get(i).copied().unwrap_or(0.0) * self.left_gain;
        }

        // Need to get channel 1 separately due to borrow rules
        let right = output.channel_mut(1);
        for i in 0..ctx.frames {
            right[i] = mono.get(i).copied().unwrap_or(0.0) * self.right_gain;
        }

        false
    }

    fn num_channels(&self) -> usize {
        2
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::PAN => {
                self.pan = value.clamp(-1.0, 1.0);
                self.update_gains();
            }
            _ => {}
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Delay Node
// ═══════════════════════════════════════════════════════════════════

/// Simple stereo delay effect.
pub struct DelayNode {
    delay_time: f32,    // In seconds
    feedback: f32,      // 0.0 - 1.0
    mix: f32,           // Dry/wet mix (0.0 = dry, 1.0 = wet)

    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    write_pos: usize,
    sample_rate: f64,
}

impl DelayNode {
    pub fn new() -> Self {
        Self {
            delay_time: 0.25,
            feedback: 0.4,
            mix: 0.5,
            buffer_l: vec![0.0; MAX_DELAY_SAMPLES],
            buffer_r: vec![0.0; MAX_DELAY_SAMPLES],
            write_pos: 0,
            sample_rate: 48000.0,
        }
    }

    fn delay_samples(&self) -> usize {
        let samples = (self.delay_time * self.sample_rate as f32) as usize;
        samples.min(MAX_DELAY_SAMPLES - 1)
    }
}

impl Default for DelayNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for DelayNode {
    fn prepare(&mut self, sample_rate: f64, _max_block: usize) {
        self.sample_rate = sample_rate;
    }

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        self.sample_rate = ctx.sample_rate;

        if inputs.is_empty() {
            output.clear();
            return false;
        }

        let input = inputs[0];
        let delay_samples = self.delay_samples();
        let buf_len = self.buffer_l.len();

        // Process left channel
        let in_l = input.channel(0);
        let out_l = output.channel_mut(0);

        for i in 0..ctx.frames {
            let dry = in_l.get(i).copied().unwrap_or(0.0);
            let read_pos = (self.write_pos + buf_len - delay_samples) % buf_len;
            let delayed = self.buffer_l[read_pos];

            self.buffer_l[self.write_pos] = dry + delayed * self.feedback;
            out_l[i] = dry * (1.0 - self.mix) + delayed * self.mix;

            self.write_pos = (self.write_pos + 1) % buf_len;
        }

        // Reset write_pos for right channel
        self.write_pos = (self.write_pos + buf_len - ctx.frames) % buf_len;

        // Process right channel (or copy left if mono)
        let in_r = if input.channels > 1 {
            input.channel(1)
        } else {
            input.channel(0)
        };
        let out_r = output.channel_mut(1);

        for i in 0..ctx.frames {
            let dry = in_r.get(i).copied().unwrap_or(0.0);
            let read_pos = (self.write_pos + buf_len - delay_samples) % buf_len;
            let delayed = self.buffer_r[read_pos];

            self.buffer_r[self.write_pos] = dry + delayed * self.feedback;
            out_r[i] = dry * (1.0 - self.mix) + delayed * self.mix;

            self.write_pos = (self.write_pos + 1) % buf_len;
        }

        true
    }

    fn num_channels(&self) -> usize {
        2
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            0 => self.delay_time = value.clamp(0.001, 2.0),  // Time in seconds
            1 => self.feedback = value.clamp(0.0, 0.99),     // Feedback
            2 => self.mix = value.clamp(0.0, 1.0),           // Mix
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.buffer_l.fill(0.0);
        self.buffer_r.fill(0.0);
        self.write_pos = 0;
    }
}

// ═══════════════════════════════════════════════════════════════════
// Reverb Node (simple Schroeder reverb)
// ═══════════════════════════════════════════════════════════════════

/// Simple algorithmic reverb using a Schroeder topology.
///
/// Uses 4 parallel comb filters and 2 series allpass filters.
pub struct ReverbNode {
    decay: f32,         // Decay time (0.0 - 1.0)
    damping: f32,       // High frequency damping (0.0 - 1.0)
    mix: f32,           // Dry/wet mix

    // Comb filter buffers (4 parallel)
    comb_buffers: [Vec<f32>; 4],
    comb_pos: [usize; 4],
    comb_filter: [f32; 4],  // Low-pass filtered feedback

    // Allpass filter buffers (2 series)
    allpass_buffers: [Vec<f32>; 2],
    allpass_pos: [usize; 2],

    sample_rate: f64,
}

// Comb filter delay times in samples (for 48kHz, scaled later)
const COMB_DELAYS: [usize; 4] = [1557, 1617, 1491, 1422];
const ALLPASS_DELAYS: [usize; 2] = [225, 556];

impl ReverbNode {
    pub fn new() -> Self {
        Self {
            decay: 0.5,
            damping: 0.5,
            mix: 0.3,
            comb_buffers: [
                vec![0.0; 4096],
                vec![0.0; 4096],
                vec![0.0; 4096],
                vec![0.0; 4096],
            ],
            comb_pos: [0; 4],
            comb_filter: [0.0; 4],
            allpass_buffers: [vec![0.0; 1024], vec![0.0; 1024]],
            allpass_pos: [0; 2],
            sample_rate: 48000.0,
        }
    }

    fn comb_delay(&self, index: usize) -> usize {
        let base = COMB_DELAYS[index];
        let scaled = (base as f64 * self.sample_rate / 48000.0) as usize;
        scaled.min(self.comb_buffers[index].len() - 1)
    }

    fn allpass_delay(&self, index: usize) -> usize {
        let base = ALLPASS_DELAYS[index];
        let scaled = (base as f64 * self.sample_rate / 48000.0) as usize;
        scaled.min(self.allpass_buffers[index].len() - 1)
    }

    #[inline]
    fn process_comb(&mut self, index: usize, input: f32) -> f32 {
        let delay = self.comb_delay(index);
        let buf_len = self.comb_buffers[index].len();
        let read_pos = (self.comb_pos[index] + buf_len - delay) % buf_len;

        let delayed = self.comb_buffers[index][read_pos];

        // Low-pass filtered feedback for damping
        self.comb_filter[index] = delayed * (1.0 - self.damping) + self.comb_filter[index] * self.damping;

        let feedback = self.comb_filter[index] * self.decay;
        self.comb_buffers[index][self.comb_pos[index]] = input + feedback;
        self.comb_pos[index] = (self.comb_pos[index] + 1) % buf_len;

        delayed
    }

    #[inline]
    fn process_allpass(&mut self, index: usize, input: f32) -> f32 {
        let delay = self.allpass_delay(index);
        let buf_len = self.allpass_buffers[index].len();
        let read_pos = (self.allpass_pos[index] + buf_len - delay) % buf_len;

        let delayed = self.allpass_buffers[index][read_pos];
        let g = 0.5_f32; // Allpass coefficient

        let output = -g * input + delayed;
        self.allpass_buffers[index][self.allpass_pos[index]] = input + g * delayed;
        self.allpass_pos[index] = (self.allpass_pos[index] + 1) % buf_len;

        output
    }
}

impl Default for ReverbNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for ReverbNode {
    fn prepare(&mut self, sample_rate: f64, _max_block: usize) {
        self.sample_rate = sample_rate;
    }

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        self.sample_rate = ctx.sample_rate;

        if inputs.is_empty() {
            output.clear();
            return false;
        }

        let input = inputs[0];
        let in_l = input.channel(0);
        let in_r = if input.channels > 1 {
            input.channel(1)
        } else {
            input.channel(0)
        };

        let out_l = output.channel_mut(0);

        for i in 0..ctx.frames {
            let dry_l = in_l.get(i).copied().unwrap_or(0.0);
            let dry_r = in_r.get(i).copied().unwrap_or(0.0);
            let mono = (dry_l + dry_r) * 0.5;

            // Parallel comb filters
            let mut wet = 0.0_f32;
            for c in 0..4 {
                wet += self.process_comb(c, mono);
            }
            wet *= 0.25;

            // Series allpass filters
            wet = self.process_allpass(0, wet);
            wet = self.process_allpass(1, wet);

            out_l[i] = dry_l * (1.0 - self.mix) + wet * self.mix;
        }

        // Process right channel (same reverb, different dry)
        // Reset positions for right channel
        for c in 0..4 {
            self.comb_pos[c] = (self.comb_pos[c] + self.comb_buffers[c].len() - ctx.frames)
                % self.comb_buffers[c].len();
        }
        for a in 0..2 {
            self.allpass_pos[a] = (self.allpass_pos[a] + self.allpass_buffers[a].len() - ctx.frames)
                % self.allpass_buffers[a].len();
        }

        let out_r = output.channel_mut(1);

        for i in 0..ctx.frames {
            let dry_l = in_l.get(i).copied().unwrap_or(0.0);
            let dry_r = in_r.get(i).copied().unwrap_or(0.0);
            let mono = (dry_l + dry_r) * 0.5;

            let mut wet = 0.0_f32;
            for c in 0..4 {
                wet += self.process_comb(c, mono);
            }
            wet *= 0.25;

            wet = self.process_allpass(0, wet);
            wet = self.process_allpass(1, wet);

            out_r[i] = dry_r * (1.0 - self.mix) + wet * self.mix;
        }

        true
    }

    fn num_channels(&self) -> usize {
        2
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            0 => self.decay = value.clamp(0.0, 0.99),    // Decay
            1 => self.damping = value.clamp(0.0, 1.0),   // Damping
            2 => self.mix = value.clamp(0.0, 1.0),       // Mix
            _ => {}
        }
    }

    fn reset(&mut self) {
        for buf in &mut self.comb_buffers {
            buf.fill(0.0);
        }
        for buf in &mut self.allpass_buffers {
            buf.fill(0.0);
        }
        self.comb_pos = [0; 4];
        self.allpass_pos = [0; 2];
        self.comb_filter = [0.0; 4];
    }
}
