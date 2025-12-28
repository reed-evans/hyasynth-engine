use crate::event::Event;
use crate::execution_plan::{ExecutionPlan, SlicePlan};
use crate::graph::Graph;
use crate::transport::Transport;
use crate::voice_allocator::VoiceAllocator;

/// Real-time audio engine.
///
/// This struct runs exclusively on the audio thread.
/// It must be deterministic, allocation-free, and lock-free.
/// It does not do musical-time reasoning.
pub struct Engine {
    /// DSP graph
    graph: Graph,

    /// Voice allocator and active voice set
    voices: VoiceAllocator,

    /// Current transport (sample domain only)
    transport: Transport,
}

impl Engine {
    pub fn new(graph: Graph, voices: VoiceAllocator) -> Self {
        Self {
            graph,
            voices,
            transport: Transport::default(),
        }
    }

    /// Execute a precompiled execution plan.
    ///
    /// Called once per audio block from the audio callback.
    /// It must not allocate or block.
    pub fn process_plan(&mut self, plan: &ExecutionPlan) {
        debug_assert!(plan.slices.iter().all(|s| s.frame_count > 0));

        // Transport is fully dictated by the plan
        self.transport.sample_pos = plan.block_start_sample;

        for slice in plan.slices.iter() {
            self.process_slice(slice);
        }
    }

    /// Execute one slice of time.
    ///
    /// This performs deterministic graph execution
    /// Event dispatch happens in the scheduler.
    #[inline(always)]
    fn process_slice(&mut self, slice: &SlicePlan) {
        // Apply events at slice boundary
        for event in &slice.events {
            self.apply_event(event);
        }

        // Transport is already resolved and stable
        self.transport = slice.transport;

        // Run DSP for the whole slice
        self.graph.process_slice(slice, &self.voices);

        // Advance global sample clock
        self.transport.sample_pos += slice.frame_count as u64;
    }

    /// Apply a musical event immediately.
    ///
    /// This is the *only* place where musical intent mutates
    /// engine-visible state.
    #[inline]
    fn apply_event(&mut self, event: &Event) {
        match event {
            Event::NoteOn { note, velocity, .. } => {
                self.voices.note_on(*note, *velocity);
            }

            Event::NoteOff { note, .. } => {
                self.voices.note_off(*note);
            }

            Event::ParamChange {
                param_id, value, ..
            } => {
                // Parameter routing happens elsewhere
                // (param store, modulation system, etc.)
                self.graph.set_param(*param_id, *value);
            }
        }
    }
}
