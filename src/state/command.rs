// Commands from UI to Engine.
//
// Commands are the ONLY way the UI can mutate engine state.
// They are queued and processed on the appropriate thread.

use super::{ClipId, ConnectionDef, NodeDef, NodeId, NodeTypeId, PortId, SceneId, TrackId};

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

    // ═══════════════════════════════════════════
    // Clips
    // ═══════════════════════════════════════════
    /// Create a new clip.
    CreateClip { name: String, length: f64 },

    /// Delete a clip.
    DeleteClip { clip_id: ClipId },

    /// Add a note to a clip.
    AddNoteToClip {
        clip_id: ClipId,
        start: f64,
        duration: f64,
        note: u8,
        velocity: f32,
    },

    /// Remove a note from a clip.
    RemoveNoteFromClip { clip_id: ClipId, note_index: usize },

    /// Clear all notes from a clip.
    ClearClip { clip_id: ClipId },

    /// Set clip length.
    SetClipLength { clip_id: ClipId, length: f64 },

    /// Set clip looping.
    SetClipLooping { clip_id: ClipId, looping: bool },

    // ═══════════════════════════════════════════
    // Tracks
    // ═══════════════════════════════════════════
    /// Create a new track.
    CreateTrack { name: String },

    /// Delete a track.
    DeleteTrack { track_id: TrackId },

    /// Set track volume.
    SetTrackVolume { track_id: TrackId, volume: f32 },

    /// Set track pan.
    SetTrackPan { track_id: TrackId, pan: f32 },

    /// Set track mute.
    SetTrackMute { track_id: TrackId, mute: bool },

    /// Set track solo.
    SetTrackSolo { track_id: TrackId, solo: bool },

    /// Set track armed for recording.
    SetTrackArmed { track_id: TrackId, armed: bool },

    /// Set track target node.
    SetTrackTarget { track_id: TrackId, node_id: Option<u32> },

    /// Assign a clip to a track's clip slot.
    SetClipSlot {
        track_id: TrackId,
        scene_index: usize,
        clip_id: Option<ClipId>,
    },

    // ═══════════════════════════════════════════
    // Scenes
    // ═══════════════════════════════════════════
    /// Create a new scene.
    CreateScene { name: String },

    /// Delete a scene.
    DeleteScene { scene_id: SceneId },

    /// Launch a scene (trigger all clips in row).
    LaunchScene { scene_index: usize },

    /// Launch a single clip on a track.
    LaunchClip { track_id: TrackId, clip_id: ClipId },

    /// Stop a clip on a track.
    StopClip { track_id: TrackId },

    /// Stop all clips.
    StopAllClips,

    // ═══════════════════════════════════════════
    // Timeline
    // ═══════════════════════════════════════════
    /// Schedule a clip on the timeline.
    ScheduleClip {
        track_id: TrackId,
        clip_id: ClipId,
        start_beat: f64,
    },

    /// Remove a clip placement from the timeline.
    RemoveClipPlacement { track_id: TrackId, start_beat: f64 },

    // ═══════════════════════════════════════════
    // Compilation
    // ═══════════════════════════════════════════
    /// Trigger recompilation of the runtime graph.
    ///
    /// This rebuilds the complete graph from GraphDef + Arrangement,
    /// including auto-generated track mixer nodes and master bus.
    RecompileGraph,

    /// Sync track mixer parameters without full recompilation.
    ///
    /// Use this for frequent updates like volume/pan changes.
    SyncTrackParams { track_id: TrackId },

    /// Sync all track parameters.
    SyncAllTrackParams,
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
