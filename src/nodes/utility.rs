// Utility nodes (output, mixer, etc.)

use crate::audio_buffer::AudioBuffer;
use crate::node::{Node, ProcessContext};

use super::params;

// ═══════════════════════════════════════════════════════════════════
// Output Node (final destination)
// ═══════════════════════════════════════════════════════════════════

pub struct OutputNode {
    master_db: f32,
    master_linear: f32,
}

impl OutputNode {
    pub fn new() -> Self {
        Self {
            master_db: 0.0,
            master_linear: 1.0,
        }
    }

    fn update_linear(&mut self) {
        self.master_linear = 10.0_f32.powf(self.master_db / 20.0);
    }
}

impl Default for OutputNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for OutputNode {
    fn prepare(&mut self, _sample_rate: f64, _max_block: usize) {}

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        if inputs.is_empty() {
            output.clear();
            return true;
        }

        // Mix all inputs and apply master gain
        output.clear();

        for input in inputs {
            for ch in 0..output.channels {
                // For mono-to-stereo: use channel 0 for all output channels if input has fewer
                let in_ch_idx = ch.min(input.channels.saturating_sub(1));
                let in_ch = input.channel(in_ch_idx);
                let out_ch = output.channel_mut(ch);
                for i in 0..ctx.frames {
                    out_ch[i] += in_ch.get(i).copied().unwrap_or(0.0) * self.master_linear;
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
                self.master_db = value;
                self.update_linear();
            }
            _ => {}
        }
    }
}
