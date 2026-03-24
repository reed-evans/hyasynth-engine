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
        if let Some(input_buf) = inputs.get(0) {
            for ch in 0..output.channels {
                // For mono-to-stereo: use channel 0 for all output channels if input has fewer
                let in_ch_idx = ch.min(input_buf.channels.saturating_sub(1));
                let input = input_buf.channel(in_ch_idx);
                let out = output.channel_mut(ch);
                for i in 0..ctx.frames {
                    out[i] = input.get(i).copied().unwrap_or(0.0) * self.gain_linear;
                }
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
// Mixer Node
// ═══════════════════════════════════════════════════════════════════

/// Sums multiple stereo inputs together with a master gain.
pub struct MixerNode {
    gain_db: f32,
    gain_linear: f32,
}

impl MixerNode {
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

impl Default for MixerNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for MixerNode {
    fn prepare(&mut self, _sample_rate: f64, _max_block: usize) {}

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        output.clear();

        if inputs.is_empty() {
            return false;
        }

        // Sum all inputs
        for input in inputs {
            for ch in 0..output.channels {
                // For mono-to-stereo: use channel 0 for all output channels if input has fewer
                let in_ch_idx = ch.min(input.channels.saturating_sub(1));
                let in_ch = input.channel(in_ch_idx);
                let out_ch = output.channel_mut(ch);
                for i in 0..ctx.frames {
                    out_ch[i] += in_ch.get(i).copied().unwrap_or(0.0);
                }
            }
        }

        // Apply master gain
        if (self.gain_linear - 1.0).abs() > 0.0001 {
            for ch in 0..output.channels {
                let out_ch = output.channel_mut(ch);
                for i in 0..ctx.frames {
                    out_ch[i] *= self.gain_linear;
                }
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
// Delay Node
// ═══════════════════════════════════════════════════════════════════

/// Simple stereo delay effect.
pub struct DelayNode {
    delay_time: f32,
    feedback: f32,
    mix: f32,

    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    write_pos_l: usize,
    write_pos_r: usize,
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
            write_pos_l: 0,
            write_pos_r: 0,
            sample_rate: 48000.0,
        }
    }

    fn delay_samples(&self) -> usize {
        let samples = (self.delay_time * self.sample_rate as f32) as usize;
        samples.clamp(1, MAX_DELAY_SAMPLES - 1)
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

        let in_l = input.channel(0);
        let in_r = if input.channels > 1 {
            input.channel(1)
        } else {
            input.channel(0)
        };

        // Process left channel
        let out_l = output.channel_mut(0);
        for i in 0..ctx.frames {
            let dry = in_l.get(i).copied().unwrap_or(0.0);
            let read_pos = (self.write_pos_l + buf_len - delay_samples) % buf_len;
            let delayed = self.buffer_l[read_pos];

            self.buffer_l[self.write_pos_l] = dry + delayed * self.feedback;
            out_l[i] = dry * (1.0 - self.mix) + delayed * self.mix;

            self.write_pos_l = (self.write_pos_l + 1) % buf_len;
        }

        // Process right channel
        let out_r = output.channel_mut(1);
        for i in 0..ctx.frames {
            let dry = in_r.get(i).copied().unwrap_or(0.0);
            let read_pos = (self.write_pos_r + buf_len - delay_samples) % buf_len;
            let delayed = self.buffer_r[read_pos];

            self.buffer_r[self.write_pos_r] = dry + delayed * self.feedback;
            out_r[i] = dry * (1.0 - self.mix) + delayed * self.mix;

            self.write_pos_r = (self.write_pos_r + 1) % buf_len;
        }

        false
    }

    fn num_channels(&self) -> usize {
        2
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::TIME => self.delay_time = value.clamp(0.001, 2.0),
            params::FEEDBACK => self.feedback = value.clamp(0.0, 0.99),
            params::MIX => self.mix = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.buffer_l.fill(0.0);
        self.buffer_r.fill(0.0);
        self.write_pos_l = 0;
        self.write_pos_r = 0;
    }
}

// ═══════════════════════════════════════════════════════════════════
// Reverb Node (Freeverb-inspired Schroeder reverb)
// ═══════════════════════════════════════════════════════════════════

const NUM_COMBS: usize = 8;
const NUM_ALLPASSES: usize = 4;

/// Stereo offset applied to R channel delay lengths for decorrelation.
const STEREO_SPREAD: usize = 23;

/// Comb filter delay lengths at 44100 Hz (Freeverb tunings).
const COMB_TUNINGS: [usize; NUM_COMBS] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];

/// Allpass filter delay lengths at 44100 Hz (Freeverb tunings).
const ALLPASS_TUNINGS: [usize; NUM_ALLPASSES] = [556, 441, 341, 225];

const ALLPASS_FEEDBACK: f32 = 0.5;

/// Internal comb filter unit with damped feedback.
struct Comb {
    buffer: Vec<f32>,
    pos: usize,
    filter_state: f32,
}

impl Comb {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size],
            pos: 0,
            filter_state: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32, feedback: f32, damping: f32) -> f32 {
        let delayed = self.buffer[self.pos];
        // One-pole lowpass on feedback path for high-frequency damping
        self.filter_state = delayed * (1.0 - damping) + self.filter_state * damping;
        self.buffer[self.pos] = input + self.filter_state * feedback;
        self.pos = (self.pos + 1) % self.buffer.len();
        delayed
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.pos = 0;
        self.filter_state = 0.0;
    }
}

/// Internal allpass filter unit.
struct Allpass {
    buffer: Vec<f32>,
    pos: usize,
}

impl Allpass {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size],
            pos: 0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.buffer[self.pos];
        self.buffer[self.pos] = input + delayed * ALLPASS_FEEDBACK;
        self.pos = (self.pos + 1) % self.buffer.len();
        delayed - input * ALLPASS_FEEDBACK
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.pos = 0;
    }
}

/// Freeverb-inspired algorithmic reverb with true stereo processing.
///
/// Uses 8 parallel comb filters and 4 series allpass filters per channel.
/// L and R channels have independent filter state with slightly offset
/// delay lengths for stereo decorrelation.
pub struct ReverbNode {
    decay: f32,
    damping: f32,
    mix: f32,

    combs_l: [Comb; NUM_COMBS],
    combs_r: [Comb; NUM_COMBS],
    allpasses_l: [Allpass; NUM_ALLPASSES],
    allpasses_r: [Allpass; NUM_ALLPASSES],

    sample_rate: f64,
}

impl ReverbNode {
    pub fn new() -> Self {
        Self::with_sample_rate(44100.0)
    }

    fn with_sample_rate(sr: f64) -> Self {
        let scale = sr / 44100.0;
        Self {
            decay: 0.5,
            damping: 0.5,
            mix: 0.3,
            combs_l: std::array::from_fn(|i| {
                Comb::new((COMB_TUNINGS[i] as f64 * scale) as usize + 1)
            }),
            combs_r: std::array::from_fn(|i| {
                Comb::new((COMB_TUNINGS[i] as f64 * scale) as usize + STEREO_SPREAD + 1)
            }),
            allpasses_l: std::array::from_fn(|i| {
                Allpass::new((ALLPASS_TUNINGS[i] as f64 * scale) as usize + 1)
            }),
            allpasses_r: std::array::from_fn(|i| {
                Allpass::new((ALLPASS_TUNINGS[i] as f64 * scale) as usize + STEREO_SPREAD + 1)
            }),
            sample_rate: sr,
        }
    }

    /// Map the user-facing decay parameter (0..1) to a feedback coefficient.
    /// At 0.0 → 0.7 (short but smooth), at 1.0 → 0.98 (very long tail).
    #[inline]
    fn feedback(&self) -> f32 {
        self.decay * 0.28 + 0.7
    }
}

impl Default for ReverbNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for ReverbNode {
    fn prepare(&mut self, sample_rate: f64, _max_block: usize) {
        if (self.sample_rate - sample_rate).abs() > 0.1 {
            let decay = self.decay;
            let damping = self.damping;
            let mix = self.mix;
            *self = Self::with_sample_rate(sample_rate);
            self.decay = decay;
            self.damping = damping;
            self.mix = mix;
        }
    }

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
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

        let feedback = self.feedback();
        let damping = self.damping;
        let mix = self.mix;

        // Process left channel
        let out_l = output.channel_mut(0);
        for i in 0..ctx.frames {
            let dry = in_l.get(i).copied().unwrap_or(0.0);
            let inp = dry * 0.075;

            let mut wet = 0.0_f32;
            for c in 0..NUM_COMBS {
                wet += self.combs_l[c].process(inp, feedback, damping);
            }

            for a in 0..NUM_ALLPASSES {
                wet = self.allpasses_l[a].process(wet);
            }

            out_l[i] = dry * (1.0 - mix) + wet * mix;
        }

        // Process right channel (independent filter state)
        let out_r = output.channel_mut(1);
        for i in 0..ctx.frames {
            let dry = in_r.get(i).copied().unwrap_or(0.0);
            let inp = dry * 0.015;

            let mut wet = 0.0_f32;
            for c in 0..NUM_COMBS {
                wet += self.combs_r[c].process(inp, feedback, damping);
            }

            for a in 0..NUM_ALLPASSES {
                wet = self.allpasses_r[a].process(wet);
            }

            out_r[i] = dry * (1.0 - mix) + wet * mix;
        }

        false
    }

    fn num_channels(&self) -> usize {
        2
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            params::REVERB_DECAY => self.decay = value.clamp(0.0, 0.99),
            params::DAMPING => self.damping = value.clamp(0.0, 1.0),
            params::MIX => self.mix = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn reset(&mut self) {
        for c in &mut self.combs_l {
            c.reset();
        }
        for c in &mut self.combs_r {
            c.reset();
        }
        for a in &mut self.allpasses_l {
            a.reset();
        }
        for a in &mut self.allpasses_r {
            a.reset();
        }
    }
}
