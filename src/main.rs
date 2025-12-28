// src/main.rs

mod audio_buffer;
mod engine;
mod event;
mod execution_plan;
mod graph;
mod node;
mod node_factory;
mod plan_handoff;
mod scheduler;
mod transport;
mod voice;
mod voice_allocator;

use crate::{
    engine::Engine,
    execution_plan::ExecutionPlan,
    graph::Graph,
    node::{Node, Polyphony},
    node_factory::SimpleNodeFactory,
    plan_handoff::PlanHandoff,
    scheduler::Scheduler,
    voice_allocator::VoiceAllocator,
};

/// ===============================
/// Test Nodes
/// ===============================

struct ConstantNode {
    value: f32,
}

impl Node for ConstantNode {
    fn prepare(&mut self, _sample_rate: f64, _max_block: usize) {}

    fn process_slice(
        &mut self,
        _slice: &execution_plan::SlicePlan,
        output: &mut audio_buffer::AudioBuffer,
    ) -> bool {
        for ch in 0..output.channels {
            output.channel_mut(ch).fill(self.value);
        }
        false
    }

    fn num_channels(&self) -> usize {
        1
    }

    fn polyphony(&self) -> Polyphony {
        Polyphony::Global
    }

    fn set_param(&mut self, _param_id: u32, _value: f32) {}
}

struct PassthroughNode;

impl Node for PassthroughNode {
    fn prepare(&mut self, _sample_rate: f64, _max_block: usize) {}

    fn process_slice(
        &mut self,
        _slice: &execution_plan::SlicePlan,
        _output: &mut audio_buffer::AudioBuffer,
    ) -> bool {
        false
    }

    fn num_channels(&self) -> usize {
        1
    }

    fn set_param(&mut self, _param_id: u32, _value: f32) {}
}

/// ===============================
/// Main
/// ===============================

fn main() {
    let sample_rate = 48_000.0;
    let block_frames = 256;
    let max_block = 512;
    let max_voices = 8;

    // --------------------------------
    // Graph
    // --------------------------------

    let mut graph = Graph::new(max_block, max_voices);
    let node_factory =
        SimpleNodeFactory::new(|| Box::new(ConstantNode { value: 0.25 }), Polyphony::Global);
    let passthrough_node_factory =
        SimpleNodeFactory::new(|| Box::new(PassthroughNode), Polyphony::Global);

    let const_node = graph.add_node(&node_factory);
    let out_node = graph.add_node(&passthrough_node_factory);

    graph.connect(const_node, out_node);
    graph.output_node = out_node;

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

    let empty_plan = ExecutionPlan {
        block_start_sample: 0,
        block_frames: 0,
        slices: Vec::new(),
    };

    let mut handoff = PlanHandoff::new(empty_plan.clone(), empty_plan);

    // --------------------------------
    // Run a few blocks
    // --------------------------------

    println!("Starting engine sanity test…");

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
            "Processed block @ sample {} ({} slices)",
            plan.block_start_sample,
            plan.slices.len()
        );
    }

    println!("Sanity test completed.");
}
