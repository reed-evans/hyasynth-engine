use crate::event::Event;
use crate::transport::Transport;

//
// ===============================
// MARK: Execution plan (scheduler -> engine)
// ===============================
//

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

    /// Event-free, transport-stable slices
    pub slices: Vec<SlicePlan>,
}

//
// ===============================
// MARK: Slice plan
// ===============================
//

/// A contiguous region of samples with stable transport within a block.
///
/// Invariants:
/// - No musical or engine events occur inside a slice
/// - Transport does not change during the slice
/// - The engine must process the entire slice in one DSP call
#[derive(Debug, Clone)]
pub struct SlicePlan {
    /// Absolute sample position of the slice start
    pub start_sample: u64,

    /// Number of frames to process
    pub frame_count: usize,

    /// Fully resolved transport state for this slice
    pub transport: Transport,

    /// Events that must be applied *before* this slice runs
    pub events: Vec<Event>,
}
