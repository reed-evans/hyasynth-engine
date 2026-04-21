// Modulation matrix routing table.
//
// Stores modulation routes that map modulator outputs to node parameters.
// These are resolved at compile time into runtime modulation state that
// applies per-sample parameter modulation during graph processing.

use super::{NodeId, PortId};

/// Unique identifier for a modulation route.
pub type ModRouteId = u32;

/// A modulation route: source output -> destination parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct ModRoute {
    /// Unique route ID.
    pub id: ModRouteId,

    /// Source modulator node.
    pub source_node: NodeId,

    /// Source output port (usually 0).
    pub source_port: PortId,

    /// Destination node to modulate.
    pub dest_node: NodeId,

    /// Destination parameter ID on the target node.
    pub dest_param: u32,

    /// Modulation depth (-1.0 to 1.0).
    /// Scales the modulator signal before adding to the base parameter value.
    pub depth: f32,
}

/// The modulation routing table.
#[derive(Debug, Clone, Default)]
pub struct ModMatrix {
    /// All modulation routes.
    pub routes: Vec<ModRoute>,

    /// Next available route ID.
    next_id: ModRouteId,
}

impl ModMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a modulation route. Returns the route ID.
    pub fn add_route(
        &mut self,
        source_node: NodeId,
        source_port: PortId,
        dest_node: NodeId,
        dest_param: u32,
        depth: f32,
    ) -> ModRouteId {
        let id = self.next_id;
        self.next_id += 1;
        self.routes.push(ModRoute {
            id,
            source_node,
            source_port,
            dest_node,
            dest_param,
            depth,
        });
        id
    }

    /// Remove a modulation route by ID.
    pub fn remove_route(&mut self, route_id: ModRouteId) -> bool {
        let len = self.routes.len();
        self.routes.retain(|r| r.id != route_id);
        self.routes.len() < len
    }

    /// Update the depth of a modulation route.
    pub fn set_depth(&mut self, route_id: ModRouteId, depth: f32) {
        if let Some(route) = self.routes.iter_mut().find(|r| r.id == route_id) {
            route.depth = depth;
        }
    }

    /// Remove all routes referencing a given node (as source or dest).
    pub fn remove_routes_for_node(&mut self, node_id: NodeId) {
        self.routes
            .retain(|r| r.source_node != node_id && r.dest_node != node_id);
    }

    /// Get all routes targeting a specific node and parameter.
    pub fn routes_for_param(
        &self,
        dest_node: NodeId,
        dest_param: u32,
    ) -> impl Iterator<Item = &ModRoute> {
        self.routes
            .iter()
            .filter(move |r| r.dest_node == dest_node && r.dest_param == dest_param)
    }

    /// Get all routes sourced from a specific node.
    pub fn routes_from(&self, source_node: NodeId) -> impl Iterator<Item = &ModRoute> {
        self.routes
            .iter()
            .filter(move |r| r.source_node == source_node)
    }
}
