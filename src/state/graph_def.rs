// Declarative graph definition.
//
// This is the "document" representation of the audio graph.
// It can be serialized, edited by the UI, and compiled to a runtime Graph.

use std::collections::HashMap;

use super::ParamInfo;

/// Unique identifier for a node type (e.g., "oscillator", "filter").
pub type NodeTypeId = u32;

/// Unique identifier for a node instance within a graph.
pub type NodeId = u32;

/// Unique identifier for a port on a node.
pub type PortId = u32;

/// Port direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDirection {
    Input,
    Output,
}

/// Port signal type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortType {
    /// Audio signal (sample rate)
    Audio,
    /// Control signal (block rate or per-slice)
    Control,
}

/// Metadata describing a port on a node type.
#[derive(Debug, Clone)]
pub struct PortInfo {
    pub id: PortId,
    pub name: String,
    pub direction: PortDirection,
    pub port_type: PortType,
    pub channels: usize,
}

impl PortInfo {
    pub fn audio_input(id: PortId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            direction: PortDirection::Input,
            port_type: PortType::Audio,
            channels: 1,
        }
    }

    pub fn audio_output(id: PortId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            direction: PortDirection::Output,
            port_type: PortType::Audio,
            channels: 1,
        }
    }

    pub fn stereo(mut self) -> Self {
        self.channels = 2;
        self
    }
}

/// Metadata describing a node type.
///
/// Used by the UI to:
/// - Show available nodes in a palette
/// - Validate connections
/// - Display parameter controls
#[derive(Debug, Clone)]
pub struct NodeTypeInfo {
    pub type_id: NodeTypeId,
    pub name: String,
    pub category: String,
    pub inputs: Vec<PortInfo>,
    pub outputs: Vec<PortInfo>,
    pub parameters: Vec<ParamInfo>,
}

impl NodeTypeInfo {
    pub fn new(type_id: NodeTypeId, name: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            type_id,
            name: name.into(),
            category: category.into(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            parameters: Vec::new(),
        }
    }

    pub fn with_input(mut self, port: PortInfo) -> Self {
        self.inputs.push(port);
        self
    }

    pub fn with_output(mut self, port: PortInfo) -> Self {
        self.outputs.push(port);
        self
    }

    pub fn with_param(mut self, param: ParamInfo) -> Self {
        self.parameters.push(param);
        self
    }

    pub fn find_param(&self, id: u32) -> Option<&ParamInfo> {
        self.parameters.iter().find(|p| p.id == id)
    }
}

/// A connection between two ports.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnectionDef {
    pub source_node: NodeId,
    pub source_port: PortId,
    pub dest_node: NodeId,
    pub dest_port: PortId,
}

/// An instance of a node in the graph.
#[derive(Debug, Clone)]
pub struct NodeDef {
    /// Unique instance ID
    pub id: NodeId,

    /// Type of this node
    pub type_id: NodeTypeId,

    /// UI position (for graph editor)
    pub position: (f32, f32),

    /// Current parameter values (sparse - only non-default values)
    pub param_values: HashMap<u32, f32>,

    /// User-defined label
    pub label: Option<String>,
}

impl NodeDef {
    pub fn new(id: NodeId, type_id: NodeTypeId) -> Self {
        Self {
            id,
            type_id,
            position: (0.0, 0.0),
            param_values: HashMap::new(),
            label: None,
        }
    }

    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.position = (x, y);
        self
    }

    pub fn with_param(mut self, param_id: u32, value: f32) -> Self {
        self.param_values.insert(param_id, value);
        self
    }

    pub fn labeled(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// The complete declarative graph definition.
///
/// This is the "document" that the UI edits.
/// It gets compiled to a runtime `Graph` by the bridge.
#[derive(Debug, Clone, Default)]
pub struct GraphDef {
    /// All nodes in the graph
    pub nodes: HashMap<NodeId, NodeDef>,

    /// All connections
    pub connections: Vec<ConnectionDef>,

    /// The output node (final audio destination)
    pub output_node: Option<NodeId>,

    /// Next available node ID
    next_id: NodeId,
}

impl GraphDef {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node to the graph, returning its ID.
    pub fn add_node(&mut self, type_id: NodeTypeId) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(id, NodeDef::new(id, type_id));
        id
    }

    /// Add a pre-configured node.
    pub fn add_node_def(&mut self, mut node: NodeDef) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        node.id = id;
        self.nodes.insert(id, node);
        id
    }

    /// Remove a node and all its connections.
    pub fn remove_node(&mut self, id: NodeId) -> Option<NodeDef> {
        self.connections
            .retain(|c| c.source_node != id && c.dest_node != id);

        if self.output_node == Some(id) {
            self.output_node = None;
        }

        self.nodes.remove(&id)
    }

    /// Connect two nodes.
    pub fn connect(
        &mut self,
        source_node: NodeId,
        source_port: PortId,
        dest_node: NodeId,
        dest_port: PortId,
    ) {
        let conn = ConnectionDef {
            source_node,
            source_port,
            dest_node,
            dest_port,
        };

        if !self.connections.contains(&conn) {
            self.connections.push(conn);
        }
    }

    /// Disconnect two nodes.
    pub fn disconnect(
        &mut self,
        source_node: NodeId,
        source_port: PortId,
        dest_node: NodeId,
        dest_port: PortId,
    ) {
        self.connections.retain(|c| {
            !(c.source_node == source_node
                && c.source_port == source_port
                && c.dest_node == dest_node
                && c.dest_port == dest_port)
        });
    }

    /// Set a parameter value on a node.
    pub fn set_param(&mut self, node_id: NodeId, param_id: u32, value: f32) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.param_values.insert(param_id, value);
        }
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: NodeId) -> Option<&NodeDef> {
        self.nodes.get(&id)
    }

    /// Get a mutable node by ID.
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut NodeDef> {
        self.nodes.get_mut(&id)
    }

    /// Iterate over all nodes.
    pub fn iter_nodes(&self) -> impl Iterator<Item = &NodeDef> {
        self.nodes.values()
    }

    /// Get connections to a specific node.
    pub fn connections_to(&self, node_id: NodeId) -> impl Iterator<Item = &ConnectionDef> {
        self.connections
            .iter()
            .filter(move |c| c.dest_node == node_id)
    }

    /// Get connections from a specific node.
    pub fn connections_from(&self, node_id: NodeId) -> impl Iterator<Item = &ConnectionDef> {
        self.connections
            .iter()
            .filter(move |c| c.source_node == node_id)
    }
}

/// Registry of available node types.
///
/// The UI uses this to:
/// - Populate the node palette
/// - Validate graph structure
/// - Display parameter controls
#[derive(Debug, Default)]
pub struct NodeTypeRegistry {
    types: HashMap<NodeTypeId, NodeTypeInfo>,
}

impl NodeTypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, info: NodeTypeInfo) {
        self.types.insert(info.type_id, info);
    }

    pub fn get(&self, type_id: NodeTypeId) -> Option<&NodeTypeInfo> {
        self.types.get(&type_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &NodeTypeInfo> {
        self.types.values()
    }

    /// Get node types grouped by category.
    pub fn by_category(&self) -> HashMap<&str, Vec<&NodeTypeInfo>> {
        let mut map: HashMap<&str, Vec<&NodeTypeInfo>> = HashMap::new();
        for info in self.types.values() {
            map.entry(&info.category).or_default().push(info);
        }
        map
    }
}
