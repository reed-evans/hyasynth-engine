// src/node_factory.rs

use std::collections::HashMap;

use crate::node::{Node, Polyphony};
use crate::state::{NodeTypeId, NodeTypeInfo, ParamInfo};

/// A factory capable of creating fresh node instances.
///
/// This is only used during graph construction / preparation.
pub trait NodeFactory: Send + Sync {
    /// Create one node instance
    fn create(&self) -> Box<dyn Node>;

    /// Polyphony behavior of nodes created by this factory
    fn polyphony(&self) -> Polyphony;

    /// Number of output channels this node produces
    fn num_channels(&self) -> usize;
}

/// Convenience factory for simple nodes
pub struct SimpleNodeFactory<F>
where
    F: Fn() -> Box<dyn Node> + Send + Sync,
{
    create_fn: F,
    polyphony: Polyphony,
    num_channels: usize,
}

impl<F> SimpleNodeFactory<F>
where
    F: Fn() -> Box<dyn Node> + Send + Sync,
{
    pub fn new(create_fn: F, polyphony: Polyphony) -> Self {
        Self {
            create_fn,
            polyphony,
            num_channels: 2,
        }
    }

    pub fn channels(mut self, n: usize) -> Self {
        self.num_channels = n;
        self
    }
}

impl<F> NodeFactory for SimpleNodeFactory<F>
where
    F: Fn() -> Box<dyn Node> + Send + Sync,
{
    fn create(&self) -> Box<dyn Node> {
        (self.create_fn)()
    }

    fn polyphony(&self) -> Polyphony {
        self.polyphony
    }

    fn num_channels(&self) -> usize {
        self.num_channels
    }
}

/// Registry that maps NodeTypeId to both metadata and factory.
///
/// This is the central registry used by:
/// - UI: to get metadata (NodeTypeInfo) for display
/// - Compiler: to create runtime nodes from GraphDef
pub struct NodeRegistry {
    entries: HashMap<NodeTypeId, NodeRegistryEntry>,
}

struct NodeRegistryEntry {
    info: NodeTypeInfo,
    factory: Box<dyn NodeFactory>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Register a node type with its metadata and factory.
    pub fn register<F>(&mut self, info: NodeTypeInfo, factory: F)
    where
        F: NodeFactory + 'static,
    {
        let type_id = info.type_id;
        self.entries.insert(
            type_id,
            NodeRegistryEntry {
                info,
                factory: Box::new(factory),
            },
        );
    }

    /// Get metadata for a node type (for UI).
    pub fn get_info(&self, type_id: NodeTypeId) -> Option<&NodeTypeInfo> {
        self.entries.get(&type_id).map(|e| &e.info)
    }

    /// Get the factory for a node type (for compilation).
    pub fn get_factory(&self, type_id: NodeTypeId) -> Option<&dyn NodeFactory> {
        self.entries.get(&type_id).map(|e| e.factory.as_ref())
    }

    /// Iterate over all registered node types.
    pub fn iter(&self) -> impl Iterator<Item = &NodeTypeInfo> {
        self.entries.values().map(|e| &e.info)
    }

    /// Get node types grouped by category.
    pub fn by_category(&self) -> HashMap<&str, Vec<&NodeTypeInfo>> {
        let mut map: HashMap<&str, Vec<&NodeTypeInfo>> = HashMap::new();
        for entry in self.entries.values() {
            map.entry(&entry.info.category)
                .or_default()
                .push(&entry.info);
        }
        map
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════
// Helper for building node type info
// ═══════════════════════════════════════════════════════════════════

/// Builder for creating NodeTypeInfo with common patterns.
pub struct NodeTypeBuilder {
    info: NodeTypeInfo,
}

impl NodeTypeBuilder {
    pub fn new(type_id: NodeTypeId, name: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            info: NodeTypeInfo::new(type_id, name, category),
        }
    }

    pub fn param(mut self, param: ParamInfo) -> Self {
        self.info.parameters.push(param);
        self
    }

    pub fn audio_in(mut self, id: u32, name: impl Into<String>) -> Self {
        use crate::state::{PortDirection, PortInfo, PortType};
        self.info.inputs.push(PortInfo {
            id,
            name: name.into(),
            direction: PortDirection::Input,
            port_type: PortType::Audio,
            channels: 1,
        });
        self
    }

    pub fn audio_out(mut self, id: u32, name: impl Into<String>) -> Self {
        use crate::state::{PortDirection, PortInfo, PortType};
        self.info.outputs.push(PortInfo {
            id,
            name: name.into(),
            direction: PortDirection::Output,
            port_type: PortType::Audio,
            channels: 1,
        });
        self
    }

    pub fn build(self) -> NodeTypeInfo {
        self.info
    }
}
