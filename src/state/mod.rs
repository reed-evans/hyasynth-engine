// src/state/mod.rs
//
// Declarative state layer for UI interaction.
//
// This module contains structures that represent the *desired* state
// of the audio engine. The UI manipulates these freely, and the bridge
// synchronizes them to the real-time engine.
//
// Key principles:
// - All structures are serializable (for save/load)
// - All structures are thread-safe to read
// - Mutations happen through Commands
// - The engine never directly accesses these structures

mod command;
mod graph_def;
mod param_info;
mod session;

pub use command::*;
pub use graph_def::*;
pub use param_info::*;
pub use session::*;

