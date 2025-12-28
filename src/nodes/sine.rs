// // src/nodes/sine.rs

// use crate::node::*;
// use crate::process::*;
// use crate::signal::*;

// pub struct SineOsc {
//     phase: f32,
//     freq: f32,
//     sample_rate: f32,
// }

// impl SineOsc {
//     pub fn new(freq: f32) -> Self {
//         Self {
//             phase: 0.0,
//             freq,
//             sample_rate: 48_000.0,
//         }
//     }
// }

// impl Node for SineOsc {
//     fn prepare(&mut self, sample_rate: f64, _max_block: usize) {
//         self.sample_rate = sample_rate as f32;
//     }

//     fn process_slice(
//         &mut self,
//         slice: &TimeSlice,
//         output: &mut AudioBuffer,
//     ) -> bool {
//         if !slice.transport.playing {
//             output.clear();
//             return true;
//         }

//         let inc = self.freq / self.sample_rate;

//         for ch in 0..output.channels {
//             let buf = output.channel_mut(ch);
//             for i in 0..slice.frame_count {
//                 buf[i] = (self.phase * std::f32::consts::TAU).sin();
//                 self.phase = (self.phase + inc) % 1.0;
//             }
//         }

//         false // not silent
//     }

//     fn num_channels(&self) -> usize {
//         1
//     }
// }
