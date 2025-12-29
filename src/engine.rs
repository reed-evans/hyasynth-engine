// src/engine.rs

use crate::event::Event;
use crate::execution_plan::{ExecutionPlan, SlicePlan};
use crate::graph::Graph;
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
    
    /// Current sample position
    sample_pos: u64,
}

impl Engine {
    pub fn new(graph: Graph, voices: VoiceAllocator) -> Self {
        Self {
            graph,
            voices,
            sample_pos: 0,
        }
    }

    /// Execute a precompiled execution plan.
    ///
    /// Called once per audio block from the audio callback.
    /// It must not allocate or block.
    pub fn process_plan(&mut self, plan: &ExecutionPlan) {
        self.sample_pos = plan.block_start_sample;
        
        // Clear one-shot voice triggers at block start
        self.voices.clear_triggers();

        for slice in &plan.slices {
            self.process_slice(slice, plan);
        }
    }

    /// Execute one slice of time.
    #[inline(always)]
    fn process_slice(&mut self, slice: &SlicePlan, plan: &ExecutionPlan) {
        // Apply events at slice boundary
        for event in &slice.events {
            self.apply_event(event);
        }

        // Process the graph for this slice
        let slice_start = self.sample_pos + slice.frame_offset as u64;
        self.graph.process(
            slice.frame_count,
            slice_start,
            plan.bpm,
            &self.voices,
        );
    }

    /// Apply a musical event immediately.
    #[inline]
    fn apply_event(&mut self, event: &Event) {
        match event {
            Event::NoteOn { note, velocity } => {
                self.voices.note_on(*note, *velocity);
            }

            Event::NoteOff { note } => {
                self.voices.note_off(*note);
            }

            Event::ParamChange { node_id, param_id, value } => {
                self.graph.set_param(*node_id as usize, *param_id, *value);
            }
        }
    }
    
    /// Reset the engine (on transport stop/seek)
    pub fn reset(&mut self) {
        self.graph.reset();
    }
    
    /// Get the output buffer after processing
    pub fn output_buffer(&self, frames: usize) -> Option<&[f32]> {
        self.graph.output_buffer(frames)
    }
    
    /// Get active voice count
    pub fn active_voices(&self) -> usize {
        self.voices.active_count()
    }
}

