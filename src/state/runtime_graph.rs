// src/state/runtime_graph.rs
//
// Runtime graph compilation.
//
// This module handles the transformation from the declarative state
// (GraphDef + Arrangement) to a complete runtime graph that the engine
// can execute.
//
// Key concepts:
// - User nodes (instruments, effects) come from GraphDef
// - Track mixer nodes are auto-generated from Arrangement
// - Master bus node receives all track outputs
// - Node IDs are partitioned to avoid collisions

use super::{Arrangement, ConnectionDef, GraphDef, NodeDef, NodeId, Session, TrackId};
use crate::nodes::{node_types, params};

// ═══════════════════════════════════════════════════════════════════════════
// Reserved Node ID Ranges
// ═══════════════════════════════════════════════════════════════════════════

/// User-created nodes use IDs 0 - 0x0FFF_FFFF (268 million nodes).
pub const USER_NODE_MAX: NodeId = 0x0FFF_FFFF;

/// Track mixer nodes: 0x1000_0000 + (track_id * 16) + offset
/// This allows 16 nodes per track (volume, pan, effects sends, etc.)
pub const TRACK_NODE_BASE: NodeId = 0x1000_0000;
pub const TRACK_NODE_STRIDE: NodeId = 16;

/// Track node offsets within a track's range.
pub const TRACK_VOLUME_OFFSET: NodeId = 0;
pub const TRACK_PAN_OFFSET: NodeId = 1;
pub const TRACK_MUTE_OFFSET: NodeId = 2; // Reserved for future mute node

/// Master bus node ID.
pub const MASTER_BUS_ID: NodeId = 0x2000_0000;

/// Master output node ID.
pub const MASTER_OUTPUT_ID: NodeId = 0x2000_0001;

// ═══════════════════════════════════════════════════════════════════════════
// Node ID Helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Get the volume node ID for a track.
pub fn track_volume_node(track_id: TrackId) -> NodeId {
    TRACK_NODE_BASE + (track_id * TRACK_NODE_STRIDE) + TRACK_VOLUME_OFFSET
}

/// Get the pan node ID for a track.
pub fn track_pan_node(track_id: TrackId) -> NodeId {
    TRACK_NODE_BASE + (track_id * TRACK_NODE_STRIDE) + TRACK_PAN_OFFSET
}

/// Check if a node ID is in the user range.
pub fn is_user_node(id: NodeId) -> bool {
    id <= USER_NODE_MAX
}

/// Check if a node ID is a track mixer node.
pub fn is_track_node(id: NodeId) -> bool {
    id >= TRACK_NODE_BASE && id < MASTER_BUS_ID
}

/// Extract track ID from a track node ID.
pub fn track_id_from_node(id: NodeId) -> Option<TrackId> {
    if is_track_node(id) {
        Some((id - TRACK_NODE_BASE) / TRACK_NODE_STRIDE)
    } else {
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Runtime Graph Builder
// ═══════════════════════════════════════════════════════════════════════════

impl Session {
    /// Build the complete runtime graph.
    ///
    /// This combines:
    /// 1. User-created nodes from `self.graph` (instruments, effects)
    /// 2. Auto-generated track mixer nodes from `self.arrangement`
    /// 3. Master bus and output routing
    ///
    /// The resulting graph is ready for compilation to the runtime engine.
    pub fn build_runtime_graph(&self) -> GraphDef {
        let mut graph = self.graph.clone();

        // Add master bus (sums all track outputs)
        graph.nodes.insert(
            MASTER_BUS_ID,
            NodeDef::new(MASTER_BUS_ID, node_types::MIXER)
                .at(800.0, 300.0)
                .labeled("Master Bus"),
        );

        // Add master output
        graph.nodes.insert(
            MASTER_OUTPUT_ID,
            NodeDef::new(MASTER_OUTPUT_ID, node_types::OUTPUT)
                .at(1000.0, 300.0)
                .labeled("Output"),
        );

        // Connect master bus to output
        graph.connections.push(ConnectionDef {
            source_node: MASTER_BUS_ID,
            source_port: 0,
            dest_node: MASTER_OUTPUT_ID,
            dest_port: 0,
        });

        // Set the output node
        graph.output_node = Some(MASTER_OUTPUT_ID);

        // Add mixer chain for each track
        self.build_track_mixers(&mut graph);

        graph
    }

    /// Build mixer nodes for all tracks.
    fn build_track_mixers(&self, graph: &mut GraphDef) {
        for track in &self.arrangement.tracks {
            self.build_track_mixer(graph, track.id);
        }
    }

    /// Build the mixer chain for a single track.
    ///
    /// Chain: [instrument] -> Volume -> Pan -> [Master Bus]
    fn build_track_mixer(&self, graph: &mut GraphDef, track_id: TrackId) {
        let track = match self.arrangement.get_track(track_id) {
            Some(t) => t,
            None => return,
        };

        let volume_id = track_volume_node(track_id);
        let pan_id = track_pan_node(track_id);

        // Calculate effective gain (includes mute state)
        let effective_volume = if track.mute { 0.0 } else { track.volume };

        // Create volume (gain) node
        graph.nodes.insert(
            volume_id,
            NodeDef::new(volume_id, node_types::GAIN)
                .at(400.0, 100.0 + (track_id as f32 * 80.0))
                .with_param(params::GAIN, effective_volume)
                .labeled(format!("{} Vol", track.name)),
        );

        // Create pan node
        graph.nodes.insert(
            pan_id,
            NodeDef::new(pan_id, node_types::PAN)
                .at(550.0, 100.0 + (track_id as f32 * 80.0))
                .with_param(params::PAN, track.pan)
                .labeled(format!("{} Pan", track.name)),
        );

        // Wire: Volume -> Pan
        graph.connections.push(ConnectionDef {
            source_node: volume_id,
            source_port: 0,
            dest_node: pan_id,
            dest_port: 0,
        });

        // Wire: Pan -> Master Bus
        graph.connections.push(ConnectionDef {
            source_node: pan_id,
            source_port: 0,
            dest_node: MASTER_BUS_ID,
            dest_port: track_id, // Each track feeds a different input
        });

        // Wire: Instrument -> Volume (if track has a target node)
        if let Some(target_node) = track.target_node {
            graph.connections.push(ConnectionDef {
                source_node: target_node,
                source_port: 0,
                dest_node: volume_id,
                dest_port: 0,
            });
        }
    }

    /// Update track mixer parameters in an existing runtime graph.
    ///
    /// Call this when track properties change to avoid full recompilation.
    /// Returns parameter change commands that should be sent to the engine.
    pub fn sync_track_params(&self, track_id: TrackId) -> Vec<(NodeId, u32, f32)> {
        let mut changes = Vec::new();

        if let Some(track) = self.arrangement.get_track(track_id) {
            let volume_id = track_volume_node(track_id);
            let pan_id = track_pan_node(track_id);

            // Volume (incorporating mute state)
            let effective_volume = if track.mute { 0.0 } else { track.volume };
            changes.push((volume_id, params::GAIN, effective_volume));

            // Pan
            changes.push((pan_id, params::PAN, track.pan));
        }

        changes
    }

    /// Get all parameter changes needed to sync all tracks.
    pub fn sync_all_track_params(&self) -> Vec<(NodeId, u32, f32)> {
        self.arrangement
            .tracks
            .iter()
            .flat_map(|t| self.sync_track_params(t.id))
            .collect()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Arrangement Helpers
// ═══════════════════════════════════════════════════════════════════════════

impl Arrangement {
    /// Check if a track should be audible considering solo state.
    ///
    /// When any track is soloed:
    /// - Soloed tracks are audible (unless muted)
    /// - Non-soloed tracks are silent
    ///
    /// This is used by `sync_track_params` to calculate effective volume.
    pub fn effective_volume(&self, track_id: TrackId) -> f32 {
        if let Some(track) = self.get_track(track_id) {
            if track.mute {
                return 0.0;
            }

            if self.has_solo() && !track.solo {
                return 0.0;
            }

            track.volume
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_node_ids() {
        assert_eq!(track_volume_node(0), TRACK_NODE_BASE);
        assert_eq!(track_pan_node(0), TRACK_NODE_BASE + 1);
        assert_eq!(track_volume_node(1), TRACK_NODE_BASE + 16);
        assert_eq!(track_pan_node(1), TRACK_NODE_BASE + 17);
    }

    #[test]
    fn test_track_id_extraction() {
        assert_eq!(track_id_from_node(track_volume_node(5)), Some(5));
        assert_eq!(track_id_from_node(track_pan_node(5)), Some(5));
        assert_eq!(track_id_from_node(0), None);
        assert_eq!(track_id_from_node(MASTER_BUS_ID), None);
    }

    #[test]
    fn test_build_runtime_graph() {
        let mut session = Session::new("Test");

        // Create some tracks
        session.arrangement.create_track("Track 1");
        session.arrangement.create_track("Track 2");

        let graph = session.build_runtime_graph();

        // Should have master bus and output
        assert!(graph.nodes.contains_key(&MASTER_BUS_ID));
        assert!(graph.nodes.contains_key(&MASTER_OUTPUT_ID));

        // Should have mixer nodes for each track
        assert!(graph.nodes.contains_key(&track_volume_node(0)));
        assert!(graph.nodes.contains_key(&track_pan_node(0)));
        assert!(graph.nodes.contains_key(&track_volume_node(1)));
        assert!(graph.nodes.contains_key(&track_pan_node(1)));
    }
}

