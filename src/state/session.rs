// src/state/session.rs
//
// Session/Project state.
//
// The Session represents the complete state of a project.
// It can be serialized for save/load.

use super::GraphDef;

/// Transport state visible to the UI.
#[derive(Debug, Clone, Default)]
pub struct TransportState {
    /// Whether the transport is playing.
    pub playing: bool,

    /// Current tempo in BPM.
    pub bpm: f64,

    /// Current position in beats.
    pub beat_position: f64,

    /// Current position in samples.
    pub sample_position: u64,

    /// Loop enabled.
    pub loop_enabled: bool,

    /// Loop start in beats.
    pub loop_start: f64,

    /// Loop end in beats.
    pub loop_end: f64,
}

impl TransportState {
    pub fn new() -> Self {
        Self {
            playing: false,
            bpm: 120.0,
            beat_position: 0.0,
            sample_position: 0,
            loop_enabled: false,
            loop_start: 0.0,
            loop_end: 4.0,
        }
    }
}

/// Complete session state.
///
/// This is the top-level document that represents a project.
/// The UI owns this and the bridge synchronizes relevant parts
/// to the real-time engine.
#[derive(Debug, Clone)]
pub struct Session {
    /// Project name.
    pub name: String,

    /// The audio graph.
    pub graph: GraphDef,

    /// Transport state (UI mirror of engine transport).
    pub transport: TransportState,

    /// Sample rate (set once on engine init).
    pub sample_rate: f64,

    /// Maximum voices for polyphony.
    pub max_voices: usize,

    /// Maximum block size.
    pub max_block_size: usize,
}

impl Session {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            graph: GraphDef::new(),
            transport: TransportState::new(),
            sample_rate: 48_000.0,
            max_voices: 8,
            max_block_size: 512,
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new("Untitled")
    }
}

/// Read-only engine state for UI display.
///
/// This is updated by the engine and read by the UI.
/// Contains things like meters, playhead position, active voices, etc.
#[derive(Debug, Clone, Default)]
pub struct EngineReadback {
    /// Current sample position (for playhead display).
    pub sample_position: u64,

    /// Current beat position.
    pub beat_position: f64,

    /// CPU usage (0.0 - 1.0).
    pub cpu_load: f32,

    /// Number of active voices.
    pub active_voices: usize,

    /// Peak levels per channel (for meters).
    pub output_peaks: [f32; 2],

    /// Whether the engine is currently processing.
    pub running: bool,
}

