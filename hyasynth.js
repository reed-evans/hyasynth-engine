/* @ts-self-types="./hyasynth.d.ts" */

/**
 * Configuration for creating a session and engine.
 */
export class HyasynthConfig {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(HyasynthConfig.prototype);
        obj.__wbg_ptr = ptr;
        HyasynthConfigFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        HyasynthConfigFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_hyasynthconfig_free(ptr, 0);
    }
    /**
     * Maximum audio block size in frames (e.g., 128, 256, 512).
     * @returns {number}
     */
    get max_block_size() {
        const ret = wasm.__wbg_get_hyasynthconfig_max_block_size(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Maximum number of simultaneous voices for polyphony.
     * @returns {number}
     */
    get max_voices() {
        const ret = wasm.__wbg_get_hyasynthconfig_max_voices(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Sample rate in Hz (e.g., 44100.0, 48000.0).
     * @returns {number}
     */
    get sample_rate() {
        const ret = wasm.__wbg_get_hyasynthconfig_sample_rate(this.__wbg_ptr);
        return ret;
    }
    /**
     * Create a new configuration with default values.
     */
    constructor() {
        const ret = wasm.hyasynthconfig_new();
        this.__wbg_ptr = ret >>> 0;
        HyasynthConfigFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Create a configuration with custom values.
     * @param {number} max_block_size
     * @param {number} max_voices
     * @param {number} sample_rate
     * @returns {HyasynthConfig}
     */
    static with_values(max_block_size, max_voices, sample_rate) {
        const ret = wasm.hyasynthconfig_with_values(max_block_size, max_voices, sample_rate);
        return HyasynthConfig.__wrap(ret);
    }
    /**
     * Maximum audio block size in frames (e.g., 128, 256, 512).
     * @param {number} arg0
     */
    set max_block_size(arg0) {
        wasm.__wbg_set_hyasynthconfig_max_block_size(this.__wbg_ptr, arg0);
    }
    /**
     * Maximum number of simultaneous voices for polyphony.
     * @param {number} arg0
     */
    set max_voices(arg0) {
        wasm.__wbg_set_hyasynthconfig_max_voices(this.__wbg_ptr, arg0);
    }
    /**
     * Sample rate in Hz (e.g., 44100.0, 48000.0).
     * @param {number} arg0
     */
    set sample_rate(arg0) {
        wasm.__wbg_set_hyasynthconfig_sample_rate(this.__wbg_ptr, arg0);
    }
}
if (Symbol.dispose) HyasynthConfig.prototype[Symbol.dispose] = HyasynthConfig.prototype.free;

/**
 * Audio-side engine handle for rendering audio.
 * Use this in an AudioWorklet for real-time processing.
 */
export class HyasynthEngine {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(HyasynthEngine.prototype);
        obj.__wbg_ptr = ptr;
        HyasynthEngineFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        HyasynthEngineFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_hyasynthengine_free(ptr, 0);
    }
    /**
     * Compile the session's graph and load it into the engine.
     * @param {HyasynthSession} session
     * @param {HyasynthRegistry} registry
     * @param {number} sample_rate
     * @returns {boolean}
     */
    compile_graph(session, registry, sample_rate) {
        _assertClass(session, HyasynthSession);
        _assertClass(registry, HyasynthRegistry);
        const ret = wasm.hyasynthengine_compile_graph(this.__wbg_ptr, session.__wbg_ptr, registry.__wbg_ptr, sample_rate);
        return ret !== 0;
    }
    /**
     * Get the number of active voices.
     * @returns {number}
     */
    get_active_voices() {
        const ret = wasm.hyasynthengine_get_active_voices(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get the current tempo in BPM.
     * @returns {number}
     */
    get_tempo() {
        const ret = wasm.hyasynthengine_get_tempo(this.__wbg_ptr);
        return ret;
    }
    /**
     * Check if the engine is currently playing.
     * @returns {boolean}
     */
    is_playing() {
        const ret = wasm.hyasynthengine_is_playing(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Prepare the engine's graph for processing.
     * @param {number} sample_rate
     */
    prepare(sample_rate) {
        wasm.hyasynthengine_prepare(this.__wbg_ptr, sample_rate);
    }
    /**
     * Process all pending commands from the UI thread.
     * Call this at the start of each audio render callback.
     * Returns true if any command requires graph recompilation.
     * @returns {boolean}
     */
    process_commands() {
        const ret = wasm.hyasynthengine_process_commands(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Render audio frames to the provided output buffer (interleaved stereo).
     * Output format: [L0, R0, L1, R1, L2, R2, ...]
     *
     * The output slice must have length >= frames * 2.
     * @param {number} frames
     * @param {Float32Array} output
     */
    render(frames, output) {
        var ptr0 = passArrayF32ToWasm0(output, wasm.__wbindgen_malloc);
        var len0 = WASM_VECTOR_LEN;
        wasm.hyasynthengine_render(this.__wbg_ptr, frames, ptr0, len0, output);
    }
    /**
     * Reset the engine state.
     */
    reset() {
        wasm.hyasynthengine_reset(this.__wbg_ptr);
    }
    /**
     * Update beat position (for external timing sync).
     * @param {number} position
     */
    update_beat_position(position) {
        wasm.hyasynthengine_update_beat_position(this.__wbg_ptr, position);
    }
    /**
     * Update sample position (for external timing sync).
     * @param {bigint} position
     */
    update_position(position) {
        wasm.hyasynthengine_update_position(this.__wbg_ptr, position);
    }
}
if (Symbol.dispose) HyasynthEngine.prototype[Symbol.dispose] = HyasynthEngine.prototype.free;

/**
 * Readback data from the engine (for UI meters/displays).
 */
export class HyasynthReadback {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(HyasynthReadback.prototype);
        obj.__wbg_ptr = ptr;
        HyasynthReadbackFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        HyasynthReadbackFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_hyasynthreadback_free(ptr, 0);
    }
    /**
     * Number of currently active voices.
     * @returns {number}
     */
    get active_voices() {
        const ret = wasm.__wbg_get_hyasynthreadback_active_voices(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Current beat position in the timeline.
     * @returns {number}
     */
    get beat_position() {
        const ret = wasm.__wbg_get_hyasynthreadback_beat_position(this.__wbg_ptr);
        return ret;
    }
    /**
     * CPU load estimate (0.0 - 1.0).
     * @returns {number}
     */
    get cpu_load() {
        const ret = wasm.__wbg_get_hyasynthreadback_cpu_load(this.__wbg_ptr);
        return ret;
    }
    /**
     * Peak level of left channel.
     * @returns {number}
     */
    get peak_left() {
        const ret = wasm.__wbg_get_hyasynthreadback_peak_left(this.__wbg_ptr);
        return ret;
    }
    /**
     * Peak level of right channel.
     * @returns {number}
     */
    get peak_right() {
        const ret = wasm.__wbg_get_hyasynthreadback_peak_right(this.__wbg_ptr);
        return ret;
    }
    /**
     * Whether the engine is running.
     * @returns {boolean}
     */
    get running() {
        const ret = wasm.__wbg_get_hyasynthreadback_running(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Current sample position in the timeline.
     * @returns {bigint}
     */
    get sample_position() {
        const ret = wasm.__wbg_get_hyasynthreadback_sample_position(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * Number of currently active voices.
     * @param {number} arg0
     */
    set active_voices(arg0) {
        wasm.__wbg_set_hyasynthreadback_active_voices(this.__wbg_ptr, arg0);
    }
    /**
     * Current beat position in the timeline.
     * @param {number} arg0
     */
    set beat_position(arg0) {
        wasm.__wbg_set_hyasynthreadback_beat_position(this.__wbg_ptr, arg0);
    }
    /**
     * CPU load estimate (0.0 - 1.0).
     * @param {number} arg0
     */
    set cpu_load(arg0) {
        wasm.__wbg_set_hyasynthreadback_cpu_load(this.__wbg_ptr, arg0);
    }
    /**
     * Peak level of left channel.
     * @param {number} arg0
     */
    set peak_left(arg0) {
        wasm.__wbg_set_hyasynthreadback_peak_left(this.__wbg_ptr, arg0);
    }
    /**
     * Peak level of right channel.
     * @param {number} arg0
     */
    set peak_right(arg0) {
        wasm.__wbg_set_hyasynthreadback_peak_right(this.__wbg_ptr, arg0);
    }
    /**
     * Whether the engine is running.
     * @param {boolean} arg0
     */
    set running(arg0) {
        wasm.__wbg_set_hyasynthreadback_running(this.__wbg_ptr, arg0);
    }
    /**
     * Current sample position in the timeline.
     * @param {bigint} arg0
     */
    set sample_position(arg0) {
        wasm.__wbg_set_hyasynthreadback_sample_position(this.__wbg_ptr, arg0);
    }
}
if (Symbol.dispose) HyasynthReadback.prototype[Symbol.dispose] = HyasynthReadback.prototype.free;

/**
 * Node registry containing all available node types.
 */
export class HyasynthRegistry {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        HyasynthRegistryFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_hyasynthregistry_free(ptr, 0);
    }
    /**
     * Get the number of registered node types.
     * @returns {number}
     */
    count() {
        const ret = wasm.hyasynthregistry_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Create a new registry with all standard nodes registered.
     */
    constructor() {
        const ret = wasm.hyasynthregistry_new();
        this.__wbg_ptr = ret >>> 0;
        HyasynthRegistryFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) HyasynthRegistry.prototype[Symbol.dispose] = HyasynthRegistry.prototype.free;

/**
 * UI-side session handle for building and controlling the synth.
 */
export class HyasynthSession {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(HyasynthSession.prototype);
        obj.__wbg_ptr = ptr;
        HyasynthSessionFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        HyasynthSessionFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_hyasynthsession_free(ptr, 0);
    }
    /**
     * Add a modulation route. Returns the route ID.
     * Requires graph recompilation to take effect.
     * @param {number} source_node
     * @param {number} source_port
     * @param {number} dest_node
     * @param {number} dest_param
     * @param {number} depth
     * @returns {number}
     */
    add_mod_route(source_node, source_port, dest_node, dest_param, depth) {
        const ret = wasm.hyasynthsession_add_mod_route(this.__wbg_ptr, source_node, source_port, dest_node, dest_param, depth);
        return ret >>> 0;
    }
    /**
     * Add a node to the graph. Returns the new node's ID.
     * @param {number} type_id
     * @param {number} x
     * @param {number} y
     * @returns {number}
     */
    add_node(type_id, x, y) {
        const ret = wasm.hyasynthsession_add_node(this.__wbg_ptr, type_id, x, y);
        return ret >>> 0;
    }
    /**
     * Add a note to a clip.
     * @param {number} clip_id
     * @param {number} start
     * @param {number} duration
     * @param {number} note
     * @param {number} velocity
     */
    add_note_to_clip(clip_id, start, duration, note, velocity) {
        wasm.hyasynthsession_add_note_to_clip(this.__wbg_ptr, clip_id, start, duration, note, velocity);
    }
    /**
     * Begin a parameter gesture (for automation recording).
     * @param {number} node_id
     * @param {number} param_id
     */
    begin_gesture(node_id, param_id) {
        wasm.hyasynthsession_begin_gesture(this.__wbg_ptr, node_id, param_id);
    }
    /**
     * Clear all notes from a clip.
     * @param {number} clip_id
     */
    clear_clip(clip_id) {
        wasm.hyasynthsession_clear_clip(this.__wbg_ptr, clip_id);
    }
    /**
     * Clear the entire graph.
     */
    clear_graph() {
        wasm.hyasynthsession_clear_graph(this.__wbg_ptr);
    }
    /**
     * Connect two nodes.
     * @param {number} source_node
     * @param {number} source_port
     * @param {number} dest_node
     * @param {number} dest_port
     */
    connect(source_node, source_port, dest_node, dest_port) {
        wasm.hyasynthsession_connect(this.__wbg_ptr, source_node, source_port, dest_node, dest_port);
    }
    /**
     * Create a new clip. Returns the clip ID.
     * @param {string} name
     * @param {number} length
     * @returns {number}
     */
    create_clip(name, length) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.hyasynthsession_create_clip(this.__wbg_ptr, ptr0, len0, length);
        return ret >>> 0;
    }
    /**
     * Create an engine handle paired with this session.
     * The engine processes audio and should be used in an AudioWorklet.
     * Must be called exactly once — the engine handle is created with the session
     * via create_bridge and can only be taken once.
     * @returns {HyasynthEngine}
     */
    create_engine() {
        const ret = wasm.hyasynthsession_create_engine(this.__wbg_ptr);
        return HyasynthEngine.__wrap(ret);
    }
    /**
     * Create a new scene. Returns the scene ID.
     * @param {string} name
     * @returns {number}
     */
    create_scene(name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.hyasynthsession_create_scene(this.__wbg_ptr, ptr0, len0);
        return ret >>> 0;
    }
    /**
     * Create a new track. Returns the track ID.
     * @param {string} name
     * @returns {number}
     */
    create_track(name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.hyasynthsession_create_track(this.__wbg_ptr, ptr0, len0);
        return ret >>> 0;
    }
    /**
     * Delete a clip.
     * @param {number} clip_id
     */
    delete_clip(clip_id) {
        wasm.hyasynthsession_delete_clip(this.__wbg_ptr, clip_id);
    }
    /**
     * Delete a scene.
     * @param {number} scene_id
     */
    delete_scene(scene_id) {
        wasm.hyasynthsession_delete_scene(this.__wbg_ptr, scene_id);
    }
    /**
     * Delete a track.
     * @param {number} track_id
     */
    delete_track(track_id) {
        wasm.hyasynthsession_delete_track(this.__wbg_ptr, track_id);
    }
    /**
     * Disconnect two nodes.
     * @param {number} source_node
     * @param {number} source_port
     * @param {number} dest_node
     * @param {number} dest_port
     */
    disconnect(source_node, source_port, dest_node, dest_port) {
        wasm.hyasynthsession_disconnect(this.__wbg_ptr, source_node, source_port, dest_node, dest_port);
    }
    /**
     * End a parameter gesture.
     * @param {number} node_id
     * @param {number} param_id
     */
    end_gesture(node_id, param_id) {
        wasm.hyasynthsession_end_gesture(this.__wbg_ptr, node_id, param_id);
    }
    /**
     * Get the number of notes in a clip.
     * @param {number} clip_id
     * @returns {number}
     */
    get_clip_note_count(clip_id) {
        const ret = wasm.hyasynthsession_get_clip_note_count(this.__wbg_ptr, clip_id);
        return ret >>> 0;
    }
    /**
     * Get the output node ID, or u32::MAX if not set.
     * @returns {number}
     */
    get_output_node() {
        const ret = wasm.hyasynthsession_get_output_node(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get the current engine readback state.
     * @returns {HyasynthReadback}
     */
    get_readback() {
        const ret = wasm.hyasynthsession_get_readback(this.__wbg_ptr);
        return HyasynthReadback.__wrap(ret);
    }
    /**
     * Get the number of scenes.
     * @returns {number}
     */
    get_scene_count() {
        const ret = wasm.hyasynthsession_get_scene_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get the current tempo.
     * @returns {number}
     */
    get_tempo() {
        const ret = wasm.hyasynthsession_get_tempo(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get the number of tracks.
     * @returns {number}
     */
    get_track_count() {
        const ret = wasm.hyasynthsession_get_track_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Check if the transport is playing.
     * @returns {boolean}
     */
    is_playing() {
        const ret = wasm.hyasynthsession_is_playing(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Launch a single clip on a track.
     * @param {number} track_id
     * @param {number} clip_id
     */
    launch_clip(track_id, clip_id) {
        wasm.hyasynthsession_launch_clip(this.__wbg_ptr, track_id, clip_id);
    }
    /**
     * Launch a scene (trigger all clips in that row).
     * @param {number} scene_index
     */
    launch_scene(scene_index) {
        wasm.hyasynthsession_launch_scene(this.__wbg_ptr, scene_index);
    }
    /**
     * Create a new session with default configuration.
     * @param {string} name
     */
    constructor(name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.hyasynthsession_new(ptr0, len0);
        this.__wbg_ptr = ret >>> 0;
        HyasynthSessionFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Create a new session with custom configuration.
     * @param {string} name
     * @param {HyasynthConfig} config
     * @returns {HyasynthSession}
     */
    static new_with_config(name, config) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertClass(config, HyasynthConfig);
        var ptr1 = config.__destroy_into_raw();
        const ret = wasm.hyasynthsession_new_with_config(ptr0, len0, ptr1);
        return HyasynthSession.__wrap(ret);
    }
    /**
     * Get the number of nodes in the graph.
     * @returns {number}
     */
    node_count() {
        const ret = wasm.hyasynthsession_node_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Send a MIDI note off.
     * @param {number} note
     */
    note_off(note) {
        wasm.hyasynthsession_note_off(this.__wbg_ptr, note);
    }
    /**
     * Send a MIDI note on.
     * @param {number} note
     * @param {number} velocity
     */
    note_on(note, velocity) {
        wasm.hyasynthsession_note_on(this.__wbg_ptr, note, velocity);
    }
    /**
     * Start playback.
     */
    play() {
        wasm.hyasynthsession_play(this.__wbg_ptr);
    }
    /**
     * Remove a clip placement from the timeline.
     * @param {number} track_id
     * @param {number} start_beat
     */
    remove_clip_placement(track_id, start_beat) {
        wasm.hyasynthsession_remove_clip_placement(this.__wbg_ptr, track_id, start_beat);
    }
    /**
     * Remove a modulation route by ID.
     * Requires graph recompilation to take effect.
     * @param {number} route_id
     */
    remove_mod_route(route_id) {
        wasm.hyasynthsession_remove_mod_route(this.__wbg_ptr, route_id);
    }
    /**
     * Remove a node from the graph.
     * @param {number} node_id
     */
    remove_node(node_id) {
        wasm.hyasynthsession_remove_node(this.__wbg_ptr, node_id);
    }
    /**
     * Schedule a clip on the timeline.
     * @param {number} track_id
     * @param {number} clip_id
     * @param {number} start_beat
     */
    schedule_clip(track_id, clip_id, start_beat) {
        wasm.hyasynthsession_schedule_clip(this.__wbg_ptr, track_id, clip_id, start_beat);
    }
    /**
     * Seek to a position in beats.
     * @param {number} beat
     */
    seek(beat) {
        wasm.hyasynthsession_seek(this.__wbg_ptr, beat);
    }
    /**
     * Assign a clip to a track's clip slot.
     * Pass u32::MAX for clip_id to clear the slot.
     * @param {number} track_id
     * @param {number} scene_index
     * @param {number} clip_id
     */
    set_clip_slot(track_id, scene_index, clip_id) {
        wasm.hyasynthsession_set_clip_slot(this.__wbg_ptr, track_id, scene_index, clip_id);
    }
    /**
     * Set the depth of a modulation route (-1.0 to 1.0).
     * This is real-time safe and takes effect immediately.
     * @param {number} route_id
     * @param {number} depth
     */
    set_mod_depth(route_id, depth) {
        wasm.hyasynthsession_set_mod_depth(this.__wbg_ptr, route_id, depth);
    }
    /**
     * Set the output node.
     * @param {number} node_id
     */
    set_output(node_id) {
        wasm.hyasynthsession_set_output(this.__wbg_ptr, node_id);
    }
    /**
     * Set a parameter value.
     * @param {number} node_id
     * @param {number} param_id
     * @param {number} value
     */
    set_param(node_id, param_id, value) {
        wasm.hyasynthsession_set_param(this.__wbg_ptr, node_id, param_id, value);
    }
    /**
     * Set tempo in BPM.
     * @param {number} bpm
     */
    set_tempo(bpm) {
        wasm.hyasynthsession_set_tempo(this.__wbg_ptr, bpm);
    }
    /**
     * Set track mute.
     * @param {number} track_id
     * @param {boolean} mute
     */
    set_track_mute(track_id, mute) {
        wasm.hyasynthsession_set_track_mute(this.__wbg_ptr, track_id, mute);
    }
    /**
     * Set track pan (-1.0 to 1.0).
     * @param {number} track_id
     * @param {number} pan
     */
    set_track_pan(track_id, pan) {
        wasm.hyasynthsession_set_track_pan(this.__wbg_ptr, track_id, pan);
    }
    /**
     * Set track solo.
     * @param {number} track_id
     * @param {boolean} solo
     */
    set_track_solo(track_id, solo) {
        wasm.hyasynthsession_set_track_solo(this.__wbg_ptr, track_id, solo);
    }
    /**
     * Set track target node (the node this track sends MIDI to).
     * Pass u32::MAX to clear the target.
     * @param {number} track_id
     * @param {number} node_id
     */
    set_track_target(track_id, node_id) {
        wasm.hyasynthsession_set_track_target(this.__wbg_ptr, track_id, node_id);
    }
    /**
     * Set track volume (0.0 - 1.0).
     * @param {number} track_id
     * @param {number} volume
     */
    set_track_volume(track_id, volume) {
        wasm.hyasynthsession_set_track_volume(this.__wbg_ptr, track_id, volume);
    }
    /**
     * Stop playback.
     */
    stop() {
        wasm.hyasynthsession_stop(this.__wbg_ptr);
    }
    /**
     * Stop all clips.
     */
    stop_all_clips() {
        wasm.hyasynthsession_stop_all_clips(this.__wbg_ptr);
    }
    /**
     * Stop a clip on a track.
     * @param {number} track_id
     */
    stop_clip(track_id) {
        wasm.hyasynthsession_stop_clip(this.__wbg_ptr, track_id);
    }
}
if (Symbol.dispose) HyasynthSession.prototype[Symbol.dispose] = HyasynthSession.prototype.free;

/**
 * Initialize the wasm module. Call this once before using any other functions.
 * Sets up panic hook to log to the browser console via web_sys.
 */
export function hyasynth_init() {
    wasm.hyasynth_init();
}

/**
 * ADSR envelope node type.
 * @returns {number}
 */
export function node_adsr_env() {
    const ret = wasm.node_adsr_env();
    return ret >>> 0;
}

/**
 * Audio player node type.
 * @returns {number}
 */
export function node_audio_player() {
    const ret = wasm.node_audio_player();
    return ret >>> 0;
}

/**
 * Bandpass filter node type.
 * @returns {number}
 */
export function node_bandpass() {
    const ret = wasm.node_bandpass();
    return ret >>> 0;
}

/**
 * Delay node type.
 * @returns {number}
 */
export function node_delay() {
    const ret = wasm.node_delay();
    return ret >>> 0;
}

/**
 * Gain node type.
 * @returns {number}
 */
export function node_gain() {
    const ret = wasm.node_gain();
    return ret >>> 0;
}

/**
 * Highpass filter node type.
 * @returns {number}
 */
export function node_highpass() {
    const ret = wasm.node_highpass();
    return ret >>> 0;
}

/**
 * LFO node type.
 * @returns {number}
 */
export function node_lfo() {
    const ret = wasm.node_lfo();
    return ret >>> 0;
}

/**
 * Lowpass filter node type.
 * @returns {number}
 */
export function node_lowpass() {
    const ret = wasm.node_lowpass();
    return ret >>> 0;
}

/**
 * Mixer node type.
 * @returns {number}
 */
export function node_mixer() {
    const ret = wasm.node_mixer();
    return ret >>> 0;
}

/**
 * Notch filter node type.
 * @returns {number}
 */
export function node_notch() {
    const ret = wasm.node_notch();
    return ret >>> 0;
}

/**
 * Output node type.
 * @returns {number}
 */
export function node_output() {
    const ret = wasm.node_output();
    return ret >>> 0;
}

/**
 * Pan node type.
 * @returns {number}
 */
export function node_pan() {
    const ret = wasm.node_pan();
    return ret >>> 0;
}

/**
 * Phase distortion oscillator node type.
 * @returns {number}
 */
export function node_phase_osc() {
    const ret = wasm.node_phase_osc();
    return ret >>> 0;
}

/**
 * Reverb node type.
 * @returns {number}
 */
export function node_reverb() {
    const ret = wasm.node_reverb();
    return ret >>> 0;
}

/**
 * Saw oscillator node type.
 * @returns {number}
 */
export function node_saw_osc() {
    const ret = wasm.node_saw_osc();
    return ret >>> 0;
}

/**
 * Sine oscillator node type.
 * @returns {number}
 */
export function node_sine_osc() {
    const ret = wasm.node_sine_osc();
    return ret >>> 0;
}

/**
 * Square oscillator node type.
 * @returns {number}
 */
export function node_square_osc() {
    const ret = wasm.node_square_osc();
    return ret >>> 0;
}

/**
 * Transport node type.
 * @returns {number}
 */
export function node_transport() {
    const ret = wasm.node_transport();
    return ret >>> 0;
}

/**
 * Triangle oscillator node type.
 * @returns {number}
 */
export function node_triangle_osc() {
    const ret = wasm.node_triangle_osc();
    return ret >>> 0;
}

/**
 * Algorithm parameter ID (phase oscillator).
 * @returns {number}
 */
export function param_algorithm() {
    const ret = wasm.param_algorithm();
    return ret >>> 0;
}

/**
 * Attack parameter ID.
 * @returns {number}
 */
export function param_attack() {
    const ret = wasm.param_attack();
    return ret >>> 0;
}

/**
 * Attack curve parameter ID.
 * @returns {number}
 */
export function param_attack_curve() {
    const ret = wasm.param_attack_curve();
    return ret >>> 0;
}

/**
 * Cutoff parameter ID.
 * @returns {number}
 */
export function param_cutoff() {
    const ret = wasm.param_cutoff();
    return ret >>> 0;
}

/**
 * Damping parameter ID.
 * @returns {number}
 */
export function param_damping() {
    const ret = wasm.param_damping();
    return ret >>> 0;
}

/**
 * Decay parameter ID.
 * @returns {number}
 */
export function param_decay() {
    const ret = wasm.param_decay();
    return ret >>> 0;
}

/**
 * Decay curve parameter ID.
 * @returns {number}
 */
export function param_decay_curve() {
    const ret = wasm.param_decay_curve();
    return ret >>> 0;
}

/**
 * Depth parameter ID.
 * @returns {number}
 */
export function param_depth() {
    const ret = wasm.param_depth();
    return ret >>> 0;
}

/**
 * Detune parameter ID.
 * @returns {number}
 */
export function param_detune() {
    const ret = wasm.param_detune();
    return ret >>> 0;
}

/**
 * Feedback parameter ID.
 * @returns {number}
 */
export function param_feedback() {
    const ret = wasm.param_feedback();
    return ret >>> 0;
}

/**
 * Formant parameter ID (phase oscillator).
 * @returns {number}
 */
export function param_formant() {
    const ret = wasm.param_formant();
    return ret >>> 0;
}

/**
 * Frequency parameter ID.
 * @returns {number}
 */
export function param_freq() {
    const ret = wasm.param_freq();
    return ret >>> 0;
}

/**
 * Gain parameter ID.
 * @returns {number}
 */
export function param_gain() {
    const ret = wasm.param_gain();
    return ret >>> 0;
}

/**
 * Key tracking parameter ID.
 * @returns {number}
 */
export function param_key_tracking() {
    const ret = wasm.param_key_tracking();
    return ret >>> 0;
}

/**
 * Mix parameter ID.
 * @returns {number}
 */
export function param_mix() {
    const ret = wasm.param_mix();
    return ret >>> 0;
}

/**
 * Mode parameter ID.
 * @returns {number}
 */
export function param_mode() {
    const ret = wasm.param_mode();
    return ret >>> 0;
}

/**
 * Oscillator key tracking parameter ID.
 * @returns {number}
 */
export function param_osc_key_tracking() {
    const ret = wasm.param_osc_key_tracking();
    return ret >>> 0;
}

/**
 * Pan parameter ID.
 * @returns {number}
 */
export function param_pan() {
    const ret = wasm.param_pan();
    return ret >>> 0;
}

/**
 * Phase parameter ID.
 * @returns {number}
 */
export function param_phase() {
    const ret = wasm.param_phase();
    return ret >>> 0;
}

/**
 * Pitch offset parameter ID (phase oscillator).
 * @returns {number}
 */
export function param_pitch_offset() {
    const ret = wasm.param_pitch_offset();
    return ret >>> 0;
}

/**
 * Pitch ratio denominator parameter ID (phase oscillator).
 * @returns {number}
 */
export function param_pitch_ratio_den() {
    const ret = wasm.param_pitch_ratio_den();
    return ret >>> 0;
}

/**
 * Pitch ratio numerator parameter ID (phase oscillator).
 * @returns {number}
 */
export function param_pitch_ratio_num() {
    const ret = wasm.param_pitch_ratio_num();
    return ret >>> 0;
}

/**
 * Phase oscillator detune parameter ID.
 * @returns {number}
 */
export function param_po_detune() {
    const ret = wasm.param_po_detune();
    return ret >>> 0;
}

/**
 * Phase oscillator feedback parameter ID.
 * @returns {number}
 */
export function param_po_feedback() {
    const ret = wasm.param_po_feedback();
    return ret >>> 0;
}

/**
 * Phase oscillator key tracking parameter ID.
 * @returns {number}
 */
export function param_po_key_tracking() {
    const ret = wasm.param_po_key_tracking();
    return ret >>> 0;
}

/**
 * Pulse width parameter ID.
 * @returns {number}
 */
export function param_pulse_width() {
    const ret = wasm.param_pulse_width();
    return ret >>> 0;
}

/**
 * Rate parameter ID.
 * @returns {number}
 */
export function param_rate() {
    const ret = wasm.param_rate();
    return ret >>> 0;
}

/**
 * Release parameter ID.
 * @returns {number}
 */
export function param_release() {
    const ret = wasm.param_release();
    return ret >>> 0;
}

/**
 * Release curve parameter ID.
 * @returns {number}
 */
export function param_release_curve() {
    const ret = wasm.param_release_curve();
    return ret >>> 0;
}

/**
 * Resonance parameter ID.
 * @returns {number}
 */
export function param_resonance() {
    const ret = wasm.param_resonance();
    return ret >>> 0;
}

/**
 * Reverb decay parameter ID.
 * @returns {number}
 */
export function param_reverb_decay() {
    const ret = wasm.param_reverb_decay();
    return ret >>> 0;
}

/**
 * Shape parameter ID (phase oscillator).
 * @returns {number}
 */
export function param_shape() {
    const ret = wasm.param_shape();
    return ret >>> 0;
}

/**
 * Stereo detune toggle parameter ID (phase oscillator).
 * @returns {number}
 */
export function param_stereo_detune() {
    const ret = wasm.param_stereo_detune();
    return ret >>> 0;
}

/**
 * Sustain parameter ID.
 * @returns {number}
 */
export function param_sustain() {
    const ret = wasm.param_sustain();
    return ret >>> 0;
}

/**
 * Time parameter ID.
 * @returns {number}
 */
export function param_time() {
    const ret = wasm.param_time();
    return ret >>> 0;
}

/**
 * Waveform parameter ID.
 * @returns {number}
 */
export function param_waveform() {
    const ret = wasm.param_waveform();
    return ret >>> 0;
}

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_copy_to_typed_array_d2f20acdab8e0740: function(arg0, arg1, arg2) {
            new Uint8Array(arg2.buffer, arg2.byteOffset, arg2.byteLength).set(getArrayU8FromWasm0(arg0, arg1));
        },
        __wbg___wbindgen_throw_6ddd609b62940d55: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_error_8d9a8e04cd1d3588: function(arg0) {
            console.error(arg0);
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./hyasynth_bg.js": import0,
    };
}

const HyasynthConfigFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_hyasynthconfig_free(ptr >>> 0, 1));
const HyasynthEngineFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_hyasynthengine_free(ptr >>> 0, 1));
const HyasynthReadbackFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_hyasynthreadback_free(ptr >>> 0, 1));
const HyasynthRegistryFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_hyasynthregistry_free(ptr >>> 0, 1));
const HyasynthSessionFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_hyasynthsession_free(ptr >>> 0, 1));

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArrayF32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getFloat32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedFloat32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('hyasynth_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
