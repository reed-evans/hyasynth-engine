// src/compile.rs
//
// Compiles GraphDef (declarative) into Graph (runtime).
//
// This is the bridge between the UI's declarative representation
// and the engine's executable audio graph.

use std::collections::HashMap;

use crate::graph::Graph;
use crate::node_factory::NodeRegistry;
use crate::state::{GraphDef, NodeId};

/// Error during graph compilation.
#[derive(Debug)]
pub enum CompileError {
    /// A node references an unknown type.
    UnknownNodeType { node_id: NodeId, type_id: u32 },

    /// A connection references a non-existent node.
    InvalidConnection { source: NodeId, dest: NodeId },
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::UnknownNodeType { node_id, type_id } => {
                write!(f, "Node {} has unknown type {}", node_id, type_id)
            }
            CompileError::InvalidConnection { source, dest } => {
                write!(f, "Invalid connection from {} to {}", source, dest)
            }
        }
    }
}

impl std::error::Error for CompileError {}

/// Result of graph compilation.
pub type CompileResult<T> = Result<T, CompileError>;

/// Compile a GraphDef into a runtime Graph.
///
/// This function:
/// 1. Creates node instances using the registry's factories
/// 2. Applies parameter values from the definition
/// 3. Wires up connections
/// 4. Sets the output node
///
/// The returned Graph is ready to be prepared and processed.
pub fn compile(
    def: &GraphDef,
    registry: &NodeRegistry,
    max_block: usize,
    max_voices: usize,
) -> CompileResult<Graph> {
    let mut graph = Graph::new(max_block, max_voices);

    // Map from NodeDef ID -> runtime Graph index
    let mut id_to_index: HashMap<NodeId, usize> = HashMap::new();

    // Sort nodes by ID for deterministic ordering
    let mut node_ids: Vec<NodeId> = def.nodes.keys().copied().collect();
    node_ids.sort();

    // Create all nodes
    for &node_id in &node_ids {
        let node_def = def.nodes.get(&node_id).unwrap();

        let factory = registry
            .get_factory(node_def.type_id)
            .ok_or(CompileError::UnknownNodeType {
                node_id,
                type_id: node_def.type_id,
            })?;

        let idx = graph.add_node(factory);
        id_to_index.insert(node_id, idx);

        // Apply parameter values
        for (&param_id, &value) in &node_def.param_values {
            graph.set_param(idx, param_id, value);
        }
    }

    // Wire up connections
    // Note: Current Graph only tracks node->node, not port->port
    // We deduplicate connections to the same dest node
    let mut connected: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for conn in &def.connections {
        let sources = connected.entry(conn.dest_node).or_default();
        if !sources.contains(&conn.source_node) {
            sources.push(conn.source_node);

            let src_idx = id_to_index.get(&conn.source_node).ok_or(
                CompileError::InvalidConnection {
                    source: conn.source_node,
                    dest: conn.dest_node,
                },
            )?;

            let dst_idx =
                id_to_index
                    .get(&conn.dest_node)
                    .ok_or(CompileError::InvalidConnection {
                        source: conn.source_node,
                        dest: conn.dest_node,
                    })?;

            graph.connect(*src_idx, *dst_idx);
        }
    }

    // Set output node
    if let Some(output_id) = def.output_node {
        if let Some(&output_idx) = id_to_index.get(&output_id) {
            graph.output_node = output_idx;
        }
    } else if !node_ids.is_empty() {
        // Default to last node if no output specified
        graph.output_node = graph.nodes.len() - 1;
    }

    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_buffer::AudioBuffer;
    use crate::node::{Node, Polyphony, ProcessContext};
    use crate::node_factory::SimpleNodeFactory;
    use crate::state::NodeTypeInfo;

    // Test node that just outputs silence
    struct TestNode;

    impl Node for TestNode {
        fn prepare(&mut self, _: f64, _: usize) {}
        
        fn process(
            &mut self,
            _ctx: &ProcessContext,
            _inputs: &[&AudioBuffer],
            _output: &mut AudioBuffer,
        ) -> bool {
            true
        }
        
        fn num_channels(&self) -> usize {
            1
        }
        
        fn set_param(&mut self, _: u32, _: f32) {}
    }

    #[test]
    fn test_compile_empty_graph() {
        let def = GraphDef::new();
        let registry = NodeRegistry::new();

        let result = compile(&def, &registry, 512, 8);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_simple_graph() {
        const TEST_NODE: u32 = 1;

        let mut def = GraphDef::new();
        let osc = def.add_node(TEST_NODE);
        let out = def.add_node(TEST_NODE);
        def.connect(osc, 0, out, 0);
        def.output_node = Some(out);

        let mut registry = NodeRegistry::new();
        registry.register(
            NodeTypeInfo::new(TEST_NODE, "Test", "Test"),
            SimpleNodeFactory::new(|| Box::new(TestNode), Polyphony::Global),
        );

        let result = compile(&def, &registry, 512, 8);
        assert!(result.is_ok());

        let graph = result.unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.nodes[1].inputs.len(), 1);
    }
}
