// src/bridge.rs
//
// Thread-safe bridge between UI and Engine.
//
// This module provides the communication layer that allows the UI thread
// to safely interact with the real-time audio engine.
//
// Architecture:
// - UI thread owns the Session (declarative state)
// - Engine thread owns the Engine (runtime state)
// - Bridge coordinates between them using lock-free queues

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Receiver, Sender, TryRecvError},
    Arc,
};

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

/// Handle for the audio thread to receive commands.
///
/// This lives on the engine side and processes incoming commands.
pub struct EngineHandle {
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
    active_voices: AtomicU64,
    running: AtomicBool,
    // Peak meters would use AtomicU32 with f32::to_bits/from_bits
}

impl SharedReadback {
    fn new() -> Self {
        Self {
            sample_position: AtomicU64::new(0),
            active_voices: AtomicU64::new(0),
            running: AtomicBool::new(false),
        }
    }
}

/// Create a linked pair of handles for UI and Engine communication.
pub fn create_bridge(session: Session) -> (SessionHandle, EngineHandle) {
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
                self.session.graph.connect(
                    *source_node,
                    *source_port,
                    *dest_node,
                    *dest_port,
                );
            }
            Command::Disconnect {
                source_node,
                source_port,
                dest_node,
                dest_port,
            } => {
                self.session.graph.disconnect(
                    *source_node,
                    *source_port,
                    *dest_node,
                    *dest_port,
                );
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
            // Commands that don't affect session state
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
    pub fn readback(&self) -> EngineReadback {
        EngineReadback {
            sample_position: self.readback.sample_position.load(Ordering::Relaxed),
            beat_position: 0.0, // TODO: compute from sample position
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
}

// ═══════════════════════════════════════════════════════════════════
// EngineHandle - Audio Thread API
// ═══════════════════════════════════════════════════════════════════

impl EngineHandle {
    /// Try to receive a command (non-blocking).
    ///
    /// Call this at the start of each audio block.
    pub fn try_recv(&self) -> Option<Command> {
        self.command_rx.try_recv().ok()
    }

    /// Process all pending commands.
    ///
    /// Returns an iterator over all queued commands.
    pub fn drain_commands(&self) -> impl Iterator<Item = Command> + '_ {
        std::iter::from_fn(|| self.try_recv())
    }

    /// Send a result back to the UI.
    pub fn send_result(&self, result: CommandResult) {
        let _ = self.result_tx.send(result);
    }

    /// Update the sample position (called every block).
    pub fn update_sample_position(&self, pos: u64) {
        self.readback.sample_position.store(pos, Ordering::Relaxed);
    }

    /// Update the active voice count.
    pub fn update_active_voices(&self, count: usize) {
        self.readback
            .active_voices
            .store(count as u64, Ordering::Relaxed);
    }

    /// Set the running state.
    pub fn set_running(&self, running: bool) {
        self.readback.running.store(running, Ordering::Relaxed);
    }
}

