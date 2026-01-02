//! Audio processing graph with topological sorting and polyphonic voice support.
//!
//! The graph owns nodes and their buffers, processes them in dependency order,
//! and supports both global (shared) and per-voice (polyphonic) processing modes.

use crate::{
    audio_buffer::AudioBuffer,
    node::{Node, Polyphony, ProcessContext},
    node_factory::NodeFactory,
    voice_allocator::VoiceAllocator,
};

/// Storage for one node's output buffers.
pub struct NodeBuffer {
    pub channels: usize,
    pub data: Vec<f32>,
    /// Scratch buffer for accumulating per-voice outputs.
    pub temp_voice: Vec<f32>,
}

impl NodeBuffer {
    pub fn new(channels: usize, max_block: usize) -> Self {
        let size = channels * max_block;
        Self {
            channels,
            data: vec![0.0; size],
            temp_voice: vec![0.0; size],
        }
    }

    /// Get a mutable AudioBuffer view for writing output.
    #[inline]
    pub fn as_buffer(&mut self, frames: usize) -> AudioBuffer<'_> {
        AudioBuffer {
            channels: self.channels,
            frames,
            data: &mut self.data[..self.channels * frames],
        }
    }
}

/// Node instancing strategy
pub enum NodeInstance {
    Global(Box<dyn Node>),
    PerVoice(Vec<Box<dyn Node>>),
}

impl NodeInstance {
    #[inline]
    pub fn set_param(&mut self, param_id: u32, value: f32) {
        match self {
            NodeInstance::Global(node) => node.set_param(param_id, value),
            NodeInstance::PerVoice(nodes) => {
                for node in nodes {
                    node.set_param(param_id, value);
                }
            }
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        match self {
            NodeInstance::Global(node) => node.reset(),
            NodeInstance::PerVoice(nodes) => {
                for node in nodes {
                    node.reset();
                }
            }
        }
    }

    #[inline]
    pub fn is_per_voice(&self) -> bool {
        matches!(self, NodeInstance::PerVoice(_))
    }

    #[inline]
    pub fn start_audio(
        &mut self,
        audio_id: crate::state::AudioPoolId,
        start_sample: u64,
        duration_samples: u64,
        gain: f32,
    ) {
        match self {
            NodeInstance::Global(node) => {
                node.start_audio(audio_id, start_sample, duration_samples, gain);
            }
            NodeInstance::PerVoice(_) => {
                // Audio playback is typically global, not per-voice
            }
        }
    }

    #[inline]
    pub fn stop_audio(&mut self, audio_id: crate::state::AudioPoolId) {
        match self {
            NodeInstance::Global(node) => {
                node.stop_audio(audio_id);
            }
            NodeInstance::PerVoice(_) => {}
        }
    }

    #[inline]
    pub fn load_audio(&mut self, data: crate::nodes::SharedAudioData) {
        match self {
            NodeInstance::Global(node) => {
                node.load_audio(data);
            }
            NodeInstance::PerVoice(_) => {}
        }
    }

    #[inline]
    pub fn unload_audio(&mut self, audio_id: crate::state::AudioPoolId) {
        match self {
            NodeInstance::Global(node) => {
                node.unload_audio(audio_id);
            }
            NodeInstance::PerVoice(_) => {}
        }
    }
}

/// One node in the graph
pub struct GraphNode {
    pub instance: NodeInstance,
    pub inputs: Vec<usize>,
    pub silent: bool,
}

/// The audio graph
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub buffers: Vec<NodeBuffer>,
    pub output_node: usize,
    pub max_block: usize,
    pub max_voices: usize,
    pub sample_rate: f64,

    /// Topologically sorted evaluation order (computed in prepare)
    eval_order: Vec<usize>,

    /// Scratch space for collecting input buffer references
    input_scratch: Vec<usize>,

    /// Maps session node IDs to runtime graph indices.
    /// Populated during compilation.
    pub id_to_index: std::collections::HashMap<crate::state::NodeId, usize>,
}

impl Graph {
    pub fn new(max_block: usize, max_voices: usize) -> Self {
        Self {
            nodes: Vec::new(),
            buffers: Vec::new(),
            output_node: 0,
            max_block,
            max_voices,
            sample_rate: 48_000.0,
            eval_order: Vec::new(),
            input_scratch: Vec::new(),
            id_to_index: std::collections::HashMap::new(),
        }
    }

    /// Add a node to the graph. Returns the node index.
    pub fn add_node(&mut self, factory: &dyn NodeFactory) -> usize {
        let channels = factory.num_channels();

        let instance = match factory.polyphony() {
            Polyphony::Global => NodeInstance::Global(factory.create()),
            Polyphony::PerVoice => {
                let nodes = (0..self.max_voices).map(|_| factory.create()).collect();
                NodeInstance::PerVoice(nodes)
            }
        };

        let idx = self.nodes.len();

        self.nodes.push(GraphNode {
            instance,
            inputs: Vec::new(),
            silent: false,
        });

        self.buffers.push(NodeBuffer::new(channels, self.max_block));

        idx
    }

    /// Add an edge: src -> dst
    pub fn connect(&mut self, src: usize, dst: usize) {
        if !self.nodes[dst].inputs.contains(&src) {
            self.nodes[dst].inputs.push(src);
        }
    }

    /// Prepare all nodes and compute evaluation order
    pub fn prepare(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;

        // Compute topological order
        self.eval_order = self.topological_sort();

        // Prepare all nodes
        for (node, buf) in self.nodes.iter_mut().zip(&mut self.buffers) {
            match &mut node.instance {
                NodeInstance::Global(n) => n.prepare(sample_rate, self.max_block),
                NodeInstance::PerVoice(nodes) => {
                    for n in nodes {
                        n.prepare(sample_rate, self.max_block);
                    }
                }
            }
            node.silent = false;
            buf.data.fill(0.0);
            buf.temp_voice.fill(0.0);
        }
    }

    /// Compute topological sort of the graph (Kahn's algorithm)
    fn topological_sort(&self) -> Vec<usize> {
        let n = self.nodes.len();
        if n == 0 {
            return Vec::new();
        }

        // Count incoming edges
        let mut in_degree = vec![0usize; n];
        for (i, node) in self.nodes.iter().enumerate() {
            in_degree[i] = node.inputs.len();
        }

        // For each node, count how many nodes depend on it
        let mut out_edges: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (idx, node) in self.nodes.iter().enumerate() {
            for &input in &node.inputs {
                out_edges[input].push(idx);
            }
        }

        // Start with nodes that have no inputs (sources)
        let mut queue: Vec<usize> = in_degree
            .iter()
            .enumerate()
            .filter(|&(_, deg)| *deg == 0)
            .map(|(i, _)| i)
            .collect();

        let mut result = Vec::with_capacity(n);
        let mut processed = vec![false; n];

        while let Some(idx) = queue.pop() {
            if processed[idx] {
                continue;
            }
            processed[idx] = true;
            result.push(idx);

            // For each node that depends on this one
            for &dependent in &out_edges[idx] {
                // Check if all its inputs are processed
                let all_inputs_ready = self.nodes[dependent].inputs.iter().all(|&i| processed[i]);
                if all_inputs_ready && !processed[dependent] {
                    queue.push(dependent);
                }
            }
        }

        // If we didn't process all nodes, there's a cycle in the graph.
        // This is a graph definition error - cycles should be prevented at the UI level.
        // We append unprocessed nodes to avoid panicking, but results will be incorrect.
        let has_cycle = processed.iter().any(|&p| !p);
        if has_cycle {
            debug_assert!(
                false,
                "Graph contains a cycle - audio output will be incorrect"
            );
            for (i, &was_processed) in processed.iter().enumerate() {
                if !was_processed {
                    result.push(i);
                }
            }
        }

        result
    }

    /// Process one block of audio
    pub fn process(&mut self, frames: usize, sample_pos: u64, bpm: f64, voices: &VoiceAllocator) {
        let ctx = ProcessContext::new(frames, self.sample_rate, sample_pos, bpm);

        // Process nodes in topological order
        // Use index iteration to avoid cloning eval_order
        for i in 0..self.eval_order.len() {
            let idx = self.eval_order[i];
            self.process_node(idx, &ctx, voices);
        }
    }

    fn process_node(&mut self, idx: usize, ctx: &ProcessContext, voices: &VoiceAllocator) {
        // Collect input indices first (avoid borrow issues)
        self.input_scratch.clear();
        self.input_scratch
            .extend_from_slice(&self.nodes[idx].inputs);

        // Check if all inputs are silent
        let inputs_silent = self.input_scratch.iter().all(|&i| self.nodes[i].silent);

        let is_per_voice = self.nodes[idx].instance.is_per_voice();

        if is_per_voice {
            self.process_per_voice_node(idx, ctx, voices);
        } else {
            self.process_global_node(idx, ctx, inputs_silent);
        }
    }

    fn process_global_node(&mut self, idx: usize, ctx: &ProcessContext, inputs_silent: bool) {
        let frames = ctx.frames;
        let num_inputs = self.input_scratch.len();
        let has_inputs = num_inputs > 0;

        // Clear output buffer
        let buf = &mut self.buffers[idx];
        buf.data[..buf.channels * frames].fill(0.0);

        // Early exit if all inputs are silent
        if inputs_silent && has_inputs {
            self.nodes[idx].silent = true;
            return;
        }

        // SAFETY: We need simultaneous read access to multiple input buffers.
        // The borrow checker cannot verify that input indices differ from the output index,
        // but we guarantee this by construction (a node cannot be its own input).
        // We cast const pointers to mutable only because AudioBuffer requires &mut,
        // but we only read from input buffers during processing.
        let input_ptrs: Vec<_> = self
            .input_scratch
            .iter()
            .map(|&i| {
                let b = &self.buffers[i];
                (b.data.as_ptr(), b.channels)
            })
            .collect();

        let input_buffers: Vec<AudioBuffer<'_>> = input_ptrs
            .iter()
            .map(|&(ptr, channels)| unsafe {
                AudioBuffer {
                    channels,
                    frames,
                    data: std::slice::from_raw_parts_mut(ptr as *mut f32, channels * frames),
                }
            })
            .collect();

        let input_refs: Vec<&AudioBuffer<'_>> = input_buffers.iter().collect();

        // Process node
        let buf = &mut self.buffers[idx];
        let mut output = buf.as_buffer(frames);

        let silent = match &mut self.nodes[idx].instance {
            NodeInstance::Global(n) => n.process(ctx, &input_refs, &mut output),
            NodeInstance::PerVoice(_) => unreachable!(),
        };

        self.nodes[idx].silent = silent;
    }

    fn process_per_voice_node(
        &mut self,
        idx: usize,
        ctx: &ProcessContext,
        voices: &VoiceAllocator,
    ) {
        let frames = ctx.frames;

        // Clear output buffer
        let buf = &mut self.buffers[idx];
        let channels = buf.channels;
        buf.data[..channels * frames].fill(0.0);

        // SAFETY: Same justification as process_global_node - we need simultaneous
        // read access to input buffers while writing to a separate output buffer.
        let input_ptrs: Vec<_> = self
            .input_scratch
            .iter()
            .map(|&i| {
                let b = &self.buffers[i];
                (b.data.as_ptr(), b.channels)
            })
            .collect();

        let input_buffers: Vec<AudioBuffer<'_>> = input_ptrs
            .iter()
            .map(|&(ptr, ch)| unsafe {
                AudioBuffer {
                    channels: ch,
                    frames,
                    data: std::slice::from_raw_parts_mut(ptr as *mut f32, ch * frames),
                }
            })
            .collect();

        let input_refs: Vec<&AudioBuffer<'_>> = input_buffers.iter().collect();

        let mut all_silent = true;

        // Process each active voice
        for voice_ctx in voices.active_voices() {
            let voice_id = voice_ctx.id;
            let ctx_with_voice = ctx.with_voice(voice_ctx);

            // Clear temp buffer and create view
            let buf = &mut self.buffers[idx];
            buf.temp_voice[..channels * frames].fill(0.0);

            let mut voice_output = AudioBuffer {
                channels,
                frames,
                data: &mut buf.temp_voice[..channels * frames],
            };

            let silent = match &mut self.nodes[idx].instance {
                NodeInstance::PerVoice(nodes) => {
                    nodes[voice_id].process(&ctx_with_voice, &input_refs, &mut voice_output)
                }
                NodeInstance::Global(_) => unreachable!(),
            };

            if !silent {
                all_silent = false;
                // Mix voice output into node output
                let buf = &mut self.buffers[idx];
                for (out, temp) in buf.data[..channels * frames]
                    .iter_mut()
                    .zip(&buf.temp_voice[..channels * frames])
                {
                    *out += temp;
                }
            }
        }

        self.nodes[idx].silent = all_silent;
    }

    /// Set a parameter on a specific node by graph index.
    #[inline]
    pub fn set_param(&mut self, node_idx: usize, param_id: u32, value: f32) {
        if let Some(node) = self.nodes.get_mut(node_idx) {
            node.instance.set_param(param_id, value);
        }
    }

    /// Set a parameter on a node by session node ID.
    /// Uses the id_to_index mapping populated during compilation.
    #[inline]
    pub fn set_param_by_id(&mut self, node_id: crate::state::NodeId, param_id: u32, value: f32) {
        if let Some(&idx) = self.id_to_index.get(&node_id) {
            if let Some(node) = self.nodes.get_mut(idx) {
                node.instance.set_param(param_id, value);
            }
        }
    }

    /// Start audio playback on a node by graph index.
    pub fn start_audio(
        &mut self,
        node_idx: usize,
        audio_id: crate::state::AudioPoolId,
        start_sample: u64,
        duration_samples: u64,
        gain: f32,
    ) {
        if let Some(node) = self.nodes.get_mut(node_idx) {
            node.instance.start_audio(audio_id, start_sample, duration_samples, gain);
        }
    }

    /// Start audio playback on a node by session node ID.
    pub fn start_audio_by_id(
        &mut self,
        node_id: crate::state::NodeId,
        audio_id: crate::state::AudioPoolId,
        start_sample: u64,
        duration_samples: u64,
        gain: f32,
    ) {
        if let Some(&idx) = self.id_to_index.get(&node_id) {
            self.start_audio(idx, audio_id, start_sample, duration_samples, gain);
        }
    }

    /// Stop audio playback on a node by graph index.
    pub fn stop_audio(&mut self, node_idx: usize, audio_id: crate::state::AudioPoolId) {
        if let Some(node) = self.nodes.get_mut(node_idx) {
            node.instance.stop_audio(audio_id);
        }
    }

    /// Stop audio playback on a node by session node ID.
    pub fn stop_audio_by_id(&mut self, node_id: crate::state::NodeId, audio_id: crate::state::AudioPoolId) {
        if let Some(&idx) = self.id_to_index.get(&node_id) {
            self.stop_audio(idx, audio_id);
        }
    }

    /// Load audio data into a node.
    pub fn load_audio(&mut self, node_id: usize, data: crate::nodes::SharedAudioData) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.instance.load_audio(data);
        }
    }

    /// Unload audio data from a node.
    pub fn unload_audio(&mut self, node_id: usize, audio_id: crate::state::AudioPoolId) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.instance.unload_audio(audio_id);
        }
    }

    /// Load audio data into all nodes that handle audio.
    ///
    /// This is useful for initializing all audio players with pool data.
    pub fn load_audio_to_all(&mut self, data: crate::nodes::SharedAudioData) {
        for node in &mut self.nodes {
            node.instance.load_audio(data.clone());
        }
    }

    /// Reset all nodes (on transport stop/seek)
    pub fn reset(&mut self) {
        for node in &mut self.nodes {
            node.instance.reset();
            node.silent = false;
        }
        for buf in &mut self.buffers {
            buf.data.fill(0.0);
            buf.temp_voice.fill(0.0);
        }
    }

    /// Get the output buffer for reading
    pub fn output_buffer(&self, frames: usize) -> Option<&[f32]> {
        self.buffers
            .get(self.output_node)
            .map(|b| &b.data[..b.channels * frames])
    }
}
