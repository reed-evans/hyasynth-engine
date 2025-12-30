// src/engine_controller.rs
//
// Engine-side command processor.
//
// This module handles commands received from the UI thread and applies
// them to the real-time engine. It manages graph recompilation, parameter
// updates, transport control, and audio pool synchronization.

use crate::{
    bridge::EngineHandle,
    clip_playback::ClipPlayback,
    compile::compile,
    engine::Engine,
    event::MusicalEvent,
    node_factory::NodeRegistry,
    nodes::SharedAudioData,
    scheduler::Scheduler,
    state::{Command, Session},
};

/// Controller that manages the engine and processes UI commands.
///
/// This runs on the audio thread and coordinates between:
/// - Engine (real-time audio processing)
/// - Scheduler (musical time -> sample time)
/// - ClipPlayback (clip -> events)
/// - Commands from UI
pub struct EngineController {
    /// The real-time audio engine.
    engine: Engine,
    
    /// Musical time scheduler.
    scheduler: Scheduler,
    
    /// Clip playback engine.
    clip_playback: ClipPlayback,
    
    /// Node registry for graph compilation.
    node_registry: NodeRegistry,
    
    /// Reference to the UI session (read-only).
    /// The UI owns this, we just read from it.
    session: Session,
    
    /// Scratch buffer for musical events.
    event_buffer: Vec<MusicalEvent>,
}

impl EngineController {
    pub fn new(
        engine: Engine,
        scheduler: Scheduler,
        node_registry: NodeRegistry,
        session: Session,
        sample_rate: f64,
    ) -> Self {
        let clip_playback = ClipPlayback::new(sample_rate);
        
        Self {
            engine,
            scheduler,
            clip_playback,
            node_registry,
            session,
            event_buffer: Vec::with_capacity(128),
        }
    }

    /// Process pending commands from the UI.
    ///
    /// Call this at the start of each audio block (before processing).
    pub fn process_commands(&mut self, engine_handle: &EngineHandle, session: &Session) {
        for command in engine_handle.drain_commands() {
            self.process_command(command, session);
        }
    }

    /// Process a single command.
    fn process_command(&mut self, command: Command, session: &Session) {
        match command {
            // ═════════════════════════════════════════════════════════════
            // Graph Compilation
            // ═════════════════════════════════════════════════════════════
            Command::RecompileGraph => {
                self.recompile_graph(session);
            }

            // ═════════════════════════════════════════════════════════════
            // Transport Control
            // ═════════════════════════════════════════════════════════════
            Command::Play => {
                // Engine starts processing on next block
            }

            Command::Stop => {
                self.engine.reset();
                self.clip_playback.stop_all();
            }

            Command::SetTempo { bpm } => {
                self.scheduler.set_bpm(bpm);
            }

            Command::Seek { beat } => {
                // TODO: Implement seek
                let _ = beat;
            }

            // ═════════════════════════════════════════════════════════════
            // Real-time Note Events
            // ═════════════════════════════════════════════════════════════
            Command::NoteOn { note, velocity } => {
                // These are handled by adding to the event stream
                // The scheduler will process them in the next block
                // For now, we could add them to a realtime event queue
                let _ = (note, velocity);
            }

            Command::NoteOff { note } => {
                let _ = note;
            }

            // ═════════════════════════════════════════════════════════════
            // Parameter Updates (already handled by graph)
            // ═════════════════════════════════════════════════════════════
            Command::SetParam {
                node_id,
                param_id,
                value,
            } => {
                // Already routed through engine in real-time
                // This is a confirmation from UI side
                let _ = (node_id, param_id, value);
            }

            // ═════════════════════════════════════════════════════════════
            // Track Parameter Sync
            // ═════════════════════════════════════════════════════════════
            Command::SyncTrackParams { track_id } => {
                // Track params are synced via SetParam commands that follow
                let _ = track_id;
            }

            Command::SyncAllTrackParams => {
                // Same as above
            }

            // ═════════════════════════════════════════════════════════════
            // Clip/Track/Scene Changes (require recompilation)
            // ═════════════════════════════════════════════════════════════
            Command::CreateTrack { .. }
            | Command::DeleteTrack { .. }
            | Command::SetTrackTarget { .. } => {
                // These are followed by RecompileGraph command
            }

            // ═════════════════════════════════════════════════════════════
            // Clip Playback Control
            // ═════════════════════════════════════════════════════════════
            Command::LaunchScene { scene_index: _ } => {
                let current_beat = self.scheduler.beat_position();
                self.clip_playback
                    .sync_with_arrangement(&session.arrangement, current_beat);
            }

            Command::LaunchClip { track_id, clip_id } => {
                let current_beat = self.scheduler.beat_position();
                self.clip_playback.start_clip(clip_id, track_id, current_beat);
            }

            Command::StopClip { track_id } => {
                let current_beat = self.scheduler.beat_position();
                self.clip_playback.stop_track(track_id, current_beat);
            }

            Command::StopAllClips => {
                self.clip_playback.stop_all();
            }

            // All other commands don't require engine-side action
            _ => {}
        }
    }

    /// Recompile the graph from the session's GraphDef + Arrangement.
    fn recompile_graph(&mut self, session: &Session) {
        // 1. Build the runtime graph (user nodes + track mixer nodes + master bus)
        let runtime_graph_def = session.build_runtime_graph();

        // 2. Compile GraphDef -> Graph
        match compile(
            &runtime_graph_def,
            &self.node_registry,
            session.max_block_size,
            session.max_voices,
        ) {
            Ok(mut new_graph) => {
                // 3. Load audio pool data into audio player nodes
                for audio_entry in session.arrangement.audio_pool.iter() {
                    let shared_data = SharedAudioData::from_pool_entry(audio_entry);
                    new_graph.load_audio_to_all(shared_data);
                }
                
                // 4. Replace the engine's graph
                self.engine.set_graph(new_graph);
            }
            Err(e) => {
                eprintln!("Graph compilation failed: {:?}", e);
            }
        }
    }

    /// Process one audio block.
    ///
    /// This is the main entry point called from the audio callback.
    pub fn process_block(
        &mut self,
        output: &mut [f32],
        session: &Session,
        engine_handle: &EngineHandle,
    ) {
        // 1. Process commands from UI
        self.process_commands(engine_handle, session);

        // 2. Sync clip playback state with arrangement
        let current_beat = self.scheduler.beat_position();
        self.clip_playback
            .sync_with_arrangement(&session.arrangement, current_beat);

        // 3. Generate events from playing clips
        let block_frames = output.len() / 2; // Assuming stereo
        let start_beat = current_beat;
        let end_beat = start_beat + self.beat_duration_for_frames(block_frames);
        
        let clip_events = self.clip_playback.generate_events(
            &session.arrangement,
            start_beat,
            end_beat,
            session.transport.bpm,
        );

        // 4. Combine with any real-time events (MIDI input, automation)
        self.event_buffer.clear();
        self.event_buffer.extend_from_slice(clip_events);
        // TODO: Add real-time MIDI events here

        // 5. Compile musical events to execution plan
        // TODO: Get plan handoff from somewhere
        // self.scheduler.compile_block(&mut plan_handoff, block_frames, &self.event_buffer);

        // 6. Execute the plan on the engine
        // self.engine.process_plan(&plan);

        // 7. Copy engine output to audio callback buffer
        // if let Some(engine_output) = self.engine.output_buffer(block_frames) {
        //     output.copy_from_slice(engine_output);
        // }

        // 8. Update readback for UI
        engine_handle.update_sample_position(0); // TODO: actual position
        engine_handle.update_active_voices(self.engine.active_voices());
    }

    /// Calculate beat duration for a number of frames.
    fn beat_duration_for_frames(&self, frames: usize) -> f64 {
        let sample_rate = 48000.0; // TODO: get from scheduler
        let seconds = frames as f64 / sample_rate;
        let bpm = self.session.transport.bpm;
        seconds * (bpm / 60.0)
    }

    /// Get a reference to the engine.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Get a mutable reference to the engine.
    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    /// Get a reference to the scheduler.
    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    /// Get a mutable reference to the scheduler.
    pub fn scheduler_mut(&mut self) -> &mut Scheduler {
        &mut self.scheduler
    }
}

/// Helper to create a complete engine setup.
pub fn create_engine_controller(
    session: Session,
    node_registry: NodeRegistry,
    sample_rate: f64,
    max_block_size: usize,
) -> EngineController {
    use crate::voice_allocator::VoiceAllocator;

    // Create initial graph
    let graph_def = session.build_runtime_graph();
    let graph = compile(&graph_def, &node_registry, max_block_size, session.max_voices)
        .expect("Initial graph compilation failed");

    // Create voice allocator
    let voices = VoiceAllocator::new(session.max_voices);

    // Create engine
    let engine = Engine::new(graph, voices);

    // Create scheduler
    let scheduler = Scheduler::new(sample_rate);

    EngineController::new(engine, scheduler, node_registry, session, sample_rate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::register_standard_nodes;

    #[test]
    fn test_engine_controller_creation() {
        // Create session without tracks (so no mixer nodes needed)
        let session = Session::new("Test");
        let mut registry = NodeRegistry::new();
        register_standard_nodes(&mut registry);

        // Note: This creates a runtime graph with master bus and output nodes
        // If the session has no tracks, it should still compile successfully
        let controller = create_engine_controller(session, registry, 48000.0, 512);
        
        assert_eq!(controller.engine().active_voices(), 0);
    }
    
    #[test]
    fn test_command_processing() {
        let session = Session::new("Test");
        let mut registry = NodeRegistry::new();
        register_standard_nodes(&mut registry);
        
        let mut controller = create_engine_controller(session.clone(), registry, 48000.0, 512);
        
        // Create a mock EngineHandle for testing
        let (_, engine_handle) = crate::bridge::create_bridge(session.clone());
        
        // Process commands (should not panic)
        let session_ref = &session;
        controller.process_commands(&engine_handle, session_ref);
    }
}

