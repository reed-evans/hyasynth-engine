// src/lib.rs
//
// Library entry point for FFI consumers (iOS/Swift).

mod audio_buffer;
mod bridge;
mod compile;
mod engine;
mod event;
mod execution_plan;
mod graph;
mod modulation;
mod node;
mod node_factory;
mod nodes;
mod parameter;
mod plan_handoff;
mod scheduler;
mod state;
mod transport;
mod voice;
mod voice_allocator;

pub mod ffi;

// Re-export key types for Rust consumers
pub use bridge::{create_bridge, EngineHandle, SessionHandle};
pub use compile::compile;
pub use engine::Engine;
pub use node_factory::NodeRegistry;
pub use nodes::register_standard_nodes;
pub use state::{GraphDef, NodeId, NodeTypeId, Session};

