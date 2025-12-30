// hyasynth.h
//
// C header for Hyasynth audio engine FFI.
// Use this with Swift via a bridging header or module map.

#ifndef HYASYNTH_H
#define HYASYNTH_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ═══════════════════════════════════════════════════════════════════════════
// Opaque Types
// ═══════════════════════════════════════════════════════════════════════════

/// Opaque handle to the session (UI-side).
typedef struct HyasynthSession HyasynthSession;

/// Opaque handle to the engine (audio-side).
typedef struct HyasynthEngine HyasynthEngine;

/// Opaque handle to the node registry.
typedef struct HyasynthRegistry HyasynthRegistry;

// ═══════════════════════════════════════════════════════════════════════════
// Data Structures
// ═══════════════════════════════════════════════════════════════════════════

/// Engine readback data for UI meters/displays.
typedef struct {
    uint64_t sample_position;
    double beat_position;
    float cpu_load;
    uint32_t active_voices;
    float peak_left;
    float peak_right;
    bool running;
} HyasynthReadback;

// ═══════════════════════════════════════════════════════════════════════════
// Node Type Constants
// ═══════════════════════════════════════════════════════════════════════════

extern const uint32_t NODE_SINE_OSC;
extern const uint32_t NODE_SAW_OSC;
extern const uint32_t NODE_SQUARE_OSC;
extern const uint32_t NODE_TRIANGLE_OSC;
extern const uint32_t NODE_ADSR_ENV;
extern const uint32_t NODE_GAIN;
extern const uint32_t NODE_PAN;
extern const uint32_t NODE_OUTPUT;

// ═══════════════════════════════════════════════════════════════════════════
// Parameter ID Constants
// ═══════════════════════════════════════════════════════════════════════════

extern const uint32_t PARAM_FREQ;
extern const uint32_t PARAM_DETUNE;
extern const uint32_t PARAM_ATTACK;
extern const uint32_t PARAM_DECAY;
extern const uint32_t PARAM_SUSTAIN;
extern const uint32_t PARAM_RELEASE;
extern const uint32_t PARAM_GAIN;
extern const uint32_t PARAM_PAN;

// ═══════════════════════════════════════════════════════════════════════════
// Registry Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new node registry with all standard nodes.
/// Returns an opaque pointer that must be freed with registry_destroy.
HyasynthRegistry* registry_create(void);

/// Destroy a node registry.
void registry_destroy(HyasynthRegistry* registry);

/// Get the number of registered node types.
uint32_t registry_count(const HyasynthRegistry* registry);

// ═══════════════════════════════════════════════════════════════════════════
// Session/Engine Lifecycle
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new session and engine pair.
/// 
/// @param name Session name (UTF-8, null-terminated). Pass NULL for "Untitled".
/// @param out_engine Pointer to receive the engine handle.
/// @return The session handle.
HyasynthSession* session_create(
    const char* name,
    HyasynthEngine** out_engine
);

/// Destroy a session handle.
void session_destroy(HyasynthSession* session);

/// Destroy an engine handle.
void engine_destroy(HyasynthEngine* engine);

// ═══════════════════════════════════════════════════════════════════════════
// Graph Mutations
// ═══════════════════════════════════════════════════════════════════════════

/// Add a node to the graph.
/// @return The new node's ID, or UINT32_MAX on error.
uint32_t session_add_node(
    HyasynthSession* session,
    uint32_t type_id,
    float x,
    float y
);

/// Remove a node from the graph.
void session_remove_node(HyasynthSession* session, uint32_t node_id);

/// Connect two nodes.
void session_connect(
    HyasynthSession* session,
    uint32_t source_node,
    uint32_t source_port,
    uint32_t dest_node,
    uint32_t dest_port
);

/// Disconnect two nodes.
void session_disconnect(
    HyasynthSession* session,
    uint32_t source_node,
    uint32_t source_port,
    uint32_t dest_node,
    uint32_t dest_port
);

/// Set the output node.
void session_set_output(HyasynthSession* session, uint32_t node_id);

/// Clear the entire graph.
void session_clear_graph(HyasynthSession* session);

// ═══════════════════════════════════════════════════════════════════════════
// Parameters
// ═══════════════════════════════════════════════════════════════════════════

/// Set a parameter value.
void session_set_param(
    HyasynthSession* session,
    uint32_t node_id,
    uint32_t param_id,
    float value
);

/// Begin a parameter gesture (for automation recording).
void session_begin_gesture(
    HyasynthSession* session,
    uint32_t node_id,
    uint32_t param_id
);

/// End a parameter gesture.
void session_end_gesture(
    HyasynthSession* session,
    uint32_t node_id,
    uint32_t param_id
);

// ═══════════════════════════════════════════════════════════════════════════
// Transport
// ═══════════════════════════════════════════════════════════════════════════

/// Start playback.
void session_play(HyasynthSession* session);

/// Stop playback.
void session_stop(HyasynthSession* session);

/// Set tempo in BPM.
void session_set_tempo(HyasynthSession* session, double bpm);

/// Seek to a position in beats.
void session_seek(HyasynthSession* session, double beat);

// ═══════════════════════════════════════════════════════════════════════════
// MIDI
// ═══════════════════════════════════════════════════════════════════════════

/// Send a MIDI note on.
void session_note_on(HyasynthSession* session, uint8_t note, float velocity);

/// Send a MIDI note off.
void session_note_off(HyasynthSession* session, uint8_t note);

// ═══════════════════════════════════════════════════════════════════════════
// Readback (UI polling)
// ═══════════════════════════════════════════════════════════════════════════

/// Get the current engine readback state.
HyasynthReadback session_get_readback(const HyasynthSession* session);

/// Check if the transport is playing.
bool session_is_playing(const HyasynthSession* session);

/// Get the current tempo.
double session_get_tempo(const HyasynthSession* session);

/// Get the number of nodes in the graph.
uint32_t session_node_count(const HyasynthSession* session);

/// Get the output node ID, or UINT32_MAX if not set.
uint32_t session_get_output_node(const HyasynthSession* session);

// ═══════════════════════════════════════════════════════════════════════════
// Engine (Audio Thread)
// ═══════════════════════════════════════════════════════════════════════════

/// Get the raw engine handle pointer for audio thread use.
void* engine_get_ptr(HyasynthEngine* engine);

/// Update the sample position (called from audio thread).
void engine_update_position(HyasynthEngine* engine, uint64_t position);

/// Update the active voice count (called from audio thread).
void engine_update_voices(HyasynthEngine* engine, uint32_t count);

/// Set the running state (called from audio thread).
void engine_set_running(HyasynthEngine* engine, bool running);

#ifdef __cplusplus
}
#endif

#endif // HYASYNTH_H

