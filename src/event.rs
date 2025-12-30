use crate::state::AudioPoolId;

/// ===============================
/// Scheduler-side musical events
/// ===============================

/// An event expressed in musical time (beats).
///
/// These events:
/// - live ONLY in the scheduler
/// - are NOT real-time safe
/// - may be cloned, reordered, quantized, etc.
#[derive(Debug, Clone)]
pub enum MusicalEvent {
    /// Note on (global, for live playing).
    NoteOn {
        beat: f64,
        note: u8,
        velocity: f32,
    },

    /// Note off (global).
    NoteOff {
        beat: f64,
        note: u8,
    },

    /// Note on targeted to a specific node (for clip playback).
    NoteOnTarget {
        beat: f64,
        node_id: u32,
        note: u8,
        velocity: f32,
    },

    /// Note off targeted to a specific node.
    NoteOffTarget {
        beat: f64,
        node_id: u32,
        note: u8,
    },

    /// Parameter change.
    ParamChange {
        beat: f64,
        node_id: u32,
        param_id: u32,
        value: f32,
    },

    /// Start audio region playback.
    AudioStart {
        beat: f64,
        /// Target node to output audio to.
        node_id: u32,
        /// Audio pool entry ID.
        audio_id: AudioPoolId,
        /// Offset into the audio (in samples).
        start_sample: u64,
        /// Duration to play (in samples).
        duration_samples: u64,
        /// Gain level.
        gain: f32,
    },

    /// Stop audio region playback.
    AudioStop {
        beat: f64,
        node_id: u32,
        audio_id: AudioPoolId,
    },
}

impl MusicalEvent {
    pub fn beat(&self) -> f64 {
        match self {
            MusicalEvent::NoteOn { beat, .. } => *beat,
            MusicalEvent::NoteOff { beat, .. } => *beat,
            MusicalEvent::NoteOnTarget { beat, .. } => *beat,
            MusicalEvent::NoteOffTarget { beat, .. } => *beat,
            MusicalEvent::ParamChange { beat, .. } => *beat,
            MusicalEvent::AudioStart { beat, .. } => *beat,
            MusicalEvent::AudioStop { beat, .. } => *beat,
        }
    }
}

/// ===============================
/// Engine-side scheduled events
/// ===============================

/// An event expressed in sample time.
///
/// These events:
/// - are RT-safe
/// - contain NO musical-time information
/// - are dispatched by the engine exactly once
#[derive(Debug, Clone)]
pub enum Event {
    /// Note on (broadcast to all voice-enabled nodes).
    NoteOn { note: u8, velocity: f32 },

    /// Note off (broadcast).
    NoteOff { note: u8 },

    /// Note on targeted to a specific node.
    NoteOnTarget {
        node_id: u32,
        note: u8,
        velocity: f32,
    },

    /// Note off targeted to a specific node.
    NoteOffTarget { node_id: u32, note: u8 },

    /// Parameter change.
    ParamChange {
        node_id: u32,
        param_id: u32,
        value: f32,
    },

    /// Start audio playback.
    AudioStart {
        /// Target node to output audio to.
        node_id: u32,
        /// Audio pool entry ID.
        audio_id: AudioPoolId,
        /// Offset into the audio (in samples).
        start_sample: u64,
        /// Duration to play (in samples).
        duration_samples: u64,
        /// Gain level.
        gain: f32,
    },

    /// Stop audio playback.
    AudioStop { node_id: u32, audio_id: AudioPoolId },
}
