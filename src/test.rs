use crate::state::Session;
use crate::graph::Graph;
use crate::voice_allocator::VoiceAllocator;
use crate::engine::Engine;
use crate::bridge::{create_bridge};
use crate::scheduler::Scheduler;
use crate::plan_handoff::PlanHandoff;
use crate::execution_plan::ExecutionPlan;
use crate::node_factory::NodeRegistry;
use crate::nodes::register_standard_nodes;
use crate::state::Command;
use crate::nodes::node_types;
use crate::nodes::params;

pub const max_block_size: usize = 512;
pub const max_voices: usize = 16;
pub const sample_rate: f64 = 48000.0;

pub fn end_to_end_test() {
    // --------------------------------
    // Creating session and engine (session_create_with_config())
    // --------------------------------
    let session = Session::new("Test Session");
    let mut graph = Graph::new(max_block_size, max_voices);
    graph.prepare(sample_rate);
    let voices = VoiceAllocator::new(max_voices as usize);
    let engine = Engine::new(graph, voices);

    let (mut session_handle, mut engine_handle) = create_bridge(session, engine);

    let mut scheduler = Scheduler::new(sample_rate);
    let mut handoff = PlanHandoff::new(
        ExecutionPlan::new(sample_rate),
        ExecutionPlan::new(sample_rate),
    );

    // --------------------------------
    // Creating registry (registry_create())
    // --------------------------------
    let mut registry = NodeRegistry::new();
    register_standard_nodes(&mut registry);

    // --------------------------------
    // Creating graph definition (session_add_node(), session_connect(), session_set_output_node())
    // --------------------------------
    let osc = session_handle.add_node(node_types::SINE_OSC, 0.0, 0.0);
    let env = session_handle.add_node(node_types::ADSR_ENV, 0.0, 0.0);
    let out = session_handle.add_node(node_types::OUTPUT, 0.0, 0.0);
    session_handle.send(Command::Connect { source_node: osc, source_port: 0, dest_node: env, dest_port: 0 });
    session_handle.send(Command::Connect { source_node: env, source_port: 0, dest_node: out, dest_port: 0 });
    session_handle.send(Command::SetOutputNode { node_id: out });

    // --------------------------------
    // Setting parameters (session_set_param())
    // --------------------------------
    session_handle.set_param(env, params::ATTACK, 0.01);
    session_handle.set_param(env, params::DECAY, 0.01);
    session_handle.set_param(env, params::SUSTAIN, 0.8);
    session_handle.set_param(env, params::RELEASE, 0.021);

    // --------------------------------
    // Compiling graph (engine_compile_graph())
    // --------------------------------
    // Use the raw session graph instead of build_runtime_graph() which adds
    // track mixer routing that we don't need for this simple test.
    let graph_def = session_handle.session().graph.clone();

    match crate::compile::compile(&graph_def, &registry, max_block_size, max_voices) {
        Ok(mut graph) => {
            graph.prepare(sample_rate);
            engine_handle.swap_graph(graph);
        }
        Err(e) => {
            println!("Error compiling graph: {:?}", e);
            return;
        }
    }

    // --------------------------------
    // Preparing engine (engine_prepare())
    // --------------------------------
    engine_handle.engine_mut().graph_mut().prepare(sample_rate);

    // --------------------------------
    // Starting playback (session_note_on())
    // --------------------------------
    session_handle.note_on(60, 0.8);

    // --------------------------------
    // Rendering audio (engine_render())
    // --------------------------------
    let total_frames = max_block_size * 4 + 512 as usize;
    let mut out_left = vec![0.0; total_frames];
    let mut out_right = vec![0.0; total_frames];

    let mut offset = 0;
    while offset < total_frames {
        let chunk_frames = (total_frames - offset).min(max_block_size);

        // Use the scheduler to compile a proper execution plan
        scheduler.compile_block(
            &mut handoff,
            chunk_frames,
            &[], // No musical events from this path (they come via commands)
        );

        // Process any pending commands (like note_on)
        engine_handle.process_commands();

        // Read the compiled plan and process it
        let plan = handoff.read_plan();
        engine_handle.process_plan(plan);

        // Copy output to provided buffers
        // Note: output buffer is in PLANAR format: [L0..LN, R0..RN]
        if let Some(output) = engine_handle.output_buffer(chunk_frames) {
            if output.len() >= chunk_frames * 2 {
                // Stereo output - planar format: first half is left, second half is right
                out_left[offset..offset + chunk_frames].copy_from_slice(&output[..chunk_frames]);
                out_right[offset..offset + chunk_frames].copy_from_slice(&output[chunk_frames..chunk_frames * 2]);
            } else if output.len() >= chunk_frames {
                // Mono output - copy to both channels
                out_left[offset..offset + chunk_frames].copy_from_slice(&output[..chunk_frames]);
                out_right[offset..offset + chunk_frames].copy_from_slice(&output[..chunk_frames]);
            } else {
                // Not enough output - fill with silence
                out_left[offset..offset + chunk_frames].fill(0.0);
                out_right[offset..offset + chunk_frames].fill(0.0);
            }
        } else {
            // No output buffer - fill with silence
            out_left[offset..offset + chunk_frames].fill(0.0);
            out_right[offset..offset + chunk_frames].fill(0.0);
        }

        offset += chunk_frames;
        // offset = 0 -> 512 -> 1536 -> | note off| -> 2048
        if offset > max_block_size * 2 {
            session_handle.note_off(60);
        }
    }

    // println!("out_left: {:?}", out_left);
    println!("{:?}", out_right);
}