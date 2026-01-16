//! Hyasynth Audio Engine
//!
//! A real-time audio synthesis engine designed for cross-platform integration.
//!
//! # Architecture
//!
//! The engine is split into UI-thread and audio-thread components:
//!
//! - [`SessionHandle`]: UI-side handle for sending commands and reading state
//! - [`EngineHandle`]: Audio-side handle that owns the [`Engine`] and processes audio
//! - [`create_bridge`]: Creates a linked pair of handles for communication
//!
//! # Quick Start
//!
//! ```ignore
//! use hyasynth_engine::*;
//!
//! // Create session and engine
//! let graph = Graph::new(512, 16);
//! let voices = VoiceAllocator::new(16);
//! let engine = Engine::new(graph, voices);
//! let session = Session::new("My Synth");
//!
//! let (mut session_handle, mut engine_handle) = create_bridge(session, engine);
//!
//! // Build a synth graph
//! let osc = session_handle.add_node(1, 0.0, 0.0);  // Oscillator
//! let out = session_handle.add_node(100, 0.0, 0.0); // Output
//! // ... connect and configure
//! ```
//!
//! # Platform Bindings
//!
//! - **iOS/Swift**: Enable the `ios` feature and use the [`ffi`] module for C-compatible functions.
//! - **WebAssembly**: Enable the `web` feature and use the [`wasm`] module for wasm-bindgen exports.

mod audio_buffer;
mod bridge;
mod clip_playback;
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

/// C-compatible FFI bindings for iOS/Swift integration.
/// Only available when the `ios` feature is enabled.
#[cfg(feature = "ios")]
pub mod ffi;

/// WebAssembly bindings via wasm-bindgen.
/// Only available when the `web` feature is enabled.
#[cfg(feature = "web")]
pub mod wasm;

// Re-export key types for Rust consumers
pub use bridge::{EngineHandle, SessionHandle, create_bridge};
pub use clip_playback::ClipPlayback;
pub use compile::compile;
pub use engine::Engine;
pub use node_factory::NodeRegistry;
pub use nodes::register_standard_nodes;
pub use state::{GraphDef, NodeId, NodeTypeId, Session};
