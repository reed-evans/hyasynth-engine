// src/state/command.rs
//
// Commands from UI to Engine.
//
// Commands are the ONLY way the UI can mutate engine state.
// They are queued and processed on the appropriate thread.

use super::{ConnectionDef, NodeDef, NodeId, NodeTypeId, PortId};

/// A command from the UI to the engine.
///
/// Commands are:
/// - Immutable once created
/// - Processed asynchronously by the engine bridge
/// - Applied atomically (all-or-nothing)
#[derive(Debug, Clone)]
pub enum Command {
    // ═══════════════════════════════════════════
    // Graph mutations
    // ═══════════════════════════════════════════
    /// Add a new node to the graph.
    AddNode {
        type_id: NodeTypeId,
        position: (f32, f32),
    },

    /// Add a pre-configured node.
    AddNodeDef { node: NodeDef },

    /// Remove a node and its connections.
    RemoveNode { node_id: NodeId },

    /// Connect two ports.
    Connect {
        source_node: NodeId,
        source_port: PortId,
        dest_node: NodeId,
        dest_port: PortId,
    },

    /// Disconnect two ports.
    Disconnect {
        source_node: NodeId,
        source_port: PortId,
        dest_node: NodeId,
        dest_port: PortId,
    },

    /// Set the graph output node.
    SetOutputNode { node_id: NodeId },

    /// Move a node in the UI.
    MoveNode {
        node_id: NodeId,
        position: (f32, f32),
    },

    // ═══════════════════════════════════════════
    // Parameter changes
    // ═══════════════════════════════════════════
    /// Set a parameter value.
    SetParam {
        node_id: NodeId,
        param_id: u32,
        value: f32,
    },

    /// Begin a parameter gesture (for automation recording).
    BeginParamGesture { node_id: NodeId, param_id: u32 },

    /// End a parameter gesture.
    EndParamGesture { node_id: NodeId, param_id: u32 },

    // ═══════════════════════════════════════════
    // Transport
    // ═══════════════════════════════════════════
    /// Start playback.
    Play,

    /// Stop playback.
    Stop,

    /// Set tempo in BPM.
    SetTempo { bpm: f64 },

    /// Seek to a position in beats.
    Seek { beat: f64 },

    // ═══════════════════════════════════════════
    // MIDI
    // ═══════════════════════════════════════════
    /// MIDI note on.
    NoteOn { note: u8, velocity: f32 },

    /// MIDI note off.
    NoteOff { note: u8 },

    // ═══════════════════════════════════════════
    // Session
    // ═══════════════════════════════════════════
    /// Reset the graph to empty.
    ClearGraph,

    /// Apply a batch of connections (used for loading).
    LoadConnections { connections: Vec<ConnectionDef> },
}

/// Response from the engine after processing a command.
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Command succeeded.
    Ok,

    /// Command succeeded and created a node.
    NodeCreated { node_id: NodeId },

    /// Command failed.
    Error { message: String },
}

/// Batch of commands to apply atomically.
#[derive(Debug, Clone, Default)]
pub struct CommandBatch {
    pub commands: Vec<Command>,
}

impl CommandBatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, cmd: Command) {
        self.commands.push(cmd);
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl FromIterator<Command> for CommandBatch {
    fn from_iter<T: IntoIterator<Item = Command>>(iter: T) -> Self {
        Self {
            commands: iter.into_iter().collect(),
        }
    }
}

