// src/lib.rs
//
// Library entry point for FFI consumers (iOS/Swift).

mod audio_buffer;
mod bridge;
mod clip_playback;
mod compile;
mod engine;
mod engine_controller;
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
pub use bridge::{EngineHandle, SessionHandle, create_bridge};
pub use clip_playback::ClipPlayback;
pub use compile::compile;
pub use engine::Engine;
pub use engine_controller::{EngineController, create_engine_controller};
pub use node_factory::NodeRegistry;
pub use nodes::register_standard_nodes;
pub use state::{GraphDef, NodeId, NodeTypeId, Session};
