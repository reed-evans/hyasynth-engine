// src/nodes/audio_player.rs
//
// Audio Player Node - Plays back audio regions from the audio pool.
//
// This node receives audio start/stop commands and plays back
// audio data from a shared audio pool. It supports multiple
// simultaneous playback "voices" for overlapping regions.

use std::collections::HashMap;
use std::sync::Arc;

use crate::audio_buffer::AudioBuffer;
use crate::node::{Node, Polyphony, ProcessContext};
use crate::state::AudioPoolId;

/// Maximum number of simultaneous audio playback voices.
const MAX_AUDIO_VOICES: usize = 16;

/// Shared audio data that can be passed to the audio player.
///
/// This is an Arc-wrapped slice of samples that can be safely
/// shared between the UI and audio threads.
#[derive(Debug, Clone)]
pub struct SharedAudioData {
    /// Unique ID from the audio pool.
    pub id: AudioPoolId,
    /// Sample rate of the audio.
    pub sample_rate: f64,
    /// Number of channels (1 = mono, 2 = stereo).
    pub channels: usize,
    /// Total number of frames.
    pub frames: usize,
    /// The actual sample data (interleaved if stereo).
    pub samples: Arc<Vec<f32>>,
}

impl SharedAudioData {
    /// Create from an AudioPoolEntry.
    pub fn from_pool_entry(entry: &crate::state::AudioPoolEntry) -> Self {
        Self {
            id: entry.id,
            sample_rate: entry.sample_rate,
            channels: entry.channels,
            frames: entry.frames,
            samples: Arc::clone(&entry.samples),
        }
    }
}

/// A single audio playback voice.
#[derive(Debug, Clone)]
struct AudioVoice {
    /// The audio data being played.
    data: SharedAudioData,
    /// Current playback position (in frames).
    position: usize,
    /// Remaining frames to play.
    remaining: usize,
    /// Gain level.
    gain: f32,
    /// Whether this voice is active.
    active: bool,
}

impl AudioVoice {
    fn new(data: SharedAudioData, start_frame: usize, duration_frames: usize, gain: f32) -> Self {
        Self {
            data,
            position: start_frame,
            remaining: duration_frames,
            gain,
            active: true,
        }
    }

    /// Process one block of audio, writing to the output buffer.
    /// Returns true if the voice finished.
    fn process(&mut self, output: &mut [f32], output_channels: usize) -> bool {
        if !self.active {
            return true;
        }

        let frames_to_process = (output.len() / output_channels).min(self.remaining);
        let samples = &self.data.samples;
        let src_channels = self.data.channels;

        for frame in 0..frames_to_process {
            let src_frame = self.position + frame;

            // Check bounds
            if src_frame >= self.data.frames {
                self.active = false;
                return true;
            }

            // Read source samples
            for ch in 0..output_channels {
                let src_ch = ch % src_channels; // Handle mono -> stereo
                let src_idx = src_frame * src_channels + src_ch;
                let dst_idx = frame * output_channels + ch;

                if src_idx < samples.len() && dst_idx < output.len() {
                    output[dst_idx] += samples[src_idx] * self.gain;
                }
            }
        }

        self.position += frames_to_process;
        self.remaining -= frames_to_process;

        if self.remaining == 0 {
            self.active = false;
            true
        } else {
            false
        }
    }
}

/// Audio player node that plays back audio regions.
///
/// This node receives audio start/stop commands and outputs
/// the mixed audio from all active playback voices.
pub struct AudioPlayerNode {
    /// Available audio data (loaded from pool).
    audio_data: HashMap<AudioPoolId, SharedAudioData>,
    
    /// Active playback voices.
    voices: Vec<Option<AudioVoice>>,
    
    /// Number of output channels.
    channels: usize,
    
    /// Current sample rate.
    sample_rate: f64,
    
    /// Master gain.
    gain: f32,
    
    /// Scratch buffer for mixing.
    scratch: Vec<f32>,
}

impl AudioPlayerNode {
    pub fn new(channels: usize) -> Self {
        Self {
            audio_data: HashMap::new(),
            voices: vec![None; MAX_AUDIO_VOICES],
            channels,
            sample_rate: 48000.0,
            gain: 1.0,
            scratch: Vec::new(),
        }
    }

    /// Load audio data into the player.
    ///
    /// Call this when audio is added to the pool.
    pub fn load_audio(&mut self, data: SharedAudioData) {
        self.audio_data.insert(data.id, data);
    }

    /// Unload audio data from the player.
    pub fn unload_audio(&mut self, id: AudioPoolId) {
        self.audio_data.remove(&id);
        // Stop any voices playing this audio
        for voice in &mut self.voices {
            if let Some(v) = voice {
                if v.data.id == id {
                    v.active = false;
                }
            }
        }
    }

    /// Start playing an audio region.
    ///
    /// - `audio_id`: The audio pool entry to play
    /// - `start_sample`: Offset into the source audio
    /// - `duration_samples`: How long to play
    /// - `gain`: Playback gain
    pub fn start_audio(
        &mut self,
        audio_id: AudioPoolId,
        start_sample: u64,
        duration_samples: u64,
        gain: f32,
    ) {
        let Some(data) = self.audio_data.get(&audio_id).cloned() else {
            return;
        };

        // Find an empty voice slot
        let slot = self.voices.iter_mut().find(|v| v.is_none() || !v.as_ref().unwrap().active);

        if let Some(slot) = slot {
            *slot = Some(AudioVoice::new(
                data,
                start_sample as usize,
                duration_samples as usize,
                gain,
            ));
        }
        // If no slots available, the audio is dropped (could log a warning)
    }

    /// Stop playing a specific audio.
    pub fn stop_audio(&mut self, audio_id: AudioPoolId) {
        for voice in &mut self.voices {
            if let Some(v) = voice {
                if v.data.id == audio_id {
                    v.active = false;
                }
            }
        }
    }

    /// Stop all playback.
    pub fn stop_all(&mut self) {
        for voice in &mut self.voices {
            *voice = None;
        }
    }

    /// Check if any audio is playing.
    pub fn is_playing(&self) -> bool {
        self.voices.iter().any(|v| v.as_ref().map(|v| v.active).unwrap_or(false))
    }

    /// Get the number of active voices.
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.as_ref().map(|v| v.active).unwrap_or(false)).count()
    }
}

impl Node for AudioPlayerNode {
    fn prepare(&mut self, sample_rate: f64, max_block: usize) {
        self.sample_rate = sample_rate;
        self.scratch.resize(max_block * self.channels, 0.0);
    }

    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool {
        let frames = ctx.frames;
        let out_samples = output.samples_mut();

        // Mix any input through
        if let Some(input) = inputs.first() {
            let in_samples = input.samples();
            let copy_len = out_samples.len().min(in_samples.len());
            out_samples[..copy_len].copy_from_slice(&in_samples[..copy_len]);
        } else {
            // Clear output if no input
            out_samples.fill(0.0);
        }

        // Process each active voice and mix into output
        for voice in &mut self.voices {
            if let Some(v) = voice {
                if v.active {
                    // Process this voice (adds to output)
                    let output_slice = &mut out_samples[..frames * self.channels];
                    v.process(output_slice, self.channels);
                }
            }
        }

        // Apply master gain
        if (self.gain - 1.0).abs() > 0.0001 {
            for sample in out_samples.iter_mut() {
                *sample *= self.gain;
            }
        }

        // Check if silent (no active voices and no input)
        !self.is_playing() && inputs.is_empty()
    }

    fn num_channels(&self) -> usize {
        self.channels
    }

    fn polyphony(&self) -> Polyphony {
        Polyphony::Global
    }

    fn set_param(&mut self, param_id: u32, value: f32) {
        match param_id {
            0 => self.gain = value.max(0.0), // GAIN
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.stop_all();
    }

    fn start_audio(
        &mut self,
        audio_id: AudioPoolId,
        start_sample: u64,
        duration_samples: u64,
        gain: f32,
    ) {
        AudioPlayerNode::start_audio(self, audio_id, start_sample, duration_samples, gain);
    }

    fn stop_audio(&mut self, audio_id: AudioPoolId) {
        AudioPlayerNode::stop_audio(self, audio_id);
    }

    fn handles_audio(&self) -> bool {
        true
    }

    fn load_audio(&mut self, data: SharedAudioData) {
        AudioPlayerNode::load_audio(self, data);
    }

    fn unload_audio(&mut self, audio_id: AudioPoolId) {
        AudioPlayerNode::unload_audio(self, audio_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_audio() -> SharedAudioData {
        // Create a simple sine wave test audio
        let sample_rate = 48000.0;
        let frames = 48000; // 1 second
        let channels = 2;
        
        let mut samples = Vec::with_capacity(frames * channels);
        for i in 0..frames {
            let t = i as f32 / sample_rate as f32;
            let sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.5;
            samples.push(sample); // L
            samples.push(sample); // R
        }

        SharedAudioData {
            id: 1,
            sample_rate,
            channels,
            frames,
            samples: Arc::new(samples),
        }
    }

    #[test]
    fn test_audio_player_basic() {
        let mut player = AudioPlayerNode::new(2);
        player.prepare(48000.0, 512);

        // Load audio
        let audio = make_test_audio();
        player.load_audio(audio);

        // Start playback
        player.start_audio(1, 0, 48000, 1.0);
        assert!(player.is_playing());
        assert_eq!(player.active_voice_count(), 1);

        // Process some audio
        let ctx = ProcessContext::new(512, 48000.0, 0, 120.0);
        let mut output_data = vec![0.0f32; 512 * 2];
        let mut output = AudioBuffer::new(&mut output_data, 2);
        
        player.process(&ctx, &[], &mut output);

        // Output should have audio data
        assert!(output_data.iter().any(|&s| s.abs() > 0.0));
    }

    #[test]
    fn test_audio_player_stop() {
        let mut player = AudioPlayerNode::new(2);
        player.prepare(48000.0, 512);

        let audio = make_test_audio();
        player.load_audio(audio);

        player.start_audio(1, 0, 48000, 1.0);
        assert!(player.is_playing());

        player.stop_audio(1);
        
        // Voice is marked inactive but still in slot
        assert!(!player.is_playing());
    }
}

