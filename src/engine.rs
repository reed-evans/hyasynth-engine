//! Real-time audio engine core.
//!
//! The engine executes precompiled execution plans, processes DSP graphs,
//! and handles commands from the UI thread. All operations are designed to be
//! deterministic, allocation-free, and lock-free for real-time safety.

use crate::event::Event;
use crate::execution_plan::{ExecutionPlan, SlicePlan};
use crate::graph::Graph;
use crate::state::Command;
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

    /// Whether playback is active
    playing: bool,

    /// Current tempo in BPM
    bpm: f64,
}

impl Engine {
    pub fn new(graph: Graph, voices: VoiceAllocator) -> Self {
        Self {
            graph,
            voices,
            sample_pos: 0,
            playing: false,
            bpm: 120.0,
        }
    }

    /// Check if the engine is currently playing.
    #[inline]
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Get the current tempo.
    #[inline]
    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Execute a precompiled execution plan.
    ///
    /// Called once per audio block from the audio callback.
    /// It must not allocate or block.
    pub fn process_plan(&mut self, plan: &ExecutionPlan) {
        self.sample_pos = plan.block_start_sample;

        for slice in &plan.slices {
            self.process_slice(slice, plan);
        }

        // Clear one-shot voice triggers at block end, after processing.
        // This ensures triggers set by process_commands() are visible during processing.
        self.voices.clear_triggers();
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
        self.graph
            .process(slice.frame_count, slice_start, plan.bpm, &self.voices);
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

            Event::NoteOnTarget {
                node_id,
                note,
                velocity,
            } => {
                // For targeted notes, we need to route to a specific node.
                // For now, we broadcast but the node_id could be used for
                // voice allocation per-instrument in the future.
                let _ = node_id;
                self.voices.note_on(*note, *velocity);
            }

            Event::NoteOffTarget { node_id, note } => {
                let _ = node_id;
                self.voices.note_off(*note);
            }

            Event::ParamChange {
                node_id,
                param_id,
                value,
            } => {
                self.graph.set_param_by_id(*node_id, *param_id, *value);
            }

            Event::AudioStart {
                node_id,
                audio_id,
                start_sample,
                duration_samples,
                gain,
            } => {
                self.graph.start_audio_by_id(
                    *node_id,
                    *audio_id,
                    *start_sample,
                    *duration_samples,
                    *gain,
                );
            }

            Event::AudioStop { node_id, audio_id } => {
                self.graph.stop_audio_by_id(*node_id, *audio_id);
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

    // ═══════════════════════════════════════════════════════════════════
    // Command Processing
    // ═══════════════════════════════════════════════════════════════════

    /// Process a command from the UI.
    ///
    /// Returns `true` if the command was fully handled, `false` if it requires
    /// additional action (like graph recompilation) that cannot be done on the
    /// audio thread.
    ///
    /// Real-time safe for most commands. Graph structure changes return `false`
    /// and must be handled by recompiling the graph on a non-RT thread.
    pub fn process_command(&mut self, cmd: &Command) -> bool {
        match cmd {
            // ═══════════════════════════════════════════════════════════
            // Parameter changes - RT safe
            // ═══════════════════════════════════════════════════════════
            Command::SetParam {
                node_id,
                param_id,
                value,
            } => {
                self.graph.set_param_by_id(*node_id, *param_id, *value);
                true
            }

            Command::BeginParamGesture { .. } | Command::EndParamGesture { .. } => {
                // Gestures are for automation recording, not RT processing
                true
            }

            // ═══════════════════════════════════════════════════════════
            // Transport - RT safe
            // ═══════════════════════════════════════════════════════════
            Command::Play => {
                self.playing = true;
                true
            }

            Command::Stop => {
                self.playing = false;
                self.reset();
                true
            }

            Command::SetTempo { bpm } => {
                self.bpm = *bpm;
                true
            }

            Command::Seek { beat: _ } => {
                // Seek requires coordination with the scheduler.
                // Reset the engine state; the scheduler handles position.
                self.reset();
                true
            }

            // ═══════════════════════════════════════════════════════════
            // MIDI - RT safe
            // ═══════════════════════════════════════════════════════════
            Command::NoteOn { note, velocity } => {
                self.voices.note_on(*note, *velocity);
                true
            }

            Command::NoteOff { note } => {
                self.voices.note_off(*note);
                true
            }

            // ═══════════════════════════════════════════════════════════
            // Graph structure - NOT RT safe, requires recompilation
            // ═══════════════════════════════════════════════════════════
            Command::AddNode { .. }
            | Command::AddNodeDef { .. }
            | Command::RemoveNode { .. }
            | Command::Connect { .. }
            | Command::Disconnect { .. }
            | Command::SetOutputNode { .. }
            | Command::ClearGraph
            | Command::LoadConnections { .. }
            | Command::RecompileGraph => {
                // These commands modify graph structure.
                // The caller must recompile the graph from the updated GraphDef
                // and swap in the new graph.
                false
            }

            // ═══════════════════════════════════════════════════════════
            // Session-only commands - no engine action needed
            // ═══════════════════════════════════════════════════════════
            Command::MoveNode { .. } => {
                // UI position only, doesn't affect audio
                true
            }

            // Clip commands - handled by session state
            Command::CreateClip { .. }
            | Command::DeleteClip { .. }
            | Command::AddNoteToClip { .. }
            | Command::RemoveNoteFromClip { .. }
            | Command::ClearClip { .. }
            | Command::SetClipLength { .. }
            | Command::SetClipLooping { .. } => true,

            // Track commands - handled by session state
            Command::CreateTrack { .. }
            | Command::DeleteTrack { .. }
            | Command::SetTrackVolume { .. }
            | Command::SetTrackPan { .. }
            | Command::SetTrackMute { .. }
            | Command::SetTrackSolo { .. }
            | Command::SetTrackArmed { .. }
            | Command::SetTrackTarget { .. }
            | Command::SetClipSlot { .. } => true,

            // Scene commands - handled by session state
            Command::CreateScene { .. }
            | Command::DeleteScene { .. }
            | Command::LaunchScene { .. }
            | Command::LaunchClip { .. }
            | Command::StopClip { .. }
            | Command::StopAllClips => true,

            // Timeline commands - handled by session state
            Command::ScheduleClip { .. } | Command::RemoveClipPlacement { .. } => true,

            // Compilation commands - sync handled elsewhere
            Command::SyncTrackParams { .. } | Command::SyncAllTrackParams => true,
        }
    }

    /// Replace the current graph with a new one.
    ///
    /// Call this after recompiling the graph from an updated GraphDef.
    /// The new graph should already be prepared (call `graph.prepare(sample_rate)`).
    pub fn swap_graph(&mut self, new_graph: Graph) {
        self.graph = new_graph;
    }

    /// Get a reference to the current graph.
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Get a mutable reference to the current graph.
    pub fn graph_mut(&mut self) -> &mut Graph {
        &mut self.graph
    }
}
