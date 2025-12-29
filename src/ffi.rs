// src/ffi.rs
//
// C-compatible FFI bindings for Swift/iOS integration.
//
// Safety requirements:
// - All pointers must be non-null unless documented otherwise
// - All handles must be created by this module and not fabricated
// - String parameters must be valid UTF-8 (Swift strings are always valid)
// - Caller must call the corresponding _destroy function for each _create

use std::ffi::{c_char, c_float, c_void, CStr};
use std::ptr;

use crate::bridge::{create_bridge, EngineHandle, SessionHandle};
use crate::node_factory::NodeRegistry;
use crate::nodes::register_standard_nodes;
use crate::state::{EngineReadback, NodeId, NodeTypeId, Session};

// ═══════════════════════════════════════════════════════════════════════════
// Opaque Handle Types
// ═══════════════════════════════════════════════════════════════════════════

/// Opaque handle to the SessionHandle (UI-side).
pub struct HyaSynthSession {
    inner: SessionHandle,
}

/// Opaque handle to the EngineHandle (audio-side).
pub struct HyaSynthEngine {
    inner: EngineHandle,
}

/// Opaque handle to the NodeRegistry.
pub struct HyaSynthRegistry {
    inner: NodeRegistry,
}

// ═══════════════════════════════════════════════════════════════════════════
// FFI Result Types
// ═══════════════════════════════════════════════════════════════════════════

/// Readback data from the engine (for UI meters/displays).
#[repr(C)]
pub struct HyaReadback {
    pub sample_position: u64,
    pub beat_position: f64,
    pub cpu_load: f32,
    pub active_voices: u32,
    pub peak_left: f32,
    pub peak_right: f32,
    pub running: bool,
}

impl From<EngineReadback> for HyaReadback {
    fn from(r: EngineReadback) -> Self {
        Self {
            sample_position: r.sample_position,
            beat_position: r.beat_position,
            cpu_load: r.cpu_load,
            active_voices: r.active_voices as u32,
            peak_left: r.output_peaks[0],
            peak_right: r.output_peaks[1],
            running: r.running,
        }
    }
}

/// Node type info for UI display.
#[repr(C)]
pub struct HyaNodeTypeInfo {
    pub type_id: u32,
    pub name: *const c_char,
    pub category: *const c_char,
    pub num_inputs: u32,
    pub num_outputs: u32,
    pub num_params: u32,
}

/// Parameter info for UI controls.
#[repr(C)]
pub struct HyaParamInfo {
    pub id: u32,
    pub name: *const c_char,
    pub min_value: f32,
    pub max_value: f32,
    pub default_value: f32,
}

// ═══════════════════════════════════════════════════════════════════════════
// Registry Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new node registry with all standard nodes.
///
/// Returns an opaque pointer that must be freed with `hya_registry_destroy`.
#[no_mangle]
pub extern "C" fn hya_registry_create() -> *mut HyaSynthRegistry {
    let mut registry = NodeRegistry::new();
    register_standard_nodes(&mut registry);
    Box::into_raw(Box::new(HyaSynthRegistry { inner: registry }))
}

/// Destroy a node registry.
///
/// # Safety
/// `registry` must be a valid pointer returned by `hya_registry_create`.
#[no_mangle]
pub unsafe extern "C" fn hya_registry_destroy(registry: *mut HyaSynthRegistry) {
    if !registry.is_null() {
        drop(Box::from_raw(registry));
    }
}

/// Get the number of registered node types.
#[no_mangle]
pub unsafe extern "C" fn hya_registry_count(registry: *const HyaSynthRegistry) -> u32 {
    if registry.is_null() {
        return 0;
    }
    (*registry).inner.iter().count() as u32
}

// ═══════════════════════════════════════════════════════════════════════════
// Session/Engine Creation
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new session and engine pair.
///
/// Returns a session handle. The engine handle is stored internally and can
/// be retrieved with `hya_session_take_engine`.
///
/// # Safety
/// `name` must be a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn hya_session_create(
    name: *const c_char,
    out_engine: *mut *mut HyaSynthEngine,
) -> *mut HyaSynthSession {
    let name_str = if name.is_null() {
        "Untitled".to_string()
    } else {
        CStr::from_ptr(name)
            .to_str()
            .unwrap_or("Untitled")
            .to_string()
    };

    let session = Session::new(name_str);
    let (session_handle, engine_handle) = create_bridge(session);

    // Output the engine handle
    if !out_engine.is_null() {
        *out_engine = Box::into_raw(Box::new(HyaSynthEngine {
            inner: engine_handle,
        }));
    }

    Box::into_raw(Box::new(HyaSynthSession {
        inner: session_handle,
    }))
}

/// Destroy a session handle.
///
/// # Safety
/// `session` must be a valid pointer returned by `hya_session_create`.
#[no_mangle]
pub unsafe extern "C" fn hya_session_destroy(session: *mut HyaSynthSession) {
    if !session.is_null() {
        drop(Box::from_raw(session));
    }
}

/// Destroy an engine handle.
///
/// # Safety
/// `engine` must be a valid pointer returned via `hya_session_create`.
#[no_mangle]
pub unsafe extern "C" fn hya_engine_destroy(engine: *mut HyaSynthEngine) {
    if !engine.is_null() {
        drop(Box::from_raw(engine));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Session - Graph Mutations
// ═══════════════════════════════════════════════════════════════════════════

/// Add a node to the graph.
///
/// Returns the new node's ID.
#[no_mangle]
pub unsafe extern "C" fn hya_session_add_node(
    session: *mut HyaSynthSession,
    type_id: u32,
    x: f32,
    y: f32,
) -> u32 {
    if session.is_null() {
        return u32::MAX;
    }
    (*session).inner.add_node(type_id, x, y)
}

/// Remove a node from the graph.
#[no_mangle]
pub unsafe extern "C" fn hya_session_remove_node(session: *mut HyaSynthSession, node_id: u32) {
    if session.is_null() {
        return;
    }
    (*session).inner.remove_node(node_id);
}

/// Connect two nodes.
#[no_mangle]
pub unsafe extern "C" fn hya_session_connect(
    session: *mut HyaSynthSession,
    source_node: u32,
    source_port: u32,
    dest_node: u32,
    dest_port: u32,
) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    (*session).inner.send(Command::Connect {
        source_node,
        source_port,
        dest_node,
        dest_port,
    });
}

/// Disconnect two nodes.
#[no_mangle]
pub unsafe extern "C" fn hya_session_disconnect(
    session: *mut HyaSynthSession,
    source_node: u32,
    source_port: u32,
    dest_node: u32,
    dest_port: u32,
) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    (*session).inner.send(Command::Disconnect {
        source_node,
        source_port,
        dest_node,
        dest_port,
    });
}

/// Set the output node.
#[no_mangle]
pub unsafe extern "C" fn hya_session_set_output(session: *mut HyaSynthSession, node_id: u32) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    (*session).inner.send(Command::SetOutputNode { node_id });
}

/// Clear the entire graph.
#[no_mangle]
pub unsafe extern "C" fn hya_session_clear_graph(session: *mut HyaSynthSession) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    (*session).inner.send(Command::ClearGraph);
}

// ═══════════════════════════════════════════════════════════════════════════
// Session - Parameters
// ═══════════════════════════════════════════════════════════════════════════

/// Set a parameter value.
#[no_mangle]
pub unsafe extern "C" fn hya_session_set_param(
    session: *mut HyaSynthSession,
    node_id: u32,
    param_id: u32,
    value: f32,
) {
    if session.is_null() {
        return;
    }
    (*session).inner.set_param(node_id, param_id, value);
}

/// Begin a parameter gesture (for automation recording).
#[no_mangle]
pub unsafe extern "C" fn hya_session_begin_gesture(
    session: *mut HyaSynthSession,
    node_id: u32,
    param_id: u32,
) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    (*session)
        .inner
        .send(Command::BeginParamGesture { node_id, param_id });
}

/// End a parameter gesture.
#[no_mangle]
pub unsafe extern "C" fn hya_session_end_gesture(
    session: *mut HyaSynthSession,
    node_id: u32,
    param_id: u32,
) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    (*session)
        .inner
        .send(Command::EndParamGesture { node_id, param_id });
}

// ═══════════════════════════════════════════════════════════════════════════
// Session - Transport
// ═══════════════════════════════════════════════════════════════════════════

/// Start playback.
#[no_mangle]
pub unsafe extern "C" fn hya_session_play(session: *mut HyaSynthSession) {
    if session.is_null() {
        return;
    }
    (*session).inner.play();
}

/// Stop playback.
#[no_mangle]
pub unsafe extern "C" fn hya_session_stop(session: *mut HyaSynthSession) {
    if session.is_null() {
        return;
    }
    (*session).inner.stop();
}

/// Set tempo in BPM.
#[no_mangle]
pub unsafe extern "C" fn hya_session_set_tempo(session: *mut HyaSynthSession, bpm: f64) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    (*session).inner.send(Command::SetTempo { bpm });
}

/// Seek to a position in beats.
#[no_mangle]
pub unsafe extern "C" fn hya_session_seek(session: *mut HyaSynthSession, beat: f64) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    (*session).inner.send(Command::Seek { beat });
}

// ═══════════════════════════════════════════════════════════════════════════
// Session - MIDI
// ═══════════════════════════════════════════════════════════════════════════

/// Send a MIDI note on.
#[no_mangle]
pub unsafe extern "C" fn hya_session_note_on(
    session: *mut HyaSynthSession,
    note: u8,
    velocity: f32,
) {
    if session.is_null() {
        return;
    }
    (*session).inner.note_on(note, velocity);
}

/// Send a MIDI note off.
#[no_mangle]
pub unsafe extern "C" fn hya_session_note_off(session: *mut HyaSynthSession, note: u8) {
    if session.is_null() {
        return;
    }
    (*session).inner.note_off(note);
}

// ═══════════════════════════════════════════════════════════════════════════
// Session - Readback
// ═══════════════════════════════════════════════════════════════════════════

/// Get the current engine readback state.
#[no_mangle]
pub unsafe extern "C" fn hya_session_get_readback(
    session: *const HyaSynthSession,
) -> HyaReadback {
    if session.is_null() {
        return HyaReadback {
            sample_position: 0,
            beat_position: 0.0,
            cpu_load: 0.0,
            active_voices: 0,
            peak_left: 0.0,
            peak_right: 0.0,
            running: false,
        };
    }
    (*session).inner.readback().into()
}

/// Check if the transport is playing.
#[no_mangle]
pub unsafe extern "C" fn hya_session_is_playing(session: *const HyaSynthSession) -> bool {
    if session.is_null() {
        return false;
    }
    (*session).inner.session().transport.playing
}

/// Get the current tempo.
#[no_mangle]
pub unsafe extern "C" fn hya_session_get_tempo(session: *const HyaSynthSession) -> f64 {
    if session.is_null() {
        return 120.0;
    }
    (*session).inner.session().transport.bpm
}

// ═══════════════════════════════════════════════════════════════════════════
// Session - Graph Query
// ═══════════════════════════════════════════════════════════════════════════

/// Get the number of nodes in the graph.
#[no_mangle]
pub unsafe extern "C" fn hya_session_node_count(session: *const HyaSynthSession) -> u32 {
    if session.is_null() {
        return 0;
    }
    (*session).inner.session().graph.nodes.len() as u32
}

/// Get the output node ID, or u32::MAX if not set.
#[no_mangle]
pub unsafe extern "C" fn hya_session_get_output_node(session: *const HyaSynthSession) -> u32 {
    if session.is_null() {
        return u32::MAX;
    }
    (*session)
        .inner
        .session()
        .graph
        .output_node
        .unwrap_or(u32::MAX)
}

// ═══════════════════════════════════════════════════════════════════════════
// Engine Handle Functions (for audio thread)
// ═══════════════════════════════════════════════════════════════════════════

/// Get the raw engine handle pointer for audio thread use.
/// 
/// The engine handle can be passed to the audio thread to process commands
/// and update readback state.
#[no_mangle]
pub unsafe extern "C" fn hya_engine_get_ptr(engine: *mut HyaSynthEngine) -> *mut c_void {
    engine as *mut c_void
}

/// Update the sample position (called from audio thread).
#[no_mangle]
pub unsafe extern "C" fn hya_engine_update_position(engine: *mut HyaSynthEngine, position: u64) {
    if engine.is_null() {
        return;
    }
    (*engine).inner.update_sample_position(position);
}

/// Update the active voice count (called from audio thread).
#[no_mangle]
pub unsafe extern "C" fn hya_engine_update_voices(engine: *mut HyaSynthEngine, count: u32) {
    if engine.is_null() {
        return;
    }
    (*engine).inner.update_active_voices(count as usize);
}

/// Set the running state (called from audio thread).
#[no_mangle]
pub unsafe extern "C" fn hya_engine_set_running(engine: *mut HyaSynthEngine, running: bool) {
    if engine.is_null() {
        return;
    }
    (*engine).inner.set_running(running);
}

// ═══════════════════════════════════════════════════════════════════════════
// Node Type Constants (for Swift convenience)
// ═══════════════════════════════════════════════════════════════════════════

#[no_mangle]
pub static HYA_NODE_SINE_OSC: u32 = crate::nodes::node_types::SINE_OSC;

#[no_mangle]
pub static HYA_NODE_SAW_OSC: u32 = crate::nodes::node_types::SAW_OSC;

#[no_mangle]
pub static HYA_NODE_SQUARE_OSC: u32 = crate::nodes::node_types::SQUARE_OSC;

#[no_mangle]
pub static HYA_NODE_TRIANGLE_OSC: u32 = crate::nodes::node_types::TRIANGLE_OSC;

#[no_mangle]
pub static HYA_NODE_ADSR_ENV: u32 = crate::nodes::node_types::ADSR_ENV;

#[no_mangle]
pub static HYA_NODE_GAIN: u32 = crate::nodes::node_types::GAIN;

#[no_mangle]
pub static HYA_NODE_PAN: u32 = crate::nodes::node_types::PAN;

#[no_mangle]
pub static HYA_NODE_OUTPUT: u32 = crate::nodes::node_types::OUTPUT;

// ═══════════════════════════════════════════════════════════════════════════
// Parameter ID Constants
// ═══════════════════════════════════════════════════════════════════════════

#[no_mangle]
pub static HYA_PARAM_FREQ: u32 = crate::nodes::params::FREQ;

#[no_mangle]
pub static HYA_PARAM_DETUNE: u32 = crate::nodes::params::DETUNE;

#[no_mangle]
pub static HYA_PARAM_ATTACK: u32 = crate::nodes::params::ATTACK;

#[no_mangle]
pub static HYA_PARAM_DECAY: u32 = crate::nodes::params::DECAY;

#[no_mangle]
pub static HYA_PARAM_SUSTAIN: u32 = crate::nodes::params::SUSTAIN;

#[no_mangle]
pub static HYA_PARAM_RELEASE: u32 = crate::nodes::params::RELEASE;

#[no_mangle]
pub static HYA_PARAM_GAIN: u32 = crate::nodes::params::GAIN;

#[no_mangle]
pub static HYA_PARAM_PAN: u32 = crate::nodes::params::PAN;

