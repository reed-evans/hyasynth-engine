//! Thread-safe bridge between UI and audio engine.
//!
//! This module provides the communication layer that allows the UI thread
//! to safely interact with the real-time audio engine.
//!
//! # Architecture
//!
//! - **UI thread** owns [`SessionHandle`] with local [`Session`] state
//! - **Audio thread** owns [`EngineHandle`] with the [`Engine`]
//! - Communication uses lock-free MPSC channels for commands and atomics for readback
//!
//! # Usage
//!
//! ```ignore
//! let (session, engine) = create_bridge(Session::new("My Session"), engine);
//!
//! // UI thread: send commands
//! session.note_on(60, 0.8);
//!
//! // Audio thread: process commands and render
//! engine.process_commands();
//! engine.process_plan(&plan);
//! ```

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Receiver, Sender, TryRecvError},
};

use crate::engine::Engine;
use crate::execution_plan::ExecutionPlan;
use crate::graph::Graph;
use crate::state::{Command, CommandResult, EngineReadback, NodeId, NodeTypeId, Session};

/// Handle for the UI thread to communicate with the engine.
///
/// This is the primary interface that SwiftUI/iOS will interact with.
/// All methods are safe to call from the main thread.
pub struct SessionHandle {
    /// The current session state (owned by UI thread).
    session: Session,

    /// Channel to send commands to the engine.
    command_tx: Sender<Command>,

    /// Channel to receive command results.
    result_rx: Receiver<CommandResult>,

    /// Shared readback state (updated by engine, read by UI).
    readback: Arc<SharedReadback>,
}

/// Handle for the audio thread containing the engine and communication channels.
///
/// This is the primary interface for the audio thread. It owns the Engine
/// and provides methods to process commands and audio.
pub struct EngineHandle {
    /// The audio engine (owned by audio thread).
    engine: Engine,

    /// Channel to receive commands from UI.
    command_rx: Receiver<Command>,

    /// Channel to send results back to UI.
    result_tx: Sender<CommandResult>,

    /// Shared readback state (written by engine).
    readback: Arc<SharedReadback>,
}

/// Lock-free shared state for engine -> UI readback.
///
/// Uses atomics for frequently updated values.
struct SharedReadback {
    sample_position: AtomicU64,
    /// Beat position stored as f64 bits (no AtomicF64 in std)
    beat_position_bits: AtomicU64,
    active_voices: AtomicU64,
    running: AtomicBool,
    // Peak meters would use AtomicU32 with f32::to_bits/from_bits
}

impl SharedReadback {
    fn new() -> Self {
        Self {
            sample_position: AtomicU64::new(0),
            beat_position_bits: AtomicU64::new(0.0_f64.to_bits()),
            active_voices: AtomicU64::new(0),
            running: AtomicBool::new(false),
        }
    }
}

/// Create a linked pair of handles for UI and Engine communication.
///
/// The `engine` parameter is the audio engine that will be owned by the
/// `EngineHandle` and run on the audio thread.
pub fn create_bridge(session: Session, engine: Engine) -> (SessionHandle, EngineHandle) {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    let readback = Arc::new(SharedReadback::new());

    let session_handle = SessionHandle {
        session,
        command_tx: cmd_tx,
        result_rx,
        readback: Arc::clone(&readback),
    };

    let engine_handle = EngineHandle {
        engine,
        command_rx: cmd_rx,
        result_tx,
        readback,
    };

    (session_handle, engine_handle)
}

// ═══════════════════════════════════════════════════════════════════
// SessionHandle - UI Thread API
// ═══════════════════════════════════════════════════════════════════

impl SessionHandle {
    /// Get a reference to the current session.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Get a mutable reference to the session.
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// Send a command to the engine.
    ///
    /// Also updates local session state for immediate UI feedback.
    pub fn send(&mut self, cmd: Command) {
        // Apply to local state first (optimistic update)
        self.apply_to_session(&cmd);

        // Send to engine
        let _ = self.command_tx.send(cmd);
    }

    /// Apply a command to the local session state.
    ///
    /// This provides immediate feedback before the engine processes it.
    fn apply_to_session(&mut self, cmd: &Command) {
        match cmd {
            Command::AddNode { type_id, position } => {
                let id = self.session.graph.add_node(*type_id);
                if let Some(node) = self.session.graph.get_node_mut(id) {
                    node.position = *position;
                }
            }
            Command::AddNodeDef { node } => {
                self.session.graph.add_node_def(node.clone());
            }
            Command::RemoveNode { node_id } => {
                self.session.graph.remove_node(*node_id);
            }
            Command::Connect {
                source_node,
                source_port,
                dest_node,
                dest_port,
            } => {
                self.session
                    .graph
                    .connect(*source_node, *source_port, *dest_node, *dest_port);
            }
            Command::Disconnect {
                source_node,
                source_port,
                dest_node,
                dest_port,
            } => {
                self.session
                    .graph
                    .disconnect(*source_node, *source_port, *dest_node, *dest_port);
            }
            Command::SetOutputNode { node_id } => {
                self.session.graph.output_node = Some(*node_id);
            }
            Command::MoveNode { node_id, position } => {
                if let Some(node) = self.session.graph.get_node_mut(*node_id) {
                    node.position = *position;
                }
            }
            Command::SetParam {
                node_id,
                param_id,
                value,
            } => {
                self.session.graph.set_param(*node_id, *param_id, *value);
            }
            Command::SetTempo { bpm } => {
                self.session.transport.bpm = *bpm;
            }
            Command::Play => {
                self.session.transport.playing = true;
            }
            Command::Stop => {
                self.session.transport.playing = false;
            }
            Command::ClearGraph => {
                self.session.graph = Default::default();
            }
            // ═══════════════════════════════════════════════════════════════
            // Clip commands
            // ═══════════════════════════════════════════════════════════════
            Command::CreateClip { name, length } => {
                self.session.arrangement.create_clip(name, *length);
            }
            Command::DeleteClip { clip_id } => {
                self.session.arrangement.delete_clip(*clip_id);
            }
            Command::AddNoteToClip {
                clip_id,
                start,
                duration,
                note,
                velocity,
            } => {
                use crate::state::NoteDef;
                self.session.arrangement.add_note_to_clip(
                    *clip_id,
                    NoteDef::new(*start, *duration, *note, *velocity),
                );
            }
            Command::RemoveNoteFromClip {
                clip_id,
                note_index,
            } => {
                if let Some(clip) = self.session.arrangement.get_clip_mut(*clip_id) {
                    clip.remove_note(*note_index);
                }
            }
            Command::ClearClip { clip_id } => {
                if let Some(clip) = self.session.arrangement.get_clip_mut(*clip_id) {
                    clip.clear();
                }
            }
            Command::SetClipLength { clip_id, length } => {
                if let Some(clip) = self.session.arrangement.get_clip_mut(*clip_id) {
                    clip.length = *length;
                }
            }
            Command::SetClipLooping { clip_id, looping } => {
                if let Some(clip) = self.session.arrangement.get_clip_mut(*clip_id) {
                    clip.looping = *looping;
                }
            }

            // ═══════════════════════════════════════════════════════════════
            // Track commands
            // ═══════════════════════════════════════════════════════════════
            Command::CreateTrack { name } => {
                self.session.arrangement.create_track(name);
            }
            Command::DeleteTrack { track_id } => {
                self.session.arrangement.delete_track(*track_id);
            }
            Command::SetTrackVolume { track_id, volume } => {
                self.session.arrangement.set_track_volume(*track_id, *volume);
            }
            Command::SetTrackPan { track_id, pan } => {
                self.session.arrangement.set_track_pan(*track_id, *pan);
            }
            Command::SetTrackMute { track_id, mute } => {
                self.session.arrangement.set_track_mute(*track_id, *mute);
            }
            Command::SetTrackSolo { track_id, solo } => {
                self.session.arrangement.set_track_solo(*track_id, *solo);
            }
            Command::SetTrackArmed { track_id, armed } => {
                if let Some(track) = self.session.arrangement.get_track_mut(*track_id) {
                    track.armed = *armed;
                }
            }
            Command::SetTrackTarget { track_id, node_id } => {
                self.session
                    .arrangement
                    .set_track_target(*track_id, *node_id);
            }
            Command::SetClipSlot {
                track_id,
                scene_index,
                clip_id,
            } => {
                self.session
                    .arrangement
                    .set_clip_slot(*track_id, *scene_index, *clip_id);
            }

            // ═══════════════════════════════════════════════════════════════
            // Scene commands
            // ═══════════════════════════════════════════════════════════════
            Command::CreateScene { name } => {
                self.session.arrangement.create_scene(name);
            }
            Command::DeleteScene { scene_id } => {
                self.session.arrangement.delete_scene(*scene_id);
            }
            Command::LaunchScene { scene_index } => {
                self.session.arrangement.launch_scene(*scene_index);
            }
            Command::LaunchClip { track_id, clip_id } => {
                self.session.arrangement.launch_clip(*track_id, *clip_id);
            }
            Command::StopClip { track_id } => {
                self.session.arrangement.stop_clip(*track_id);
            }
            Command::StopAllClips => {
                self.session.arrangement.stop_all();
            }

            // ═══════════════════════════════════════════════════════════════
            // Timeline commands
            // ═══════════════════════════════════════════════════════════════
            Command::ScheduleClip {
                track_id,
                clip_id,
                start_beat,
            } => {
                self.session
                    .arrangement
                    .schedule_clip(*track_id, *clip_id, *start_beat);
            }
            Command::RemoveClipPlacement {
                track_id,
                start_beat,
            } => {
                self.session
                    .arrangement
                    .remove_clip_placement(*track_id, *start_beat);
            }

            // ═══════════════════════════════════════════════════════════════
            // Compilation commands
            // ═══════════════════════════════════════════════════════════════
            Command::RecompileGraph => {
                // Runtime graph is rebuilt on-demand via build_runtime_graph().
                // This command signals the engine to fetch the new graph.
            }
            Command::SyncTrackParams { .. } => {
                // Parameter sync is computed on-demand via sync_track_params().
            }
            Command::SyncAllTrackParams => {
                // Sync computed on-demand.
            }

            // Commands that don't affect session state directly
            Command::BeginParamGesture { .. }
            | Command::EndParamGesture { .. }
            | Command::Seek { .. }
            | Command::NoteOn { .. }
            | Command::NoteOff { .. }
            | Command::LoadConnections { .. } => {}
        }
    }

    /// Poll for any command results from the engine.
    pub fn poll_results(&self) -> Vec<CommandResult> {
        let mut results = Vec::new();
        loop {
            match self.result_rx.try_recv() {
                Ok(result) => results.push(result),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        results
    }

    /// Get the current engine readback state.
    ///
    /// Note: `cpu_load` and `output_peaks` are not yet implemented.
    /// They require additional atomic storage in SharedReadback.
    pub fn readback(&self) -> EngineReadback {
        EngineReadback {
            sample_position: self.readback.sample_position.load(Ordering::Relaxed),
            beat_position: f64::from_bits(
                self.readback.beat_position_bits.load(Ordering::Relaxed),
            ),
            cpu_load: 0.0,
            active_voices: self.readback.active_voices.load(Ordering::Relaxed) as usize,
            output_peaks: [0.0, 0.0],
            running: self.readback.running.load(Ordering::Relaxed),
        }
    }

    // ───────────────────────────────────────────────────────────────
    // Convenience methods
    // ───────────────────────────────────────────────────────────────

    /// Add a node to the graph.
    pub fn add_node(&mut self, type_id: NodeTypeId, x: f32, y: f32) -> NodeId {
        let id = self.session.graph.add_node(type_id);
        if let Some(node) = self.session.graph.get_node_mut(id) {
            node.position = (x, y);
        }
        let _ = self.command_tx.send(Command::AddNode {
            type_id,
            position: (x, y),
        });
        id
    }

    /// Remove a node from the graph.
    pub fn remove_node(&mut self, node_id: NodeId) {
        self.send(Command::RemoveNode { node_id });
    }

    /// Set a parameter value.
    pub fn set_param(&mut self, node_id: NodeId, param_id: u32, value: f32) {
        self.send(Command::SetParam {
            node_id,
            param_id,
            value,
        });
    }

    /// Start playback.
    pub fn play(&mut self) {
        self.send(Command::Play);
    }

    /// Stop playback.
    pub fn stop(&mut self) {
        self.send(Command::Stop);
    }

    /// Send a MIDI note on.
    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.send(Command::NoteOn { note, velocity });
    }

    /// Send a MIDI note off.
    pub fn note_off(&mut self, note: u8) {
        self.send(Command::NoteOff { note });
    }

    // ───────────────────────────────────────────────────────────────
    // Runtime graph methods
    // ───────────────────────────────────────────────────────────────

    /// Build the complete runtime graph.
    ///
    /// This combines user nodes (instruments, effects) with auto-generated
    /// track mixer nodes and master bus routing.
    pub fn build_runtime_graph(&self) -> crate::state::GraphDef {
        self.session.build_runtime_graph()
    }

    /// Trigger a full graph recompilation on the engine.
    ///
    /// Call this after structural changes (adding/removing tracks).
    pub fn recompile_graph(&mut self) {
        self.send(Command::RecompileGraph);
    }

    /// Get parameter updates for a specific track.
    ///
    /// Returns (node_id, param_id, value) tuples for the track's mixer nodes.
    /// Use this for real-time track volume/pan/mute updates.
    pub fn get_track_param_updates(&self, track_id: crate::state::TrackId) -> Vec<(NodeId, u32, f32)> {
        self.session.sync_track_params(track_id)
    }

    /// Sync track parameters to the engine.
    ///
    /// Sends SetParam commands for the track's mixer nodes.
    /// More efficient than full recompilation for volume/pan/mute changes.
    pub fn sync_track(&mut self, track_id: crate::state::TrackId) {
        for (node_id, param_id, value) in self.session.sync_track_params(track_id) {
            self.send(Command::SetParam {
                node_id,
                param_id,
                value,
            });
        }
    }

    /// Sync all track parameters to the engine.
    pub fn sync_all_tracks(&mut self) {
        for (node_id, param_id, value) in self.session.sync_all_track_params() {
            self.send(Command::SetParam {
                node_id,
                param_id,
                value,
            });
        }
    }

    // ───────────────────────────────────────────────────────────────
    // Track convenience methods
    // ───────────────────────────────────────────────────────────────

    /// Create a new track and trigger recompilation.
    pub fn create_track(&mut self, name: impl Into<String>) -> crate::state::TrackId {
        let name = name.into();
        let id = self.session.arrangement.create_track(&name);
        let _ = self.command_tx.send(Command::CreateTrack { name });
        // Structural change requires recompilation
        let _ = self.command_tx.send(Command::RecompileGraph);
        id
    }

    /// Delete a track and trigger recompilation.
    pub fn delete_track(&mut self, track_id: crate::state::TrackId) {
        self.send(Command::DeleteTrack { track_id });
        self.send(Command::RecompileGraph);
    }

    /// Set track volume (with automatic parameter sync).
    pub fn set_track_volume(&mut self, track_id: crate::state::TrackId, volume: f32) {
        self.send(Command::SetTrackVolume { track_id, volume });
        self.sync_track(track_id);
    }

    /// Set track pan (with automatic parameter sync).
    pub fn set_track_pan(&mut self, track_id: crate::state::TrackId, pan: f32) {
        self.send(Command::SetTrackPan { track_id, pan });
        self.sync_track(track_id);
    }

    /// Set track mute (with automatic parameter sync).
    pub fn set_track_mute(&mut self, track_id: crate::state::TrackId, mute: bool) {
        self.send(Command::SetTrackMute { track_id, mute });
        self.sync_track(track_id);
    }

    /// Set track target node (the instrument this track routes MIDI to).
    pub fn set_track_target(&mut self, track_id: crate::state::TrackId, node_id: Option<u32>) {
        self.send(Command::SetTrackTarget { track_id, node_id });
        // Routing change requires recompilation
        self.send(Command::RecompileGraph);
    }
}

// ═══════════════════════════════════════════════════════════════════
// EngineHandle - Audio Thread API
// ═══════════════════════════════════════════════════════════════════

impl EngineHandle {
    // ───────────────────────────────────────────────────────────────
    // Command Processing
    // ───────────────────────────────────────────────────────────────

    /// Process all pending commands from the UI.
    ///
    /// Call this at the start of each audio block.
    /// Returns `true` if any command requires graph recompilation.
    pub fn process_commands(&mut self) -> bool {
        let mut needs_recompile = false;

        while let Ok(cmd) = self.command_rx.try_recv() {
            needs_recompile |= !self.engine.process_command(&cmd);
        }

        needs_recompile
    }

    /// Try to receive a single command (non-blocking).
    pub fn try_recv(&self) -> Option<Command> {
        self.command_rx.try_recv().ok()
    }

    /// Send a result back to the UI.
    pub fn send_result(&self, result: CommandResult) {
        let _ = self.result_tx.send(result);
    }

    // ───────────────────────────────────────────────────────────────
    // Audio Processing (delegates to Engine)
    // ───────────────────────────────────────────────────────────────

    /// Execute a precompiled execution plan.
    ///
    /// Call this once per audio block from the audio callback.
    #[inline]
    pub fn process_plan(&mut self, plan: &ExecutionPlan) {
        self.engine.process_plan(plan);
    }

    /// Get the output buffer after processing.
    #[inline]
    pub fn output_buffer(&self, frames: usize) -> Option<&[f32]> {
        self.engine.output_buffer(frames)
    }

    /// Reset the engine (on transport stop/seek).
    pub fn reset(&mut self) {
        self.engine.reset();
    }

    // ───────────────────────────────────────────────────────────────
    // Engine State Access
    // ───────────────────────────────────────────────────────────────

    /// Check if the engine is currently playing.
    #[inline]
    pub fn is_playing(&self) -> bool {
        self.engine.is_playing()
    }

    /// Get the current tempo.
    #[inline]
    pub fn bpm(&self) -> f64 {
        self.engine.bpm()
    }

    /// Get active voice count.
    #[inline]
    pub fn active_voices(&self) -> usize {
        self.engine.active_voices()
    }

    /// Get a reference to the engine.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Get a mutable reference to the engine.
    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    // ───────────────────────────────────────────────────────────────
    // Graph Management
    // ───────────────────────────────────────────────────────────────

    /// Replace the current graph with a new one.
    ///
    /// Call this after recompiling the graph from an updated GraphDef.
    /// The new graph should already be prepared (call `graph.prepare(sample_rate)`).
    pub fn swap_graph(&mut self, new_graph: Graph) {
        self.engine.swap_graph(new_graph);
    }

    // ───────────────────────────────────────────────────────────────
    // Readback Updates (for UI synchronization)
    // ───────────────────────────────────────────────────────────────

    /// Update the sample position readback (called every block).
    pub fn update_sample_position(&self, pos: u64) {
        self.readback.sample_position.store(pos, Ordering::Relaxed);
    }

    /// Update the beat position readback (called every block).
    pub fn update_beat_position(&self, pos: f64) {
        self.readback
            .beat_position_bits
            .store(pos.to_bits(), Ordering::Relaxed);
    }

    /// Update the active voice count readback.
    pub fn update_active_voices_readback(&self, count: usize) {
        self.readback
            .active_voices
            .store(count as u64, Ordering::Relaxed);
    }

    /// Sync readback state from engine.
    ///
    /// Call this at the end of each audio block to update UI-visible state.
    pub fn sync_readback(&self) {
        self.readback
            .active_voices
            .store(self.engine.active_voices() as u64, Ordering::Relaxed);
        self.readback
            .running
            .store(self.engine.is_playing(), Ordering::Relaxed);
    }

    /// Set the running state readback.
    pub fn set_running(&self, running: bool) {
        self.readback.running.store(running, Ordering::Relaxed);
    }
}
