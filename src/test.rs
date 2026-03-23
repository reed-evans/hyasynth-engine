use crate::bridge::create_bridge;
use crate::engine::Engine;
use crate::execution_plan::ExecutionPlan;
use crate::graph::Graph;
use crate::node_factory::NodeRegistry;
use crate::nodes::node_types;
use crate::nodes::params;
use crate::nodes::register_standard_nodes;
use crate::plan_handoff::PlanHandoff;
use crate::scheduler::Scheduler;
use crate::state::Command;
use crate::state::Session;
use crate::voice_allocator::VoiceAllocator;

pub const max_block_size: usize = 512;
pub const max_voices: usize = 16;
pub const sample_rate: f64 = 48_000.0;

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
    session_handle.send(Command::Connect {
        source_node: osc,
        source_port: 0,
        dest_node: env,
        dest_port: 0,
    });
    session_handle.send(Command::Connect {
        source_node: env,
        source_port: 0,
        dest_node: out,
        dest_port: 0,
    });
    session_handle.send(Command::SetOutputNode { node_id: out });

    // --------------------------------
    // Setting parameters (session_set_param())
    // --------------------------------
    session_handle.set_param(env, params::ATTACK, 0.01);
    session_handle.set_param(env, params::DECAY, 0.1);
    session_handle.set_param(env, params::SUSTAIN, 0.8);
    session_handle.set_param(env, params::RELEASE, 0.1);

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
    let total_frames = max_block_size * 8 as usize;
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
                out_right[offset..offset + chunk_frames]
                    .copy_from_slice(&output[chunk_frames..chunk_frames * 2]);
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
        // offset = 0 -> 512 -> 1024 -> 1536 -> | note off| -> 2048 -> 2560
        // if offset == max_block_size * 3 {
        //     session_handle.note_off(60);
        // }

        if offset == max_block_size * 3 {
            session_handle.note_on(71, 0.8);
        }
    }

    // println!("out_left: {:?}", out_left);
    println!("{:?}", out_right);
}

/// Diagnostic test: verify that per-voice oscillator instances have independent state.
/// Plays two notes simultaneously and examines per-voice buffers directly.
pub fn voice_isolation_test() {
    println!("=== Voice Isolation Diagnostic ===\n");

    // --- Setup ---
    let session = Session::new("Voice Isolation Test");
    let mut graph = Graph::new(max_block_size, max_voices);
    graph.prepare(sample_rate);
    let voices = VoiceAllocator::new(max_voices);
    let engine = Engine::new(graph, voices);
    let (mut session_handle, mut engine_handle) = create_bridge(session, engine);

    let mut registry = NodeRegistry::new();
    register_standard_nodes(&mut registry);

    let mut scheduler = Scheduler::new(sample_rate);
    let mut handoff = PlanHandoff::new(
        ExecutionPlan::new(sample_rate),
        ExecutionPlan::new(sample_rate),
    );

    // Simple graph: SineOsc -> Output (no envelope, to isolate oscillator behavior)
    let osc = session_handle.add_node(node_types::SINE_OSC, 0.0, 0.0);
    let out = session_handle.add_node(node_types::OUTPUT, 0.0, 0.0);
    session_handle.send(Command::Connect {
        source_node: osc,
        source_port: 0,
        dest_node: out,
        dest_port: 0,
    });
    session_handle.send(Command::SetOutputNode { node_id: out });

    // Compile
    let graph_def = session_handle.session().graph.clone();
    match crate::compile::compile(&graph_def, &registry, max_block_size, max_voices) {
        Ok(mut g) => {
            g.prepare(sample_rate);
            engine_handle.swap_graph(g);
        }
        Err(e) => {
            println!("Compile error: {:?}", e);
            return;
        }
    }

    // --- Play two notes simultaneously ---
    session_handle.note_on(60, 0.8); // C4 = 261.63 Hz
    session_handle.note_on(76, 0.8); // E5 = 659.26 Hz

    // Process multiple blocks and accumulate per-voice data for better frequency resolution
    let frames = max_block_size;
    let num_blocks = 8;
    let total_frames = frames * num_blocks;
    let mut voice0_accum: Vec<f32> = Vec::with_capacity(total_frames);
    let mut voice1_accum: Vec<f32> = Vec::with_capacity(total_frames);

    let graph = engine_handle.engine().graph();
    let osc_idx = *graph.id_to_index.get(&osc).expect("osc not in graph");
    assert!(graph.buffers[osc_idx].is_per_voice, "Oscillator buffer should be per-voice");
    drop(graph);

    for _ in 0..num_blocks {
        scheduler.compile_block(&mut handoff, frames, &[]);
        engine_handle.process_commands();
        let plan = handoff.read_plan();
        engine_handle.process_plan(plan);

        let graph = engine_handle.engine().graph();
        let buf = &graph.buffers[osc_idx];
        let voice_size = buf.channels * frames;
        voice0_accum.extend_from_slice(&buf.data[0..voice_size]);
        voice1_accum.extend_from_slice(&buf.data[voice_size..2 * voice_size]);
    }

    let voice0_data = &voice0_accum;
    let voice1_data = &voice1_accum;

    // Print first 10 samples of each voice
    println!("Voice 0 (note 60, C4) first 10 samples:");
    for i in 0..10 {
        print!("  [{:3}] {:.6}", i, voice0_data[i]);
    }
    println!("\n");

    println!("Voice 1 (note 76, E5) first 10 samples:");
    for i in 0..10 {
        print!("  [{:3}] {:.6}", i, voice1_data[i]);
    }
    println!("\n");

    // Check if samples differ (they must, since the notes are different)
    let samples_differ = voice0_data[5] != voice1_data[5];
    println!("Samples at index 5 differ: {} (voice0={:.6}, voice1={:.6})",
        samples_differ, voice0_data[5], voice1_data[5]);

    // Estimate frequency via zero-crossing count
    let zc0 = voice0_data.windows(2).filter(|w| w[0] * w[1] < 0.0).count();
    let zc1 = voice1_data.windows(2).filter(|w| w[0] * w[1] < 0.0).count();
    let freq0 = (zc0 as f64 / 2.0) * sample_rate / total_frames as f64;
    let freq1 = (zc1 as f64 / 2.0) * sample_rate / total_frames as f64;

    println!("\nVoice 0 zero-crossings: {} -> estimated freq: {:.1} Hz (expected ~261.6 Hz)", zc0, freq0);
    println!("Voice 1 zero-crossings: {} -> estimated freq: {:.1} Hz (expected ~659.3 Hz)", zc1, freq1);

    // Check remaining voice slots are silent (check the last block's buffer)
    let graph = engine_handle.engine().graph();
    let buf = &graph.buffers[osc_idx];
    let voice_size = buf.channels * frames;
    let mut silent_voices = 0;
    for v in 2..max_voices {
        let start = v * voice_size;
        let end = start + voice_size;
        if buf.data[start..end].iter().all(|&s| s == 0.0) {
            silent_voices += 1;
        }
    }
    println!("\nInactive voice slots (should be {}): {} are silent",
        max_voices - 2, silent_voices);

    // Verdict
    let freq0_ok = (freq0 - 261.6).abs() < 5.0;
    let freq1_ok = (freq1 - 659.3).abs() < 5.0;
    println!("\n--- Result ---");
    if samples_differ && freq0_ok && freq1_ok && silent_voices == max_voices - 2 {
        println!("PASS: Per-voice isolation is working correctly.");
        println!("Each voice has independent phase and frequency.");
    } else {
        println!("FAIL: Per-voice isolation issue detected!");
        if !samples_differ { println!("  - Voices produce identical samples"); }
        if !freq0_ok { println!("  - Voice 0 frequency mismatch"); }
        if !freq1_ok { println!("  - Voice 1 frequency mismatch"); }
    }
}
