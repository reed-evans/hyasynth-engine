// src/execution_plan.rs

use crate::event::Event;

/// A fully precompiled, immutable plan for executing one audio block.
///
/// Produced by the Scheduler.
/// Consumed by the Engine (audio thread).
///
/// Invariants:
/// - No allocation during engine execution
/// - No musical-time information
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Absolute sample position where this block starts
    pub block_start_sample: u64,

    /// Total number of frames in this block
    pub block_frames: usize,

    /// Current tempo in BPM
    pub bpm: f64,

    /// Sample rate
    pub sample_rate: f64,

    /// Event-free slices (for sample-accurate event timing)
    pub slices: Vec<SlicePlan>,
}

impl ExecutionPlan {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            block_start_sample: 0,
            block_frames: 0,
            bpm: 120.0,
            sample_rate,
            slices: Vec::with_capacity(16), // Pre-allocate for typical case
        }
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new(48_000.0)
    }
}

/// A contiguous region of samples within a block.
///
/// Invariants:
/// - Events are applied at the START of this slice
/// - No events occur during the slice
#[derive(Debug, Clone)]
pub struct SlicePlan {
    /// Offset from block start (in frames)
    pub frame_offset: usize,

    /// Number of frames to process
    pub frame_count: usize,

    /// Events to apply before processing this slice
    pub events: Vec<Event>,
}

impl SlicePlan {
    pub fn new(frame_offset: usize, frame_count: usize) -> Self {
        Self {
            frame_offset,
            frame_count,
            events: Vec::new(),
        }
    }
}
