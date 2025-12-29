// src/main.rs

mod audio_buffer;
mod bridge;
mod compile;
mod engine;
mod event;
mod execution_plan;
mod graph;
mod modulation;
mod node;
mod node_factory;
mod nodes;
mod parameter;
mod plan_handoff;
mod scheduler;
mod state;
mod transport;
mod voice;
mod voice_allocator;

use crate::{
    compile::compile,
    engine::Engine,
    execution_plan::ExecutionPlan,
    node_factory::NodeRegistry,
    nodes::{node_types, register_standard_nodes},
    plan_handoff::PlanHandoff,
    scheduler::Scheduler,
    state::GraphDef,
    voice_allocator::VoiceAllocator,
};

fn main() {
    let sample_rate = 48_000.0;
    let block_frames = 256;
    let max_block = 512;
    let max_voices = 8;

    // --------------------------------
    // Setup node registry
    // --------------------------------
    
    let mut registry = NodeRegistry::new();
    register_standard_nodes(&mut registry);

    // --------------------------------
    // Build declarative graph
    // --------------------------------
    
    let mut graph_def = GraphDef::new();
    
    // Create a simple synth: Osc -> Env -> Output
    let osc = graph_def.add_node(node_types::SINE_OSC);
    let env = graph_def.add_node(node_types::ADSR_ENV);
    let out = graph_def.add_node(node_types::OUTPUT);
    
    // Wire them up
    graph_def.connect(osc, 0, env, 0);
    graph_def.connect(env, 0, out, 0);
    graph_def.output_node = Some(out);
    
    // Set some parameters
    graph_def.set_param(osc, nodes::params::FREQ, 440.0);
    graph_def.set_param(env, nodes::params::ATTACK, 0.01);
    graph_def.set_param(env, nodes::params::RELEASE, 0.5);

    // --------------------------------
    // Compile to runtime graph
    // --------------------------------
    
    let mut graph = compile(&graph_def, &registry, max_block, max_voices)
        .expect("Failed to compile graph");
    graph.prepare(sample_rate);

    // --------------------------------
    // Engine + Scheduler
    // --------------------------------

    let voices = VoiceAllocator::new(max_voices);
    let mut engine = Engine::new(graph, voices);

    let mut scheduler = Scheduler::new(sample_rate);

    // --------------------------------
    // Plan handoff (double buffer)
    // --------------------------------

    let mut handoff = PlanHandoff::new(
        ExecutionPlan::new(sample_rate),
        ExecutionPlan::new(sample_rate),
    );

    // --------------------------------
    // Run a few blocks
    // --------------------------------

    println!("Starting engine test with compiled graph...");
    println!("Graph: SineOsc -> ADSR -> Output");
    println!();

    for block in 0..4 {
        println!("--- Block {} ---", block);

        // Compile next block
        scheduler.compile_block(
            &mut handoff,
            block_frames,
            &[], // no musical events yet
        );

        // Audio thread would do this:
        let plan = handoff.read_plan();
        engine.process_plan(plan);

        println!(
            "  Sample: {}, Slices: {}, Active voices: {}",
            plan.block_start_sample,
            plan.slices.len(),
            engine.active_voices(),
        );
    }

    // Simulate a note on
    println!();
    println!("--- Simulating Note On (C4) ---");
    
    use crate::event::MusicalEvent;
    
    for block in 4..12 {
        // Send note on at the current beat position (block 4)
        let block_events: Vec<MusicalEvent> = if block == 4 {
            vec![MusicalEvent::NoteOn { 
                beat: scheduler.beat_position(), 
                note: 60, 
                velocity: 0.8,
            }]
        } else {
            vec![]
        };
        
        scheduler.compile_block(&mut handoff, block_frames, &block_events);
        let plan = handoff.read_plan();
        engine.process_plan(plan);
        
        if let Some(output) = engine.output_buffer(block_frames) {
            let peak: f32 = output.iter().map(|s| s.abs()).fold(0.0, f32::max);
            println!(
                "Block {}: sample {}, voices: {}, peak: {:.4}",
                block,
                plan.block_start_sample,
                engine.active_voices(),
                peak,
            );
        }
    }

    println!();
    println!("Engine test completed.");
}
