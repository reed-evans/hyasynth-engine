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

// Oscillators
extern const uint32_t NODE_SINE_OSC;
extern const uint32_t NODE_SAW_OSC;
extern const uint32_t NODE_SQUARE_OSC;
extern const uint32_t NODE_TRIANGLE_OSC;

// Envelopes
extern const uint32_t NODE_ADSR_ENV;

// Effects
extern const uint32_t NODE_GAIN;
extern const uint32_t NODE_PAN;
extern const uint32_t NODE_DELAY;
extern const uint32_t NODE_REVERB;

// Filters
extern const uint32_t NODE_LOWPASS;
extern const uint32_t NODE_HIGHPASS;
extern const uint32_t NODE_BANDPASS;
extern const uint32_t NODE_NOTCH;

// Modulators
extern const uint32_t NODE_LFO;

// Utility
extern const uint32_t NODE_OUTPUT;

// ═══════════════════════════════════════════════════════════════════════════
// Parameter ID Constants
// ═══════════════════════════════════════════════════════════════════════════

// Oscillator params
extern const uint32_t PARAM_FREQ;
extern const uint32_t PARAM_DETUNE;

// Envelope params
extern const uint32_t PARAM_ATTACK;
extern const uint32_t PARAM_DECAY;
extern const uint32_t PARAM_SUSTAIN;
extern const uint32_t PARAM_RELEASE;

// Gain/mixer params
extern const uint32_t PARAM_GAIN;
extern const uint32_t PARAM_PAN;

// Filter params
extern const uint32_t PARAM_CUTOFF;
extern const uint32_t PARAM_RESONANCE;

// LFO params
extern const uint32_t PARAM_RATE;
extern const uint32_t PARAM_DEPTH;

// Effect params
extern const uint32_t PARAM_TIME;
extern const uint32_t PARAM_FEEDBACK;
extern const uint32_t PARAM_MIX;
extern const uint32_t PARAM_DAMPING;

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

// ═══════════════════════════════════════════════════════════════════════════
// Clips
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new clip. Returns the clip ID.
uint32_t session_create_clip(HyasynthSession* session, const char* name, double length);

/// Delete a clip.
void session_delete_clip(HyasynthSession* session, uint32_t clip_id);

/// Add a note to a clip.
void session_add_note_to_clip(
    HyasynthSession* session,
    uint32_t clip_id,
    double start,
    double duration,
    uint8_t note,
    float velocity
);

/// Clear all notes from a clip.
void session_clear_clip(HyasynthSession* session, uint32_t clip_id);

/// Get the number of notes in a clip.
uint32_t session_get_clip_note_count(const HyasynthSession* session, uint32_t clip_id);

/// Get the number of audio regions in a clip.
uint32_t session_get_clip_audio_count(const HyasynthSession* session, uint32_t clip_id);

// ═══════════════════════════════════════════════════════════════════════════
// Audio Pool
// ═══════════════════════════════════════════════════════════════════════════

/// Add audio samples to the pool.
/// Returns the audio pool ID.
uint32_t session_add_audio_to_pool(
    HyasynthSession* session,
    const char* name,
    double sample_rate,
    uint32_t channels,
    const float* samples,
    uint32_t num_samples
);

/// Remove audio from the pool.
void session_remove_audio_from_pool(HyasynthSession* session, uint32_t audio_id);

/// Add an audio region to a clip.
void session_add_audio_to_clip(
    HyasynthSession* session,
    uint32_t clip_id,
    double start,
    double duration,
    uint32_t audio_id,
    double source_offset,
    float gain
);

/// Create a clip from audio in the pool.
/// Returns the clip ID, or UINT32_MAX on failure.
uint32_t session_create_clip_from_audio(
    HyasynthSession* session,
    uint32_t audio_id,
    double bpm
);

/// Get the number of audio entries in the pool.
uint32_t session_get_audio_pool_count(const HyasynthSession* session);

// ═══════════════════════════════════════════════════════════════════════════
// Tracks
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new track. Returns the track ID.
uint32_t session_create_track(HyasynthSession* session, const char* name);

/// Delete a track.
void session_delete_track(HyasynthSession* session, uint32_t track_id);

/// Set track volume (0.0 - 1.0).
void session_set_track_volume(HyasynthSession* session, uint32_t track_id, float volume);

/// Set track pan (-1.0 to 1.0).
void session_set_track_pan(HyasynthSession* session, uint32_t track_id, float pan);

/// Set track mute.
void session_set_track_mute(HyasynthSession* session, uint32_t track_id, bool mute);

/// Set track solo.
void session_set_track_solo(HyasynthSession* session, uint32_t track_id, bool solo);

/// Set track target node.
void session_set_track_target(HyasynthSession* session, uint32_t track_id, uint32_t node_id);

/// Get the number of tracks.
uint32_t session_get_track_count(const HyasynthSession* session);

// ═══════════════════════════════════════════════════════════════════════════
// Scenes
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new scene. Returns the scene ID.
uint32_t session_create_scene(HyasynthSession* session, const char* name);

/// Delete a scene.
void session_delete_scene(HyasynthSession* session, uint32_t scene_id);

/// Launch a scene (trigger all clips in that row).
void session_launch_scene(HyasynthSession* session, uint32_t scene_index);

/// Launch a single clip on a track.
void session_launch_clip(HyasynthSession* session, uint32_t track_id, uint32_t clip_id);

/// Stop a clip on a track.
void session_stop_clip(HyasynthSession* session, uint32_t track_id);

/// Stop all clips.
void session_stop_all_clips(HyasynthSession* session);

/// Get the number of scenes.
uint32_t session_get_scene_count(const HyasynthSession* session);

/// Assign a clip to a track's clip slot.
void session_set_clip_slot(
    HyasynthSession* session,
    uint32_t track_id,
    uint32_t scene_index,
    uint32_t clip_id
);

// ═══════════════════════════════════════════════════════════════════════════
// Timeline
// ═══════════════════════════════════════════════════════════════════════════

/// Schedule a clip on the timeline.
void session_schedule_clip(
    HyasynthSession* session,
    uint32_t track_id,
    uint32_t clip_id,
    double start_beat
);

/// Remove a clip placement from the timeline.
void session_remove_clip_placement(
    HyasynthSession* session,
    uint32_t track_id,
    double start_beat
);

#ifdef __cplusplus
}
#endif

#endif // HYASYNTH_H

