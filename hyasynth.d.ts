/* tslint:disable */
/* eslint-disable */

/**
 * Configuration for creating a session and engine.
 */
export class HyasynthConfig {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Create a new configuration with default values.
     */
    constructor();
    /**
     * Create a configuration with custom values.
     */
    static with_values(max_block_size: number, max_voices: number, sample_rate: number): HyasynthConfig;
    /**
     * Maximum audio block size in frames (e.g., 128, 256, 512).
     */
    max_block_size: number;
    /**
     * Maximum number of simultaneous voices for polyphony.
     */
    max_voices: number;
    /**
     * Sample rate in Hz (e.g., 44100.0, 48000.0).
     */
    sample_rate: number;
}

/**
 * Audio-side engine handle for rendering audio.
 * Use this in an AudioWorklet for real-time processing.
 */
export class HyasynthEngine {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Compile the session's graph and load it into the engine.
     */
    compile_graph(session: HyasynthSession, registry: HyasynthRegistry, sample_rate: number): boolean;
    /**
     * Get the number of active voices.
     */
    get_active_voices(): number;
    /**
     * Get the current tempo in BPM.
     */
    get_tempo(): number;
    /**
     * Check if the engine is currently playing.
     */
    is_playing(): boolean;
    /**
     * Prepare the engine's graph for processing.
     */
    prepare(sample_rate: number): void;
    /**
     * Process all pending commands from the UI thread.
     * Call this at the start of each audio render callback.
     * Returns true if any command requires graph recompilation.
     */
    process_commands(): boolean;
    /**
     * Render audio frames to the provided output buffer (interleaved stereo).
     * Output format: [L0, R0, L1, R1, L2, R2, ...]
     *
     * The output slice must have length >= frames * 2.
     */
    render(frames: number, output: Float32Array): void;
    /**
     * Reset the engine state.
     */
    reset(): void;
    /**
     * Update beat position (for external timing sync).
     */
    update_beat_position(position: number): void;
    /**
     * Update sample position (for external timing sync).
     */
    update_position(position: bigint): void;
}

/**
 * Readback data from the engine (for UI meters/displays).
 */
export class HyasynthReadback {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Number of currently active voices.
     */
    active_voices: number;
    /**
     * Current beat position in the timeline.
     */
    beat_position: number;
    /**
     * CPU load estimate (0.0 - 1.0).
     */
    cpu_load: number;
    /**
     * Peak level of left channel.
     */
    peak_left: number;
    /**
     * Peak level of right channel.
     */
    peak_right: number;
    /**
     * Whether the engine is running.
     */
    running: boolean;
    /**
     * Current sample position in the timeline.
     */
    sample_position: bigint;
}

/**
 * Node registry containing all available node types.
 */
export class HyasynthRegistry {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Get the number of registered node types.
     */
    count(): number;
    /**
     * Create a new registry with all standard nodes registered.
     */
    constructor();
}

/**
 * UI-side session handle for building and controlling the synth.
 */
export class HyasynthSession {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Add a modulation route. Returns the route ID.
     * Requires graph recompilation to take effect.
     */
    add_mod_route(source_node: number, source_port: number, dest_node: number, dest_param: number, depth: number): number;
    /**
     * Add a node to the graph. Returns the new node's ID.
     */
    add_node(type_id: number, x: number, y: number): number;
    /**
     * Add a note to a clip.
     */
    add_note_to_clip(clip_id: number, start: number, duration: number, note: number, velocity: number): void;
    /**
     * Begin a parameter gesture (for automation recording).
     */
    begin_gesture(node_id: number, param_id: number): void;
    /**
     * Clear all notes from a clip.
     */
    clear_clip(clip_id: number): void;
    /**
     * Clear the entire graph.
     */
    clear_graph(): void;
    /**
     * Connect two nodes.
     */
    connect(source_node: number, source_port: number, dest_node: number, dest_port: number): void;
    /**
     * Create a new clip. Returns the clip ID.
     */
    create_clip(name: string, length: number): number;
    /**
     * Create an engine handle paired with this session.
     * The engine processes audio and should be used in an AudioWorklet.
     * Must be called exactly once — the engine handle is created with the session
     * via create_bridge and can only be taken once.
     */
    create_engine(): HyasynthEngine;
    /**
     * Create a new scene. Returns the scene ID.
     */
    create_scene(name: string): number;
    /**
     * Create a new track. Returns the track ID.
     */
    create_track(name: string): number;
    /**
     * Delete a clip.
     */
    delete_clip(clip_id: number): void;
    /**
     * Delete a scene.
     */
    delete_scene(scene_id: number): void;
    /**
     * Delete a track.
     */
    delete_track(track_id: number): void;
    /**
     * Disconnect two nodes.
     */
    disconnect(source_node: number, source_port: number, dest_node: number, dest_port: number): void;
    /**
     * End a parameter gesture.
     */
    end_gesture(node_id: number, param_id: number): void;
    /**
     * Get the number of notes in a clip.
     */
    get_clip_note_count(clip_id: number): number;
    /**
     * Get the output node ID, or u32::MAX if not set.
     */
    get_output_node(): number;
    /**
     * Get the current engine readback state.
     */
    get_readback(): HyasynthReadback;
    /**
     * Get the number of scenes.
     */
    get_scene_count(): number;
    /**
     * Get the current tempo.
     */
    get_tempo(): number;
    /**
     * Get the number of tracks.
     */
    get_track_count(): number;
    /**
     * Check if the transport is playing.
     */
    is_playing(): boolean;
    /**
     * Launch a single clip on a track.
     */
    launch_clip(track_id: number, clip_id: number): void;
    /**
     * Launch a scene (trigger all clips in that row).
     */
    launch_scene(scene_index: number): void;
    /**
     * Create a new session with default configuration.
     */
    constructor(name: string);
    /**
     * Create a new session with custom configuration.
     */
    static new_with_config(name: string, config: HyasynthConfig): HyasynthSession;
    /**
     * Get the number of nodes in the graph.
     */
    node_count(): number;
    /**
     * Send a MIDI note off.
     */
    note_off(note: number): void;
    /**
     * Send a MIDI note on.
     */
    note_on(note: number, velocity: number): void;
    /**
     * Start playback.
     */
    play(): void;
    /**
     * Remove a clip placement from the timeline.
     */
    remove_clip_placement(track_id: number, start_beat: number): void;
    /**
     * Remove a modulation route by ID.
     * Requires graph recompilation to take effect.
     */
    remove_mod_route(route_id: number): void;
    /**
     * Remove a node from the graph.
     */
    remove_node(node_id: number): void;
    /**
     * Schedule a clip on the timeline.
     */
    schedule_clip(track_id: number, clip_id: number, start_beat: number): void;
    /**
     * Seek to a position in beats.
     */
    seek(beat: number): void;
    /**
     * Assign a clip to a track's clip slot.
     * Pass u32::MAX for clip_id to clear the slot.
     */
    set_clip_slot(track_id: number, scene_index: number, clip_id: number): void;
    /**
     * Set the depth of a modulation route (-1.0 to 1.0).
     * This is real-time safe and takes effect immediately.
     */
    set_mod_depth(route_id: number, depth: number): void;
    /**
     * Set the output node.
     */
    set_output(node_id: number): void;
    /**
     * Set a parameter value.
     */
    set_param(node_id: number, param_id: number, value: number): void;
    /**
     * Set tempo in BPM.
     */
    set_tempo(bpm: number): void;
    /**
     * Set track mute.
     */
    set_track_mute(track_id: number, mute: boolean): void;
    /**
     * Set track pan (-1.0 to 1.0).
     */
    set_track_pan(track_id: number, pan: number): void;
    /**
     * Set track solo.
     */
    set_track_solo(track_id: number, solo: boolean): void;
    /**
     * Set track target node (the node this track sends MIDI to).
     * Pass u32::MAX to clear the target.
     */
    set_track_target(track_id: number, node_id: number): void;
    /**
     * Set track volume (0.0 - 1.0).
     */
    set_track_volume(track_id: number, volume: number): void;
    /**
     * Stop playback.
     */
    stop(): void;
    /**
     * Stop all clips.
     */
    stop_all_clips(): void;
    /**
     * Stop a clip on a track.
     */
    stop_clip(track_id: number): void;
}

/**
 * Initialize the wasm module. Call this once before using any other functions.
 * Sets up panic hook to log to the browser console via web_sys.
 */
export function hyasynth_init(): void;

/**
 * ADSR envelope node type.
 */
export function node_adsr_env(): number;

/**
 * Audio player node type.
 */
export function node_audio_player(): number;

/**
 * Bandpass filter node type.
 */
export function node_bandpass(): number;

/**
 * Delay node type.
 */
export function node_delay(): number;

/**
 * Gain node type.
 */
export function node_gain(): number;

/**
 * Highpass filter node type.
 */
export function node_highpass(): number;

/**
 * LFO node type.
 */
export function node_lfo(): number;

/**
 * Lowpass filter node type.
 */
export function node_lowpass(): number;

/**
 * Mixer node type.
 */
export function node_mixer(): number;

/**
 * Notch filter node type.
 */
export function node_notch(): number;

/**
 * Output node type.
 */
export function node_output(): number;

/**
 * Pan node type.
 */
export function node_pan(): number;

/**
 * Phase distortion oscillator node type.
 */
export function node_phase_osc(): number;

/**
 * Reverb node type.
 */
export function node_reverb(): number;

/**
 * Saw oscillator node type.
 */
export function node_saw_osc(): number;

/**
 * Sine oscillator node type.
 */
export function node_sine_osc(): number;

/**
 * Square oscillator node type.
 */
export function node_square_osc(): number;

/**
 * Transport node type.
 */
export function node_transport(): number;

/**
 * Triangle oscillator node type.
 */
export function node_triangle_osc(): number;

/**
 * Algorithm parameter ID (phase oscillator).
 */
export function param_algorithm(): number;

/**
 * Attack parameter ID.
 */
export function param_attack(): number;

/**
 * Attack curve parameter ID.
 */
export function param_attack_curve(): number;

/**
 * Cutoff parameter ID.
 */
export function param_cutoff(): number;

/**
 * Damping parameter ID.
 */
export function param_damping(): number;

/**
 * Decay parameter ID.
 */
export function param_decay(): number;

/**
 * Decay curve parameter ID.
 */
export function param_decay_curve(): number;

/**
 * Depth parameter ID.
 */
export function param_depth(): number;

/**
 * Detune parameter ID.
 */
export function param_detune(): number;

/**
 * Feedback parameter ID.
 */
export function param_feedback(): number;

/**
 * Formant parameter ID (phase oscillator).
 */
export function param_formant(): number;

/**
 * Frequency parameter ID.
 */
export function param_freq(): number;

/**
 * Gain parameter ID.
 */
export function param_gain(): number;

/**
 * Key tracking parameter ID.
 */
export function param_key_tracking(): number;

/**
 * Mix parameter ID.
 */
export function param_mix(): number;

/**
 * Mode parameter ID.
 */
export function param_mode(): number;

/**
 * Oscillator key tracking parameter ID.
 */
export function param_osc_key_tracking(): number;

/**
 * Pan parameter ID.
 */
export function param_pan(): number;

/**
 * Phase parameter ID.
 */
export function param_phase(): number;

/**
 * Pitch offset parameter ID (phase oscillator).
 */
export function param_pitch_offset(): number;

/**
 * Pitch ratio denominator parameter ID (phase oscillator).
 */
export function param_pitch_ratio_den(): number;

/**
 * Pitch ratio numerator parameter ID (phase oscillator).
 */
export function param_pitch_ratio_num(): number;

/**
 * Phase oscillator detune parameter ID.
 */
export function param_po_detune(): number;

/**
 * Phase oscillator feedback parameter ID.
 */
export function param_po_feedback(): number;

/**
 * Phase oscillator key tracking parameter ID.
 */
export function param_po_key_tracking(): number;

/**
 * Pulse width parameter ID.
 */
export function param_pulse_width(): number;

/**
 * Rate parameter ID.
 */
export function param_rate(): number;

/**
 * Release parameter ID.
 */
export function param_release(): number;

/**
 * Release curve parameter ID.
 */
export function param_release_curve(): number;

/**
 * Resonance parameter ID.
 */
export function param_resonance(): number;

/**
 * Reverb decay parameter ID.
 */
export function param_reverb_decay(): number;

/**
 * Shape parameter ID (phase oscillator).
 */
export function param_shape(): number;

/**
 * Stereo detune toggle parameter ID (phase oscillator).
 */
export function param_stereo_detune(): number;

/**
 * Sustain parameter ID.
 */
export function param_sustain(): number;

/**
 * Time parameter ID.
 */
export function param_time(): number;

/**
 * Waveform parameter ID.
 */
export function param_waveform(): number;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_hyasynthconfig_free: (a: number, b: number) => void;
    readonly __wbg_get_hyasynthconfig_max_block_size: (a: number) => number;
    readonly __wbg_set_hyasynthconfig_max_block_size: (a: number, b: number) => void;
    readonly __wbg_get_hyasynthconfig_max_voices: (a: number) => number;
    readonly __wbg_set_hyasynthconfig_max_voices: (a: number, b: number) => void;
    readonly __wbg_get_hyasynthconfig_sample_rate: (a: number) => number;
    readonly __wbg_set_hyasynthconfig_sample_rate: (a: number, b: number) => void;
    readonly hyasynthconfig_new: () => number;
    readonly hyasynthconfig_with_values: (a: number, b: number, c: number) => number;
    readonly __wbg_hyasynthreadback_free: (a: number, b: number) => void;
    readonly __wbg_get_hyasynthreadback_sample_position: (a: number) => bigint;
    readonly __wbg_set_hyasynthreadback_sample_position: (a: number, b: bigint) => void;
    readonly __wbg_get_hyasynthreadback_beat_position: (a: number) => number;
    readonly __wbg_set_hyasynthreadback_beat_position: (a: number, b: number) => void;
    readonly __wbg_get_hyasynthreadback_cpu_load: (a: number) => number;
    readonly __wbg_set_hyasynthreadback_cpu_load: (a: number, b: number) => void;
    readonly __wbg_get_hyasynthreadback_active_voices: (a: number) => number;
    readonly __wbg_set_hyasynthreadback_active_voices: (a: number, b: number) => void;
    readonly __wbg_get_hyasynthreadback_peak_left: (a: number) => number;
    readonly __wbg_set_hyasynthreadback_peak_left: (a: number, b: number) => void;
    readonly __wbg_get_hyasynthreadback_peak_right: (a: number) => number;
    readonly __wbg_set_hyasynthreadback_peak_right: (a: number, b: number) => void;
    readonly __wbg_get_hyasynthreadback_running: (a: number) => number;
    readonly __wbg_set_hyasynthreadback_running: (a: number, b: number) => void;
    readonly __wbg_hyasynthregistry_free: (a: number, b: number) => void;
    readonly hyasynthregistry_new: () => number;
    readonly hyasynthregistry_count: (a: number) => number;
    readonly __wbg_hyasynthsession_free: (a: number, b: number) => void;
    readonly hyasynthsession_new: (a: number, b: number) => number;
    readonly hyasynthsession_new_with_config: (a: number, b: number, c: number) => number;
    readonly hyasynthsession_create_engine: (a: number) => number;
    readonly hyasynthsession_add_node: (a: number, b: number, c: number, d: number) => number;
    readonly hyasynthsession_remove_node: (a: number, b: number) => void;
    readonly hyasynthsession_connect: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly hyasynthsession_disconnect: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly hyasynthsession_set_output: (a: number, b: number) => void;
    readonly hyasynthsession_clear_graph: (a: number) => void;
    readonly hyasynthsession_set_param: (a: number, b: number, c: number, d: number) => void;
    readonly hyasynthsession_begin_gesture: (a: number, b: number, c: number) => void;
    readonly hyasynthsession_end_gesture: (a: number, b: number, c: number) => void;
    readonly hyasynthsession_play: (a: number) => void;
    readonly hyasynthsession_stop: (a: number) => void;
    readonly hyasynthsession_set_tempo: (a: number, b: number) => void;
    readonly hyasynthsession_seek: (a: number, b: number) => void;
    readonly hyasynthsession_is_playing: (a: number) => number;
    readonly hyasynthsession_get_tempo: (a: number) => number;
    readonly hyasynthsession_note_on: (a: number, b: number, c: number) => void;
    readonly hyasynthsession_note_off: (a: number, b: number) => void;
    readonly hyasynthsession_get_readback: (a: number) => number;
    readonly hyasynthsession_node_count: (a: number) => number;
    readonly hyasynthsession_get_output_node: (a: number) => number;
    readonly hyasynthsession_create_clip: (a: number, b: number, c: number, d: number) => number;
    readonly hyasynthsession_delete_clip: (a: number, b: number) => void;
    readonly hyasynthsession_add_note_to_clip: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly hyasynthsession_clear_clip: (a: number, b: number) => void;
    readonly hyasynthsession_get_clip_note_count: (a: number, b: number) => number;
    readonly hyasynthsession_create_track: (a: number, b: number, c: number) => number;
    readonly hyasynthsession_delete_track: (a: number, b: number) => void;
    readonly hyasynthsession_set_track_volume: (a: number, b: number, c: number) => void;
    readonly hyasynthsession_set_track_pan: (a: number, b: number, c: number) => void;
    readonly hyasynthsession_set_track_mute: (a: number, b: number, c: number) => void;
    readonly hyasynthsession_set_track_solo: (a: number, b: number, c: number) => void;
    readonly hyasynthsession_set_track_target: (a: number, b: number, c: number) => void;
    readonly hyasynthsession_get_track_count: (a: number) => number;
    readonly hyasynthsession_create_scene: (a: number, b: number, c: number) => number;
    readonly hyasynthsession_delete_scene: (a: number, b: number) => void;
    readonly hyasynthsession_launch_scene: (a: number, b: number) => void;
    readonly hyasynthsession_launch_clip: (a: number, b: number, c: number) => void;
    readonly hyasynthsession_stop_clip: (a: number, b: number) => void;
    readonly hyasynthsession_stop_all_clips: (a: number) => void;
    readonly hyasynthsession_get_scene_count: (a: number) => number;
    readonly hyasynthsession_set_clip_slot: (a: number, b: number, c: number, d: number) => void;
    readonly hyasynthsession_add_mod_route: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly hyasynthsession_remove_mod_route: (a: number, b: number) => void;
    readonly hyasynthsession_set_mod_depth: (a: number, b: number, c: number) => void;
    readonly hyasynthsession_schedule_clip: (a: number, b: number, c: number, d: number) => void;
    readonly hyasynthsession_remove_clip_placement: (a: number, b: number, c: number) => void;
    readonly __wbg_hyasynthengine_free: (a: number, b: number) => void;
    readonly hyasynthengine_process_commands: (a: number) => number;
    readonly hyasynthengine_render: (a: number, b: number, c: number, d: number, e: any) => void;
    readonly hyasynthengine_compile_graph: (a: number, b: number, c: number, d: number) => number;
    readonly hyasynthengine_prepare: (a: number, b: number) => void;
    readonly hyasynthengine_reset: (a: number) => void;
    readonly hyasynthengine_is_playing: (a: number) => number;
    readonly hyasynthengine_get_tempo: (a: number) => number;
    readonly hyasynthengine_get_active_voices: (a: number) => number;
    readonly hyasynthengine_update_position: (a: number, b: bigint) => void;
    readonly hyasynthengine_update_beat_position: (a: number, b: number) => void;
    readonly node_sine_osc: () => number;
    readonly node_saw_osc: () => number;
    readonly node_square_osc: () => number;
    readonly node_triangle_osc: () => number;
    readonly node_adsr_env: () => number;
    readonly node_gain: () => number;
    readonly node_pan: () => number;
    readonly node_output: () => number;
    readonly node_lowpass: () => number;
    readonly node_highpass: () => number;
    readonly node_bandpass: () => number;
    readonly node_notch: () => number;
    readonly node_lfo: () => number;
    readonly node_transport: () => number;
    readonly node_delay: () => number;
    readonly node_reverb: () => number;
    readonly node_mixer: () => number;
    readonly node_audio_player: () => number;
    readonly node_phase_osc: () => number;
    readonly param_attack: () => number;
    readonly param_pitch_ratio_den: () => number;
    readonly param_pitch_offset: () => number;
    readonly param_stereo_detune: () => number;
    readonly param_po_detune: () => number;
    readonly hyasynth_init: () => void;
    readonly param_detune: () => number;
    readonly param_freq: () => number;
    readonly param_decay: () => number;
    readonly param_sustain: () => number;
    readonly param_release: () => number;
    readonly param_gain: () => number;
    readonly param_pan: () => number;
    readonly param_cutoff: () => number;
    readonly param_resonance: () => number;
    readonly param_rate: () => number;
    readonly param_mode: () => number;
    readonly param_depth: () => number;
    readonly param_time: () => number;
    readonly param_feedback: () => number;
    readonly param_mix: () => number;
    readonly param_reverb_decay: () => number;
    readonly param_damping: () => number;
    readonly param_attack_curve: () => number;
    readonly param_decay_curve: () => number;
    readonly param_key_tracking: () => number;
    readonly param_phase: () => number;
    readonly param_pulse_width: () => number;
    readonly param_waveform: () => number;
    readonly param_osc_key_tracking: () => number;
    readonly param_shape: () => number;
    readonly param_algorithm: () => number;
    readonly param_po_feedback: () => number;
    readonly param_formant: () => number;
    readonly param_po_key_tracking: () => number;
    readonly param_pitch_ratio_num: () => number;
    readonly param_release_curve: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
