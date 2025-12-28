// // src/nodes/mod_sine.rs

// use crate::{node::*, process::*, signal::*, parameter::*};

// pub struct ModSine {
//     phase: f32,
//     freq: Parameter,
//     sample_rate: f32,
// }

// impl ModSine {
//     pub fn new(freq: f32) -> Self {
//         Self {
//             phase: 0.0,
//             freq: Parameter::new(freq),
//             sample_rate: 48_000.0,
//         }
//     }
// }

// impl Node for ModSine {
//     fn prepare(&mut self, sr: f64, _max: usize) {
//         self.sample_rate = sr as f32;
//     }

//     fn process_slice(
//         &mut self,
//         slice: &TimeSlice,
//         out: &mut AudioBuffer,
//     ) -> bool {
//         if !slice.transport.playing {
//             out.clear();
//             return true;
//         }

//         for ch in 0..out.channels {
//             let buf = out.channel_mut(ch);
//             for i in 0..slice.frame_count {
//                 let freq = self.freq.value_audio(i);
//                 let inc = freq / self.sample_rate;
//                 buf[i] = (self.phase * std::f32::consts::TAU).sin();
//                 self.phase = (self.phase + inc) % 1.0;
//             }
//         }

//         false
//     }

//     fn num_channels(&self) -> usize {
//         1
//     }
// }
