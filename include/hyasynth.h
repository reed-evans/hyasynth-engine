// hyasynth.h
//
// C header for HyaSynth audio engine FFI.
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
typedef struct HyaSynthSession HyaSynthSession;

/// Opaque handle to the engine (audio-side).
typedef struct HyaSynthEngine HyaSynthEngine;

/// Opaque handle to the node registry.
typedef struct HyaSynthRegistry HyaSynthRegistry;

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
} HyaReadback;

// ═══════════════════════════════════════════════════════════════════════════
// Node Type Constants
// ═══════════════════════════════════════════════════════════════════════════

extern const uint32_t HYA_NODE_SINE_OSC;
extern const uint32_t HYA_NODE_SAW_OSC;
extern const uint32_t HYA_NODE_SQUARE_OSC;
extern const uint32_t HYA_NODE_TRIANGLE_OSC;
extern const uint32_t HYA_NODE_ADSR_ENV;
extern const uint32_t HYA_NODE_GAIN;
extern const uint32_t HYA_NODE_PAN;
extern const uint32_t HYA_NODE_OUTPUT;

// ═══════════════════════════════════════════════════════════════════════════
// Parameter ID Constants
// ═══════════════════════════════════════════════════════════════════════════

extern const uint32_t HYA_PARAM_FREQ;
extern const uint32_t HYA_PARAM_DETUNE;
extern const uint32_t HYA_PARAM_ATTACK;
extern const uint32_t HYA_PARAM_DECAY;
extern const uint32_t HYA_PARAM_SUSTAIN;
extern const uint32_t HYA_PARAM_RELEASE;
extern const uint32_t HYA_PARAM_GAIN;
extern const uint32_t HYA_PARAM_PAN;

// ═══════════════════════════════════════════════════════════════════════════
// Registry Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new node registry with all standard nodes.
/// Returns an opaque pointer that must be freed with hya_registry_destroy.
HyaSynthRegistry* hya_registry_create(void);

/// Destroy a node registry.
void hya_registry_destroy(HyaSynthRegistry* registry);

/// Get the number of registered node types.
uint32_t hya_registry_count(const HyaSynthRegistry* registry);

// ═══════════════════════════════════════════════════════════════════════════
// Session/Engine Lifecycle
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new session and engine pair.
/// 
/// @param name Session name (UTF-8, null-terminated). Pass NULL for "Untitled".
/// @param out_engine Pointer to receive the engine handle.
/// @return The session handle.
HyaSynthSession* hya_session_create(
    const char* name,
    HyaSynthEngine** out_engine
);

/// Destroy a session handle.
void hya_session_destroy(HyaSynthSession* session);

/// Destroy an engine handle.
void hya_engine_destroy(HyaSynthEngine* engine);

// ═══════════════════════════════════════════════════════════════════════════
// Graph Mutations
// ═══════════════════════════════════════════════════════════════════════════

/// Add a node to the graph.
/// @return The new node's ID, or UINT32_MAX on error.
uint32_t hya_session_add_node(
    HyaSynthSession* session,
    uint32_t type_id,
    float x,
    float y
);

/// Remove a node from the graph.
void hya_session_remove_node(HyaSynthSession* session, uint32_t node_id);

/// Connect two nodes.
void hya_session_connect(
    HyaSynthSession* session,
    uint32_t source_node,
    uint32_t source_port,
    uint32_t dest_node,
    uint32_t dest_port
);

/// Disconnect two nodes.
void hya_session_disconnect(
    HyaSynthSession* session,
    uint32_t source_node,
    uint32_t source_port,
    uint32_t dest_node,
    uint32_t dest_port
);

/// Set the output node.
void hya_session_set_output(HyaSynthSession* session, uint32_t node_id);

/// Clear the entire graph.
void hya_session_clear_graph(HyaSynthSession* session);

// ═══════════════════════════════════════════════════════════════════════════
// Parameters
// ═══════════════════════════════════════════════════════════════════════════

/// Set a parameter value.
void hya_session_set_param(
    HyaSynthSession* session,
    uint32_t node_id,
    uint32_t param_id,
    float value
);

/// Begin a parameter gesture (for automation recording).
void hya_session_begin_gesture(
    HyaSynthSession* session,
    uint32_t node_id,
    uint32_t param_id
);

/// End a parameter gesture.
void hya_session_end_gesture(
    HyaSynthSession* session,
    uint32_t node_id,
    uint32_t param_id
);

// ═══════════════════════════════════════════════════════════════════════════
// Transport
// ═══════════════════════════════════════════════════════════════════════════

/// Start playback.
void hya_session_play(HyaSynthSession* session);

/// Stop playback.
void hya_session_stop(HyaSynthSession* session);

/// Set tempo in BPM.
void hya_session_set_tempo(HyaSynthSession* session, double bpm);

/// Seek to a position in beats.
void hya_session_seek(HyaSynthSession* session, double beat);

// ═══════════════════════════════════════════════════════════════════════════
// MIDI
// ═══════════════════════════════════════════════════════════════════════════

/// Send a MIDI note on.
void hya_session_note_on(HyaSynthSession* session, uint8_t note, float velocity);

/// Send a MIDI note off.
void hya_session_note_off(HyaSynthSession* session, uint8_t note);

// ═══════════════════════════════════════════════════════════════════════════
// Readback (UI polling)
// ═══════════════════════════════════════════════════════════════════════════

/// Get the current engine readback state.
HyaReadback hya_session_get_readback(const HyaSynthSession* session);

/// Check if the transport is playing.
bool hya_session_is_playing(const HyaSynthSession* session);

/// Get the current tempo.
double hya_session_get_tempo(const HyaSynthSession* session);

/// Get the number of nodes in the graph.
uint32_t hya_session_node_count(const HyaSynthSession* session);

/// Get the output node ID, or UINT32_MAX if not set.
uint32_t hya_session_get_output_node(const HyaSynthSession* session);

// ═══════════════════════════════════════════════════════════════════════════
// Engine (Audio Thread)
// ═══════════════════════════════════════════════════════════════════════════

/// Get the raw engine handle pointer for audio thread use.
void* hya_engine_get_ptr(HyaSynthEngine* engine);

/// Update the sample position (called from audio thread).
void hya_engine_update_position(HyaSynthEngine* engine, uint64_t position);

/// Update the active voice count (called from audio thread).
void hya_engine_update_voices(HyaSynthEngine* engine, uint32_t count);

/// Set the running state (called from audio thread).
void hya_engine_set_running(HyaSynthEngine* engine, bool running);

#ifdef __cplusplus
}
#endif

#endif // HYASYNTH_H

