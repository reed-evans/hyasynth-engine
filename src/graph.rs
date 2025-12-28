// src/graph.rs

use crate::{
    audio_buffer::AudioBuffer,
    execution_plan::SlicePlan,
    node::{Node, Polyphony},
    node_factory::NodeFactory,
    voice_allocator::VoiceAllocator,
};

/// Storage for one node's buffers
pub struct NodeBuffer {
    pub channels: usize,
    pub data: Vec<f32>,
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
            NodeInstance::Global(node) => {
                node.set_param(param_id, value);
            }
            NodeInstance::PerVoice(nodes) => {
                for node in nodes.iter_mut() {
                    node.set_param(param_id, value);
                }
            }
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
}

impl Graph {
    pub fn new(max_block: usize, max_voices: usize) -> Self {
        Self {
            nodes: Vec::new(),
            buffers: Vec::new(),
            output_node: 0,
            max_block,
            max_voices,
        }
    }

    /// Add a node to the graph.
    /// Returns the node index.
    pub fn add_node(&mut self, factory: &dyn NodeFactory) -> usize {
        let channels = factory.num_channels();

        let instance = match factory.polyphony() {
            Polyphony::Global => NodeInstance::Global(factory.create()),
            Polyphony::PerVoice => {
                let mut nodes = Vec::with_capacity(self.max_voices);
                for _ in 0..self.max_voices {
                    nodes.push(factory.create());
                }
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
        self.nodes[dst].inputs.push(src);
    }

    /// Prepare all nodes
    pub fn prepare(&mut self, sample_rate: f64) {
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

    /// Process one slice
    pub fn process_slice(&mut self, slice: &SlicePlan, voices: &VoiceAllocator) {
        self.eval_node(self.output_node, slice, voices);
    }

    fn eval_node(&mut self, idx: usize, slice: &SlicePlan, voices: &VoiceAllocator) -> bool {
        let inputs = self.nodes[idx].inputs.clone();

        let mut inputs_silent = true;
        for i in inputs {
            inputs_silent &= self.eval_node(i, slice, voices);
        }

        let node = &mut self.nodes[idx];
        let buf = &mut self.buffers[idx];

        let mut output = AudioBuffer {
            channels: buf.channels,
            frames: slice.frame_count,
            data: &mut buf.data[..buf.channels * slice.frame_count],
        };

        output.clear();

        match &mut node.instance {
            NodeInstance::Global(n) => {
                if inputs_silent {
                    node.silent = true;
                    true
                } else {
                    let silent = n.process_slice(slice, &mut output);
                    node.silent = silent;
                    silent
                }
            }

            NodeInstance::PerVoice(nodes) => {
                let mut all_silent = true;

                for voice in voices.active_voices() {
                    let vn = &mut nodes[voice.id];

                    let mut voice_buf = AudioBuffer {
                        channels: buf.channels,
                        frames: slice.frame_count,
                        data: &mut buf.temp_voice[..buf.channels * slice.frame_count],
                    };

                    voice_buf.clear();

                    let silent = vn.process_slice(slice, &mut voice_buf);
                    if !silent {
                        all_silent = false;
                        mix_add(&voice_buf, &mut output);
                    }
                }

                node.silent = all_silent;
                all_silent
            }
        }
    }

    /// Set a parameter on all nodes.
    ///
    /// This is RT-safe *if* nodes implement set_param safely.
    /// Scheduler-side usage is preferred.
    pub fn set_param(&mut self, param_id: u32, value: f32) {
        for node in &mut self.nodes {
            match &mut node.instance {
                NodeInstance::Global(n) => {
                    n.set_param(param_id, value);
                }
                NodeInstance::PerVoice(nodes) => {
                    for n in nodes {
                        n.set_param(param_id, value);
                    }
                }
            }
        }
    }
}

/// Sum src into dst
fn mix_add(src: &AudioBuffer, dst: &mut AudioBuffer) {
    for ch in 0..dst.channels {
        for (d, s) in dst.channel_mut(ch).iter_mut().zip(src.channel(ch)) {
            *d += s;
        }
    }
}
