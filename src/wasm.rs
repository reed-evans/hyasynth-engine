//! WebAssembly bindings via wasm-bindgen for browser integration.
//!
//! This module is only compiled when the `web` feature is enabled.
//!
//! # Usage
//!
//! Build with wasm-pack:
//! ```bash
//! wasm-pack build --target web --features web
//! ```
//!
//! # JavaScript Example
//!
//! ```javascript
//! import init, { HyasynthSession, HyasynthEngine } from './hyasynth.js';
//!
//! await init();
//!
//! const session = new HyasynthSession("My Synth");
//! const engine = session.create_engine();
//!
//! // Add nodes
//! const osc = session.add_node(NODE_SINE_OSC, 0, 0);
//! const out = session.add_node(NODE_OUTPUT, 100, 0);
//! session.connect(osc, 0, out, 0);
//! session.set_output(out);
//!
//! // Compile and render in AudioWorklet
//! engine.compile_graph(session, registry, 48000);
//! ```

use wasm_bindgen::prelude::*;

use crate::bridge::{EngineHandle, SessionHandle, create_bridge};
use crate::engine::Engine;
use crate::execution_plan::ExecutionPlan;
use crate::graph::Graph;
use crate::node_factory::NodeRegistry;
use crate::nodes::register_standard_nodes;
use crate::plan_handoff::PlanHandoff;
use crate::scheduler::Scheduler;
use crate::state::{Command, EngineReadback, Session};
use crate::voice_allocator::VoiceAllocator;


// Default audio configuration
const DEFAULT_MAX_BLOCK: usize = 512;
const DEFAULT_MAX_VOICES: usize = 16;
const DEFAULT_SAMPLE_RATE: f64 = 48_000.0;

// ═══════════════════════════════════════════════════════════════════════════
// Initialization
// ═══════════════════════════════════════════════════════════════════════════

/// Initialize the wasm module. Call this once before using any other functions.
/// Sets up panic hooks and console logging.
#[wasm_bindgen]
pub fn hyasynth_init() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).ok();
}

// ═══════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════

/// Configuration for creating a session and engine.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct HyasynthConfig {
    /// Maximum audio block size in frames (e.g., 128, 256, 512).
    pub max_block_size: u32,
    /// Maximum number of simultaneous voices for polyphony.
    pub max_voices: u32,
    /// Sample rate in Hz (e.g., 44100.0, 48000.0).
    pub sample_rate: f64,
}

#[wasm_bindgen]
impl HyasynthConfig {
    /// Create a new configuration with default values.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration with custom values.
    pub fn with_values(max_block_size: u32, max_voices: u32, sample_rate: f64) -> Self {
        Self {
            max_block_size,
            max_voices,
            sample_rate,
        }
    }
}

impl Default for HyasynthConfig {
    fn default() -> Self {
        Self {
            max_block_size: DEFAULT_MAX_BLOCK as u32,
            max_voices: DEFAULT_MAX_VOICES as u32,
            sample_rate: DEFAULT_SAMPLE_RATE,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Readback Data
// ═══════════════════════════════════════════════════════════════════════════

/// Readback data from the engine (for UI meters/displays).
#[wasm_bindgen]
#[derive(Clone, Copy, Default)]
pub struct HyasynthReadback {
    /// Current sample position in the timeline.
    pub sample_position: u64,
    /// Current beat position in the timeline.
    pub beat_position: f64,
    /// CPU load estimate (0.0 - 1.0).
    pub cpu_load: f32,
    /// Number of currently active voices.
    pub active_voices: u32,
    /// Peak level of left channel.
    pub peak_left: f32,
    /// Peak level of right channel.
    pub peak_right: f32,
    /// Whether the engine is running.
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

// ═══════════════════════════════════════════════════════════════════════════
// Node Registry
// ═══════════════════════════════════════════════════════════════════════════

/// Node registry containing all available node types.
#[wasm_bindgen]
pub struct HyasynthRegistry {
    inner: NodeRegistry,
}

#[wasm_bindgen]
impl HyasynthRegistry {
    /// Create a new registry with all standard nodes registered.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let mut registry = NodeRegistry::new();
        register_standard_nodes(&mut registry);
        Self { inner: registry }
    }

    /// Get the number of registered node types.
    pub fn count(&self) -> u32 {
        self.inner.iter().count() as u32
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Session (UI-side handle)
// ═══════════════════════════════════════════════════════════════════════════

/// UI-side session handle for building and controlling the synth.
#[wasm_bindgen]
pub struct HyasynthSession {
    inner: SessionHandle,
    config: HyasynthConfig,
}

#[wasm_bindgen]
impl HyasynthSession {
    /// Create a new session with default configuration.
    #[wasm_bindgen(constructor)]
    pub fn new(name: &str) -> HyasynthSession {
        Self::new_with_config(name, HyasynthConfig::default())
    }

    /// Create a new session with custom configuration.
    pub fn new_with_config(name: &str, config: HyasynthConfig) -> HyasynthSession {
        let session = Session::new(name.to_string());
        let mut graph = Graph::new(config.max_block_size as usize, config.max_voices as usize);
        graph.prepare(config.sample_rate);
        let voices = VoiceAllocator::new(config.max_voices as usize);
        let engine = Engine::new(graph, voices);
        let (session_handle, _engine_handle) = create_bridge(session, engine);

        HyasynthSession {
            inner: session_handle,
            config,
        }
    }

    /// Create an engine handle paired with this session.
    /// The engine processes audio and should be used in an AudioWorklet.
    pub fn create_engine(&self) -> HyasynthEngine {
        let session = Session::new("".to_string());
        let mut graph = Graph::new(
            self.config.max_block_size as usize,
            self.config.max_voices as usize,
        );
        graph.prepare(self.config.sample_rate);
        let voices = VoiceAllocator::new(self.config.max_voices as usize);
        let engine = Engine::new(graph, voices);
        let (_session_handle, engine_handle) = create_bridge(session, engine);

        let scheduler = Scheduler::new(self.config.sample_rate);
        let handoff = PlanHandoff::new(
            ExecutionPlan::new(self.config.sample_rate),
            ExecutionPlan::new(self.config.sample_rate),
        );

        HyasynthEngine {
            inner: engine_handle,
            scheduler,
            handoff,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Graph Mutations
    // ─────────────────────────────────────────────────────────────────────────

    /// Add a node to the graph. Returns the new node's ID.
    pub fn add_node(&mut self, type_id: u32, x: f32, y: f32) -> u32 {
        self.inner.add_node(type_id, x, y)
    }

    /// Remove a node from the graph.
    pub fn remove_node(&mut self, node_id: u32) {
        self.inner.remove_node(node_id);
    }

    /// Connect two nodes.
    pub fn connect(&mut self, source_node: u32, source_port: u32, dest_node: u32, dest_port: u32) {
        self.inner.send(Command::Connect {
            source_node,
            source_port,
            dest_node,
            dest_port,
        });
    }

    /// Disconnect two nodes.
    pub fn disconnect(
        &mut self,
        source_node: u32,
        source_port: u32,
        dest_node: u32,
        dest_port: u32,
    ) {
        self.inner.send(Command::Disconnect {
            source_node,
            source_port,
            dest_node,
            dest_port,
        });
    }

    /// Set the output node.
    pub fn set_output(&mut self, node_id: u32) {
        self.inner.send(Command::SetOutputNode { node_id });
    }

    /// Clear the entire graph.
    pub fn clear_graph(&mut self) {
        self.inner.send(Command::ClearGraph);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Parameters
    // ─────────────────────────────────────────────────────────────────────────

    /// Set a parameter value.
    pub fn set_param(&mut self, node_id: u32, param_id: u32, value: f32) {
        self.inner.set_param(node_id, param_id, value);
    }

    /// Begin a parameter gesture (for automation recording).
    pub fn begin_gesture(&mut self, node_id: u32, param_id: u32) {
        self.inner
            .send(Command::BeginParamGesture { node_id, param_id });
    }

    /// End a parameter gesture.
    pub fn end_gesture(&mut self, node_id: u32, param_id: u32) {
        self.inner
            .send(Command::EndParamGesture { node_id, param_id });
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Transport
    // ─────────────────────────────────────────────────────────────────────────

    /// Start playback.
    pub fn play(&mut self) {
        self.inner.play();
    }

    /// Stop playback.
    pub fn stop(&mut self) {
        self.inner.stop();
    }

    /// Set tempo in BPM.
    pub fn set_tempo(&mut self, bpm: f64) {
        self.inner.send(Command::SetTempo { bpm });
    }

    /// Seek to a position in beats.
    pub fn seek(&mut self, beat: f64) {
        self.inner.send(Command::Seek { beat });
    }

    /// Check if the transport is playing.
    pub fn is_playing(&self) -> bool {
        self.inner.session().transport.playing
    }

    /// Get the current tempo.
    pub fn get_tempo(&self) -> f64 {
        self.inner.session().transport.bpm
    }

    // ─────────────────────────────────────────────────────────────────────────
    // MIDI
    // ─────────────────────────────────────────────────────────────────────────

    /// Send a MIDI note on.
    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.inner.note_on(note, velocity);
    }

    /// Send a MIDI note off.
    pub fn note_off(&mut self, note: u8) {
        self.inner.note_off(note);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Readback
    // ─────────────────────────────────────────────────────────────────────────

    /// Get the current engine readback state.
    pub fn get_readback(&self) -> HyasynthReadback {
        self.inner.readback().into()
    }

    /// Get the number of nodes in the graph.
    pub fn node_count(&self) -> u32 {
        self.inner.session().graph.nodes.len() as u32
    }

    /// Get the output node ID, or u32::MAX if not set.
    pub fn get_output_node(&self) -> u32 {
        self.inner.session().graph.output_node.unwrap_or(u32::MAX)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Clips
    // ─────────────────────────────────────────────────────────────────────────

    /// Create a new clip. Returns the clip ID.
    pub fn create_clip(&mut self, name: &str, length: f64) -> u32 {
        self.inner
            .session_mut()
            .arrangement
            .create_clip(name.to_string(), length)
    }

    /// Delete a clip.
    pub fn delete_clip(&mut self, clip_id: u32) {
        self.inner
            .session_mut()
            .arrangement
            .delete_clip(clip_id);
    }

    /// Add a note to a clip.
    pub fn add_note_to_clip(
        &mut self,
        clip_id: u32,
        start: f64,
        duration: f64,
        note: u8,
        velocity: f32,
    ) {
        use crate::state::NoteDef;
        self.inner
            .session_mut()
            .arrangement
            .add_note_to_clip(clip_id, NoteDef::new(start, duration, note, velocity));
    }

    /// Clear all notes from a clip.
    pub fn clear_clip(&mut self, clip_id: u32) {
        if let Some(clip) = self.inner.session_mut().arrangement.get_clip_mut(clip_id) {
            clip.clear();
        }
    }

    /// Get the number of notes in a clip.
    pub fn get_clip_note_count(&self, clip_id: u32) -> u32 {
        self.inner
            .session()
            .arrangement
            .get_clip(clip_id)
            .map(|c| c.note_count() as u32)
            .unwrap_or(0)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Tracks
    // ─────────────────────────────────────────────────────────────────────────

    /// Create a new track. Returns the track ID.
    pub fn create_track(&mut self, name: &str) -> u32 {
        self.inner
            .session_mut()
            .arrangement
            .create_track(name.to_string())
    }

    /// Delete a track.
    pub fn delete_track(&mut self, track_id: u32) {
        self.inner
            .session_mut()
            .arrangement
            .delete_track(track_id);
    }

    /// Set track volume (0.0 - 1.0).
    pub fn set_track_volume(&mut self, track_id: u32, volume: f32) {
        self.inner
            .session_mut()
            .arrangement
            .set_track_volume(track_id, volume);
    }

    /// Set track pan (-1.0 to 1.0).
    pub fn set_track_pan(&mut self, track_id: u32, pan: f32) {
        self.inner
            .session_mut()
            .arrangement
            .set_track_pan(track_id, pan);
    }

    /// Set track mute.
    pub fn set_track_mute(&mut self, track_id: u32, mute: bool) {
        self.inner
            .session_mut()
            .arrangement
            .set_track_mute(track_id, mute);
    }

    /// Set track solo.
    pub fn set_track_solo(&mut self, track_id: u32, solo: bool) {
        self.inner
            .session_mut()
            .arrangement
            .set_track_solo(track_id, solo);
    }

    /// Set track target node (the node this track sends MIDI to).
    /// Pass u32::MAX to clear the target.
    pub fn set_track_target(&mut self, track_id: u32, node_id: u32) {
        let target = if node_id == u32::MAX {
            None
        } else {
            Some(node_id)
        };
        self.inner
            .session_mut()
            .arrangement
            .set_track_target(track_id, target);
    }

    /// Get the number of tracks.
    pub fn get_track_count(&self) -> u32 {
        self.inner.session().arrangement.tracks.len() as u32
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Scenes
    // ─────────────────────────────────────────────────────────────────────────

    /// Create a new scene. Returns the scene ID.
    pub fn create_scene(&mut self, name: &str) -> u32 {
        self.inner
            .session_mut()
            .arrangement
            .create_scene(name.to_string())
    }

    /// Delete a scene.
    pub fn delete_scene(&mut self, scene_id: u32) {
        self.inner
            .session_mut()
            .arrangement
            .delete_scene(scene_id);
    }

    /// Launch a scene (trigger all clips in that row).
    pub fn launch_scene(&mut self, scene_index: u32) {
        self.inner
            .session_mut()
            .arrangement
            .launch_scene(scene_index as usize);
    }

    /// Launch a single clip on a track.
    pub fn launch_clip(&mut self, track_id: u32, clip_id: u32) {
        self.inner
            .session_mut()
            .arrangement
            .launch_clip(track_id, clip_id);
    }

    /// Stop a clip on a track.
    pub fn stop_clip(&mut self, track_id: u32) {
        self.inner
            .session_mut()
            .arrangement
            .stop_clip(track_id);
    }

    /// Stop all clips.
    pub fn stop_all_clips(&mut self) {
        self.inner.session_mut().arrangement.stop_all();
    }

    /// Get the number of scenes.
    pub fn get_scene_count(&self) -> u32 {
        self.inner.session().arrangement.scenes.len() as u32
    }

    /// Assign a clip to a track's clip slot.
    /// Pass u32::MAX for clip_id to clear the slot.
    pub fn set_clip_slot(&mut self, track_id: u32, scene_index: u32, clip_id: u32) {
        let clip = if clip_id == u32::MAX {
            None
        } else {
            Some(clip_id)
        };
        self.inner
            .session_mut()
            .arrangement
            .set_clip_slot(track_id, scene_index as usize, clip);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Timeline
    // ─────────────────────────────────────────────────────────────────────────

    /// Schedule a clip on the timeline.
    pub fn schedule_clip(&mut self, track_id: u32, clip_id: u32, start_beat: f64) {
        self.inner
            .session_mut()
            .arrangement
            .schedule_clip(track_id, clip_id, start_beat);
    }

    /// Remove a clip placement from the timeline.
    pub fn remove_clip_placement(&mut self, track_id: u32, start_beat: f64) {
        self.inner
            .session_mut()
            .arrangement
            .remove_clip_placement(track_id, start_beat);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Engine (Audio-side handle)
// ═══════════════════════════════════════════════════════════════════════════

/// Audio-side engine handle for rendering audio.
/// Use this in an AudioWorklet for real-time processing.
#[wasm_bindgen]
pub struct HyasynthEngine {
    inner: EngineHandle,
    scheduler: Scheduler,
    handoff: PlanHandoff,
}

#[wasm_bindgen]
impl HyasynthEngine {
    /// Process all pending commands from the UI thread.
    /// Call this at the start of each audio render callback.
    /// Returns true if any command requires graph recompilation.
    pub fn process_commands(&mut self) -> bool {
        self.inner.process_commands()
    }

    /// Render audio frames to the provided output buffer (interleaved stereo).
    /// Output format: [L0, R0, L1, R1, L2, R2, ...]
    ///
    /// The output slice must have length >= frames * 2.
    pub fn render(&mut self, frames: u32, output: &mut [f32]) {
        let total_frames = frames as usize;
        let max_block = self.inner.engine().graph().max_block;

        if output.len() < total_frames * 2 {
            output.fill(0.0);
            return;
        }

        let mut offset = 0;
        while offset < total_frames {
            let chunk_frames = (total_frames - offset).min(max_block);

            // Compile execution plan
            self.scheduler.compile_block(&mut self.handoff, chunk_frames, &[]);

            // Process pending commands
            self.inner.process_commands();

            // Read and process the plan
            let plan = self.handoff.read_plan();
            self.inner.process_plan(plan);

            let out_chunk = &mut output[offset * 2..(offset + chunk_frames) * 2];

            // Convert planar to interleaved
            if let Some(engine_output) = self.inner.output_buffer(chunk_frames) {
                if engine_output.len() >= chunk_frames * 2 {
                    for i in 0..chunk_frames {
                        out_chunk[i * 2] = engine_output[i];
                        out_chunk[i * 2 + 1] = engine_output[chunk_frames + i];
                    }
                } else if engine_output.len() >= chunk_frames {
                    for i in 0..chunk_frames {
                        out_chunk[i * 2] = engine_output[i];
                        out_chunk[i * 2 + 1] = engine_output[i];
                    }
                } else {
                    out_chunk.fill(0.0);
                }
            } else {
                out_chunk.fill(0.0);
            }

            offset += chunk_frames;
        }

        // Sync readback
        self.inner.update_sample_position(self.scheduler.sample_position());
        self.inner.update_beat_position(self.scheduler.beat_position());
        self.inner.sync_readback();
    }

    /// Compile the session's graph and load it into the engine.
    pub fn compile_graph(
        &mut self,
        session: &HyasynthSession,
        registry: &HyasynthRegistry,
        sample_rate: f64,
    ) -> bool {
        let max_block = self.inner.engine().graph().max_block;
        let max_voices = self.inner.engine().graph().max_voices;
        let graph_def = session.inner.session().graph.clone();

        match crate::compile::compile(&graph_def, &registry.inner, max_block, max_voices) {
            Ok(mut graph) => {
                graph.prepare(sample_rate);
                self.inner.swap_graph(graph);
                true
            }
            Err(e) => {
                log::error!("Error compiling graph: {:?}", e);
                false
            }
        }
    }

    /// Prepare the engine's graph for processing.
    pub fn prepare(&mut self, sample_rate: f64) {
        self.inner.engine_mut().graph_mut().prepare(sample_rate);
    }

    /// Reset the engine state.
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Check if the engine is currently playing.
    pub fn is_playing(&self) -> bool {
        self.inner.is_playing()
    }

    /// Get the current tempo in BPM.
    pub fn get_tempo(&self) -> f64 {
        self.inner.bpm()
    }

    /// Get the number of active voices.
    pub fn get_active_voices(&self) -> u32 {
        self.inner.active_voices() as u32
    }

    /// Update sample position (for external timing sync).
    pub fn update_position(&mut self, position: u64) {
        self.inner.update_sample_position(position);
    }

    /// Update beat position (for external timing sync).
    pub fn update_beat_position(&mut self, position: f64) {
        self.inner.update_beat_position(position);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Node Type Constants
// ═══════════════════════════════════════════════════════════════════════════

/// Sine oscillator node type.
#[wasm_bindgen]
pub fn node_sine_osc() -> u32 {
    crate::nodes::node_types::SINE_OSC
}

/// Saw oscillator node type.
#[wasm_bindgen]
pub fn node_saw_osc() -> u32 {
    crate::nodes::node_types::SAW_OSC
}

/// Square oscillator node type.
#[wasm_bindgen]
pub fn node_square_osc() -> u32 {
    crate::nodes::node_types::SQUARE_OSC
}

/// Triangle oscillator node type.
#[wasm_bindgen]
pub fn node_triangle_osc() -> u32 {
    crate::nodes::node_types::TRIANGLE_OSC
}

/// ADSR envelope node type.
#[wasm_bindgen]
pub fn node_adsr_env() -> u32 {
    crate::nodes::node_types::ADSR_ENV
}

/// Gain node type.
#[wasm_bindgen]
pub fn node_gain() -> u32 {
    crate::nodes::node_types::GAIN
}

/// Pan node type.
#[wasm_bindgen]
pub fn node_pan() -> u32 {
    crate::nodes::node_types::PAN
}

/// Output node type.
#[wasm_bindgen]
pub fn node_output() -> u32 {
    crate::nodes::node_types::OUTPUT
}

/// Lowpass filter node type.
#[wasm_bindgen]
pub fn node_lowpass() -> u32 {
    crate::nodes::node_types::LOWPASS
}

/// Highpass filter node type.
#[wasm_bindgen]
pub fn node_highpass() -> u32 {
    crate::nodes::node_types::HIGHPASS
}

/// Bandpass filter node type.
#[wasm_bindgen]
pub fn node_bandpass() -> u32 {
    crate::nodes::node_types::BANDPASS
}

/// Notch filter node type.
#[wasm_bindgen]
pub fn node_notch() -> u32 {
    crate::nodes::node_types::NOTCH
}

/// LFO node type.
#[wasm_bindgen]
pub fn node_lfo() -> u32 {
    crate::nodes::node_types::LFO
}

/// Delay node type.
#[wasm_bindgen]
pub fn node_delay() -> u32 {
    crate::nodes::node_types::DELAY
}

/// Reverb node type.
#[wasm_bindgen]
pub fn node_reverb() -> u32 {
    crate::nodes::node_types::REVERB
}

// ═══════════════════════════════════════════════════════════════════════════
// Parameter ID Constants
// ═══════════════════════════════════════════════════════════════════════════

/// Frequency parameter ID.
#[wasm_bindgen]
pub fn param_freq() -> u32 {
    crate::nodes::params::FREQ
}

/// Detune parameter ID.
#[wasm_bindgen]
pub fn param_detune() -> u32 {
    crate::nodes::params::DETUNE
}

/// Attack parameter ID.
#[wasm_bindgen]
pub fn param_attack() -> u32 {
    crate::nodes::params::ATTACK
}

/// Decay parameter ID.
#[wasm_bindgen]
pub fn param_decay() -> u32 {
    crate::nodes::params::DECAY
}

/// Sustain parameter ID.
#[wasm_bindgen]
pub fn param_sustain() -> u32 {
    crate::nodes::params::SUSTAIN
}

/// Release parameter ID.
#[wasm_bindgen]
pub fn param_release() -> u32 {
    crate::nodes::params::RELEASE
}

/// Gain parameter ID.
#[wasm_bindgen]
pub fn param_gain() -> u32 {
    crate::nodes::params::GAIN
}

/// Pan parameter ID.
#[wasm_bindgen]
pub fn param_pan() -> u32 {
    crate::nodes::params::PAN
}

/// Cutoff parameter ID.
#[wasm_bindgen]
pub fn param_cutoff() -> u32 {
    crate::nodes::params::CUTOFF
}

/// Resonance parameter ID.
#[wasm_bindgen]
pub fn param_resonance() -> u32 {
    crate::nodes::params::RESONANCE
}

/// Rate parameter ID.
#[wasm_bindgen]
pub fn param_rate() -> u32 {
    crate::nodes::params::RATE
}

/// Depth parameter ID.
#[wasm_bindgen]
pub fn param_depth() -> u32 {
    crate::nodes::params::DEPTH
}

/// Time parameter ID.
#[wasm_bindgen]
pub fn param_time() -> u32 {
    crate::nodes::params::TIME
}

/// Feedback parameter ID.
#[wasm_bindgen]
pub fn param_feedback() -> u32 {
    crate::nodes::params::FEEDBACK
}

/// Mix parameter ID.
#[wasm_bindgen]
pub fn param_mix() -> u32 {
    crate::nodes::params::MIX
}

/// Damping parameter ID.
#[wasm_bindgen]
pub fn param_damping() -> u32 {
    crate::nodes::params::DAMPING
}
