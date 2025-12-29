// src/nodes/effects.rs
//
// Audio effect nodes.

use crate::audio_buffer::AudioBuffer;
use crate::node::{Node, ProcessContext};

use super::params;

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
