// src/ffi.rs
//
// C-compatible FFI bindings for Swift/iOS integration.
//
// Safety requirements:
// - All pointers must be non-null unless documented otherwise
// - All handles must be created by this module and not fabricated
// - String parameters must be valid UTF-8 (Swift strings are always valid)
// - Caller must call the corresponding _destroy function for each _create

use std::ffi::{CStr, c_char, c_void};

use crate::bridge::{EngineHandle, SessionHandle, create_bridge};
use crate::node_factory::NodeRegistry;
use crate::nodes::register_standard_nodes;
use crate::state::{EngineReadback, Session};

// ═══════════════════════════════════════════════════════════════════════════
// Opaque Handle Types
// ═══════════════════════════════════════════════════════════════════════════

/// Opaque handle to the SessionHandle (UI-side).
pub struct HyasynthSession {
    inner: SessionHandle,
}

/// Opaque handle to the EngineHandle (audio-side).
pub struct HyasynthEngine {
    inner: EngineHandle,
}

/// Opaque handle to the NodeRegistry.
pub struct HyasynthRegistry {
    inner: NodeRegistry,
}

// ═══════════════════════════════════════════════════════════════════════════
// FFI Result Types
// ═══════════════════════════════════════════════════════════════════════════

/// Readback data from the engine (for UI meters/displays).
#[repr(C)]
pub struct HyasynthReadback {
    pub sample_position: u64,
    pub beat_position: f64,
    pub cpu_load: f32,
    pub active_voices: u32,
    pub peak_left: f32,
    pub peak_right: f32,
    pub running: bool,
}

impl From<EngineReadback> for HyasynthReadback {
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
/// Returns an opaque pointer that must be freed with `registry_destroy`.
#[unsafe(no_mangle)]
pub extern "C" fn registry_create() -> *mut HyasynthRegistry {
    let mut registry = NodeRegistry::new();
    register_standard_nodes(&mut registry);
    Box::into_raw(Box::new(HyasynthRegistry { inner: registry }))
}

/// Destroy a node registry.
///
/// # Safety
/// `registry` must be a valid pointer returned by `registry_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn registry_destroy(registry: *mut HyasynthRegistry) {
    if !registry.is_null() {
        unsafe { drop(Box::from_raw(registry)) };
    }
}

/// Get the number of registered node types.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn registry_count(registry: *const HyasynthRegistry) -> u32 {
    if registry.is_null() {
        return 0;
    }
    unsafe { (*registry).inner.iter().count() as u32 }
}

// ═══════════════════════════════════════════════════════════════════════════
// Session/Engine Creation
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new session and engine pair.
///
/// Returns a session handle. The engine handle is stored internally and can
/// be retrieved with `session_take_engine`.
///
/// # Safety
/// `name` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_create(
    name: *const c_char,
    out_engine: *mut *mut HyasynthEngine,
) -> *mut HyasynthSession {
    let name_str = if name.is_null() {
        "Untitled".to_string()
    } else {
        unsafe {
            CStr::from_ptr(name)
                .to_str()
                .unwrap_or("Untitled")
                .to_string()
        }
    };

    let session = Session::new(name_str);
    let (session_handle, engine_handle) = create_bridge(session);

    // Output the engine handle
    if !out_engine.is_null() {
        unsafe {
            *out_engine = Box::into_raw(Box::new(HyasynthEngine {
                inner: engine_handle,
            }));
        }
    }

    Box::into_raw(Box::new(HyasynthSession {
        inner: session_handle,
    }))
}

/// Destroy a session handle.
///
/// # Safety
/// `session` must be a valid pointer returned by `session_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_destroy(session: *mut HyasynthSession) {
    if !session.is_null() {
        unsafe { drop(Box::from_raw(session)) };
    }
}

/// Destroy an engine handle.
///
/// # Safety
/// `engine` must be a valid pointer returned via `session_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_destroy(engine: *mut HyasynthEngine) {
    if !engine.is_null() {
        unsafe { drop(Box::from_raw(engine)) };
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Session - Graph Mutations
// ═══════════════════════════════════════════════════════════════════════════

/// Add a node to the graph.
///
/// Returns the new node's ID.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_add_node(
    session: *mut HyasynthSession,
    type_id: u32,
    x: f32,
    y: f32,
) -> u32 {
    if session.is_null() {
        return u32::MAX;
    }
    unsafe { (*session).inner.add_node(type_id, x, y) }
}

/// Remove a node from the graph.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_remove_node(session: *mut HyasynthSession, node_id: u32) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.remove_node(node_id) };
}

/// Connect two nodes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_connect(
    session: *mut HyasynthSession,
    source_node: u32,
    source_port: u32,
    dest_node: u32,
    dest_port: u32,
) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    unsafe {
        (*session).inner.send(Command::Connect {
            source_node,
            source_port,
            dest_node,
            dest_port,
        })
    };
}

/// Disconnect two nodes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_disconnect(
    session: *mut HyasynthSession,
    source_node: u32,
    source_port: u32,
    dest_node: u32,
    dest_port: u32,
) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    unsafe {
        (*session).inner.send(Command::Disconnect {
            source_node,
            source_port,
            dest_node,
            dest_port,
        })
    };
}

/// Set the output node.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_set_output(session: *mut HyasynthSession, node_id: u32) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    unsafe { (*session).inner.send(Command::SetOutputNode { node_id }) };
}

/// Clear the entire graph.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_clear_graph(session: *mut HyasynthSession) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    unsafe { (*session).inner.send(Command::ClearGraph) };
}

// ═══════════════════════════════════════════════════════════════════════════
// Session - Parameters
// ═══════════════════════════════════════════════════════════════════════════

/// Set a parameter value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_set_param(
    session: *mut HyasynthSession,
    node_id: u32,
    param_id: u32,
    value: f32,
) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.set_param(node_id, param_id, value) };
}

/// Begin a parameter gesture (for automation recording).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_begin_gesture(
    session: *mut HyasynthSession,
    node_id: u32,
    param_id: u32,
) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    unsafe {
        (*session)
            .inner
            .send(Command::BeginParamGesture { node_id, param_id })
    };
}

/// End a parameter gesture.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_end_gesture(
    session: *mut HyasynthSession,
    node_id: u32,
    param_id: u32,
) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    unsafe {
        (*session)
            .inner
            .send(Command::EndParamGesture { node_id, param_id })
    };
}

// ═══════════════════════════════════════════════════════════════════════════
// Session - Transport
// ═══════════════════════════════════════════════════════════════════════════

/// Start playback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_play(session: *mut HyasynthSession) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.play() };
}

/// Stop playback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_stop(session: *mut HyasynthSession) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.stop() };
}

/// Set tempo in BPM.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_set_tempo(session: *mut HyasynthSession, bpm: f64) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    unsafe { (*session).inner.send(Command::SetTempo { bpm }) };
}

/// Seek to a position in beats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_seek(session: *mut HyasynthSession, beat: f64) {
    if session.is_null() {
        return;
    }
    use crate::state::Command;
    unsafe { (*session).inner.send(Command::Seek { beat }) };
}

// ═══════════════════════════════════════════════════════════════════════════
// Session - MIDI
// ═══════════════════════════════════════════════════════════════════════════

/// Send a MIDI note on.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_note_on(
    session: *mut HyasynthSession,
    note: u8,
    velocity: f32,
) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.note_on(note, velocity) };
}

/// Send a MIDI note off.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_note_off(session: *mut HyasynthSession, note: u8) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.note_off(note) };
}

// ═══════════════════════════════════════════════════════════════════════════
// Session - Readback
// ═══════════════════════════════════════════════════════════════════════════

/// Get the current engine readback state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_get_readback(
    session: *const HyasynthSession,
) -> HyasynthReadback {
    if session.is_null() {
        return HyasynthReadback {
            sample_position: 0,
            beat_position: 0.0,
            cpu_load: 0.0,
            active_voices: 0,
            peak_left: 0.0,
            peak_right: 0.0,
            running: false,
        };
    }
    unsafe { (*session).inner.readback().into() }
}

/// Check if the transport is playing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_is_playing(session: *const HyasynthSession) -> bool {
    if session.is_null() {
        return false;
    }
    unsafe { (*session).inner.session().transport.playing }
}

/// Get the current tempo.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_get_tempo(session: *const HyasynthSession) -> f64 {
    if session.is_null() {
        return 120.0;
    }
    unsafe { (*session).inner.session().transport.bpm }
}

// ═══════════════════════════════════════════════════════════════════════════
// Session - Graph Query
// ═══════════════════════════════════════════════════════════════════════════

/// Get the number of nodes in the graph.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_node_count(session: *const HyasynthSession) -> u32 {
    if session.is_null() {
        return 0;
    }
    unsafe { (*session).inner.session().graph.nodes.len() as u32 }
}

/// Get the output node ID, or u32::MAX if not set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_get_output_node(session: *const HyasynthSession) -> u32 {
    if session.is_null() {
        return u32::MAX;
    }
    unsafe {
        (*session)
            .inner
            .session()
            .graph
            .output_node
            .unwrap_or(u32::MAX)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Engine Handle Functions (for audio thread)
// ═══════════════════════════════════════════════════════════════════════════

/// Get the raw engine handle pointer for audio thread use.
///
/// The engine handle can be passed to the audio thread to process commands
/// and update readback state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_get_ptr(engine: *mut HyasynthEngine) -> *mut c_void {
    engine as *mut c_void
}

/// Update the sample position (called from audio thread).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_update_position(engine: *mut HyasynthEngine, position: u64) {
    if engine.is_null() {
        return;
    }
    unsafe { (*engine).inner.update_sample_position(position) };
}

/// Update the active voice count (called from audio thread).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_update_voices(engine: *mut HyasynthEngine, count: u32) {
    if engine.is_null() {
        return;
    }
    unsafe { (*engine).inner.update_active_voices(count as usize) };
}

/// Set the running state (called from audio thread).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_set_running(engine: *mut HyasynthEngine, running: bool) {
    if engine.is_null() {
        return;
    }
    unsafe { (*engine).inner.set_running(running) };
}

// ═══════════════════════════════════════════════════════════════════════════
// Clip Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new clip.
/// Returns the clip ID.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_create_clip(
    session: *mut HyasynthSession,
    name: *const c_char,
    length: f64,
) -> u32 {
    if session.is_null() {
        return u32::MAX;
    }
    let name_str = if name.is_null() {
        "Clip".to_string()
    } else {
        unsafe { CStr::from_ptr(name).to_str().unwrap_or("Clip").to_string() }
    };
    unsafe { (*session).inner.session_mut().arrangement.create_clip(name_str, length) }
}

/// Delete a clip.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_delete_clip(session: *mut HyasynthSession, clip_id: u32) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.session_mut().arrangement.delete_clip(clip_id) };
}

/// Add a note to a clip.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_add_note_to_clip(
    session: *mut HyasynthSession,
    clip_id: u32,
    start: f64,
    duration: f64,
    note: u8,
    velocity: f32,
) {
    if session.is_null() {
        return;
    }
    use crate::state::NoteDef;
    unsafe {
        (*session).inner.session_mut().arrangement.add_note_to_clip(
            clip_id,
            NoteDef::new(start, duration, note, velocity),
        )
    };
}

/// Clear all notes from a clip.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_clear_clip(session: *mut HyasynthSession, clip_id: u32) {
    if session.is_null() {
        return;
    }
    unsafe {
        if let Some(clip) = (*session).inner.session_mut().arrangement.get_clip_mut(clip_id) {
            clip.clear();
        }
    };
}

/// Get the number of notes in a clip.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_get_clip_note_count(
    session: *const HyasynthSession,
    clip_id: u32,
) -> u32 {
    if session.is_null() {
        return 0;
    }
    unsafe {
        (*session)
            .inner
            .session()
            .arrangement
            .get_clip(clip_id)
            .map(|c| c.note_count() as u32)
            .unwrap_or(0)
    }
}

/// Get the number of audio regions in a clip.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_get_clip_audio_count(
    session: *const HyasynthSession,
    clip_id: u32,
) -> u32 {
    if session.is_null() {
        return 0;
    }
    unsafe {
        (*session)
            .inner
            .session()
            .arrangement
            .get_clip(clip_id)
            .map(|c| c.audio_count() as u32)
            .unwrap_or(0)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Audio Pool Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Add audio to the pool.
///
/// # Safety
/// `samples` must point to `num_samples` valid f32 values.
/// Returns the audio pool ID.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_add_audio_to_pool(
    session: *mut HyasynthSession,
    name: *const c_char,
    sample_rate: f64,
    channels: u32,
    samples: *const f32,
    num_samples: u32,
) -> u32 {
    if session.is_null() || samples.is_null() {
        return u32::MAX;
    }
    let name_str = if name.is_null() {
        "Audio".to_string()
    } else {
        unsafe { CStr::from_ptr(name).to_str().unwrap_or("Audio").to_string() }
    };

    let samples_vec = unsafe {
        std::slice::from_raw_parts(samples, num_samples as usize).to_vec()
    };

    unsafe {
        (*session).inner.session_mut().arrangement.add_audio_to_pool(
            name_str,
            sample_rate,
            channels as usize,
            samples_vec,
        )
    }
}

/// Remove audio from the pool.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_remove_audio_from_pool(
    session: *mut HyasynthSession,
    audio_id: u32,
) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.session_mut().arrangement.remove_audio(audio_id) };
}

/// Add an audio region to a clip.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_add_audio_to_clip(
    session: *mut HyasynthSession,
    clip_id: u32,
    start: f64,
    duration: f64,
    audio_id: u32,
    source_offset: f64,
    gain: f32,
) {
    if session.is_null() {
        return;
    }
    use crate::state::AudioRegionDef;
    let region = AudioRegionDef::new(start, duration, audio_id)
        .with_offset(source_offset)
        .with_gain(gain);

    unsafe {
        (*session)
            .inner
            .session_mut()
            .arrangement
            .add_audio_to_clip(clip_id, region)
    };
}

/// Create a clip from audio in the pool.
///
/// Creates a new clip containing the full audio at the given tempo.
/// Returns the clip ID, or u32::MAX on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_create_clip_from_audio(
    session: *mut HyasynthSession,
    audio_id: u32,
    bpm: f64,
) -> u32 {
    if session.is_null() {
        return u32::MAX;
    }
    unsafe {
        (*session)
            .inner
            .session_mut()
            .arrangement
            .create_clip_from_audio(audio_id, bpm)
            .unwrap_or(u32::MAX)
    }
}

/// Get the number of audio entries in the pool.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_get_audio_pool_count(session: *const HyasynthSession) -> u32 {
    if session.is_null() {
        return 0;
    }
    unsafe {
        (*session)
            .inner
            .session()
            .arrangement
            .audio_pool
            .iter()
            .count() as u32
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Track Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new track.
/// Returns the track ID.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_create_track(
    session: *mut HyasynthSession,
    name: *const c_char,
) -> u32 {
    if session.is_null() {
        return u32::MAX;
    }
    let name_str = if name.is_null() {
        "Track".to_string()
    } else {
        unsafe { CStr::from_ptr(name).to_str().unwrap_or("Track").to_string() }
    };
    unsafe { (*session).inner.session_mut().arrangement.create_track(name_str) }
}

/// Delete a track.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_delete_track(session: *mut HyasynthSession, track_id: u32) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.session_mut().arrangement.delete_track(track_id) };
}

/// Set track volume (0.0 - 1.0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_set_track_volume(
    session: *mut HyasynthSession,
    track_id: u32,
    volume: f32,
) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.session_mut().arrangement.set_track_volume(track_id, volume) };
}

/// Set track pan (-1.0 to 1.0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_set_track_pan(
    session: *mut HyasynthSession,
    track_id: u32,
    pan: f32,
) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.session_mut().arrangement.set_track_pan(track_id, pan) };
}

/// Set track mute.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_set_track_mute(
    session: *mut HyasynthSession,
    track_id: u32,
    mute: bool,
) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.session_mut().arrangement.set_track_mute(track_id, mute) };
}

/// Set track solo.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_set_track_solo(
    session: *mut HyasynthSession,
    track_id: u32,
    solo: bool,
) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.session_mut().arrangement.set_track_solo(track_id, solo) };
}

/// Set track target node (the node this track sends MIDI to).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_set_track_target(
    session: *mut HyasynthSession,
    track_id: u32,
    node_id: u32,
) {
    if session.is_null() {
        return;
    }
    let target = if node_id == u32::MAX { None } else { Some(node_id) };
    unsafe { (*session).inner.session_mut().arrangement.set_track_target(track_id, target) };
}

/// Get the number of tracks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_get_track_count(session: *const HyasynthSession) -> u32 {
    if session.is_null() {
        return 0;
    }
    unsafe { (*session).inner.session().arrangement.tracks.len() as u32 }
}

// ═══════════════════════════════════════════════════════════════════════════
// Scene Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new scene.
/// Returns the scene ID.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_create_scene(
    session: *mut HyasynthSession,
    name: *const c_char,
) -> u32 {
    if session.is_null() {
        return u32::MAX;
    }
    let name_str = if name.is_null() {
        "Scene".to_string()
    } else {
        unsafe { CStr::from_ptr(name).to_str().unwrap_or("Scene").to_string() }
    };
    unsafe { (*session).inner.session_mut().arrangement.create_scene(name_str) }
}

/// Delete a scene.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_delete_scene(session: *mut HyasynthSession, scene_id: u32) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.session_mut().arrangement.delete_scene(scene_id) };
}

/// Launch a scene (trigger all clips in that row).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_launch_scene(session: *mut HyasynthSession, scene_index: u32) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.session_mut().arrangement.launch_scene(scene_index as usize) };
}

/// Launch a single clip on a track.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_launch_clip(
    session: *mut HyasynthSession,
    track_id: u32,
    clip_id: u32,
) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.session_mut().arrangement.launch_clip(track_id, clip_id) };
}

/// Stop a clip on a track.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_stop_clip(session: *mut HyasynthSession, track_id: u32) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.session_mut().arrangement.stop_clip(track_id) };
}

/// Stop all clips.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_stop_all_clips(session: *mut HyasynthSession) {
    if session.is_null() {
        return;
    }
    unsafe { (*session).inner.session_mut().arrangement.stop_all() };
}

/// Get the number of scenes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_get_scene_count(session: *const HyasynthSession) -> u32 {
    if session.is_null() {
        return 0;
    }
    unsafe { (*session).inner.session().arrangement.scenes.len() as u32 }
}

/// Assign a clip to a track's clip slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_set_clip_slot(
    session: *mut HyasynthSession,
    track_id: u32,
    scene_index: u32,
    clip_id: u32,
) {
    if session.is_null() {
        return;
    }
    let clip = if clip_id == u32::MAX { None } else { Some(clip_id) };
    unsafe {
        (*session)
            .inner
            .session_mut()
            .arrangement
            .set_clip_slot(track_id, scene_index as usize, clip)
    };
}

// ═══════════════════════════════════════════════════════════════════════════
// Timeline Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Schedule a clip on the timeline.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_schedule_clip(
    session: *mut HyasynthSession,
    track_id: u32,
    clip_id: u32,
    start_beat: f64,
) {
    if session.is_null() {
        return;
    }
    unsafe {
        (*session)
            .inner
            .session_mut()
            .arrangement
            .schedule_clip(track_id, clip_id, start_beat)
    };
}

/// Remove a clip placement from the timeline.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_remove_clip_placement(
    session: *mut HyasynthSession,
    track_id: u32,
    start_beat: f64,
) {
    if session.is_null() {
        return;
    }
    unsafe {
        (*session)
            .inner
            .session_mut()
            .arrangement
            .remove_clip_placement(track_id, start_beat)
    };
}

// ═══════════════════════════════════════════════════════════════════════════
// Node Type Constants (for Swift convenience)
// ═══════════════════════════════════════════════════════════════════════════

#[unsafe(no_mangle)]
pub static NODE_SINE_OSC: u32 = crate::nodes::node_types::SINE_OSC;

#[unsafe(no_mangle)]
pub static NODE_SAW_OSC: u32 = crate::nodes::node_types::SAW_OSC;

#[unsafe(no_mangle)]
pub static NODE_SQUARE_OSC: u32 = crate::nodes::node_types::SQUARE_OSC;

#[unsafe(no_mangle)]
pub static NODE_TRIANGLE_OSC: u32 = crate::nodes::node_types::TRIANGLE_OSC;

#[unsafe(no_mangle)]
pub static NODE_ADSR_ENV: u32 = crate::nodes::node_types::ADSR_ENV;

#[unsafe(no_mangle)]
pub static NODE_GAIN: u32 = crate::nodes::node_types::GAIN;

#[unsafe(no_mangle)]
pub static NODE_PAN: u32 = crate::nodes::node_types::PAN;

#[unsafe(no_mangle)]
pub static NODE_OUTPUT: u32 = crate::nodes::node_types::OUTPUT;

#[unsafe(no_mangle)]
pub static NODE_LOWPASS: u32 = crate::nodes::node_types::LOWPASS;

#[unsafe(no_mangle)]
pub static NODE_HIGHPASS: u32 = crate::nodes::node_types::HIGHPASS;

#[unsafe(no_mangle)]
pub static NODE_BANDPASS: u32 = crate::nodes::node_types::BANDPASS;

#[unsafe(no_mangle)]
pub static NODE_NOTCH: u32 = crate::nodes::node_types::NOTCH;

#[unsafe(no_mangle)]
pub static NODE_LFO: u32 = crate::nodes::node_types::LFO;

#[unsafe(no_mangle)]
pub static NODE_DELAY: u32 = crate::nodes::node_types::DELAY;

#[unsafe(no_mangle)]
pub static NODE_REVERB: u32 = crate::nodes::node_types::REVERB;

// ═══════════════════════════════════════════════════════════════════════════
// Parameter ID Constants
// ═══════════════════════════════════════════════════════════════════════════

#[unsafe(no_mangle)]
pub static PARAM_FREQ: u32 = crate::nodes::params::FREQ;

#[unsafe(no_mangle)]
pub static PARAM_DETUNE: u32 = crate::nodes::params::DETUNE;

#[unsafe(no_mangle)]
pub static PARAM_ATTACK: u32 = crate::nodes::params::ATTACK;

#[unsafe(no_mangle)]
pub static PARAM_DECAY: u32 = crate::nodes::params::DECAY;

#[unsafe(no_mangle)]
pub static PARAM_SUSTAIN: u32 = crate::nodes::params::SUSTAIN;

#[unsafe(no_mangle)]
pub static PARAM_RELEASE: u32 = crate::nodes::params::RELEASE;

#[unsafe(no_mangle)]
pub static PARAM_GAIN: u32 = crate::nodes::params::GAIN;

#[unsafe(no_mangle)]
pub static PARAM_PAN: u32 = crate::nodes::params::PAN;

#[unsafe(no_mangle)]
pub static PARAM_CUTOFF: u32 = crate::nodes::params::CUTOFF;

#[unsafe(no_mangle)]
pub static PARAM_RESONANCE: u32 = crate::nodes::params::RESONANCE;

#[unsafe(no_mangle)]
pub static PARAM_RATE: u32 = crate::nodes::params::RATE;

#[unsafe(no_mangle)]
pub static PARAM_DEPTH: u32 = crate::nodes::params::DEPTH;

#[unsafe(no_mangle)]
pub static PARAM_TIME: u32 = crate::nodes::params::TIME;

#[unsafe(no_mangle)]
pub static PARAM_FEEDBACK: u32 = crate::nodes::params::FEEDBACK;

#[unsafe(no_mangle)]
pub static PARAM_MIX: u32 = crate::nodes::params::MIX;

#[unsafe(no_mangle)]
pub static PARAM_DAMPING: u32 = crate::nodes::params::DAMPING;
