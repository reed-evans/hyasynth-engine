// src/nodes/mod.rs
//
// Standard node types for the synthesizer.

mod oscillators;
mod envelope;
mod effects;
mod utility;

pub use oscillators::*;
pub use envelope::*;
pub use effects::*;
pub use utility::*;

use crate::node::Polyphony;
use crate::node_factory::{NodeRegistry, SimpleNodeFactory};
use crate::state::{NodeTypeInfo, ParamInfo, ParamUnit, DisplayCurve, PortInfo};

// ═══════════════════════════════════════════════════════════════════
// Node Type IDs
// ═══════════════════════════════════════════════════════════════════

pub mod node_types {
    pub const SINE_OSC: u32 = 1;
    pub const SAW_OSC: u32 = 2;
    pub const SQUARE_OSC: u32 = 3;
    pub const TRIANGLE_OSC: u32 = 4;
    
    pub const ADSR_ENV: u32 = 10;
    
    pub const GAIN: u32 = 20;
    pub const PAN: u32 = 21;
    pub const MIXER: u32 = 22;
    
    pub const OUTPUT: u32 = 100;
}

// ═══════════════════════════════════════════════════════════════════
// Parameter IDs (per-node-type)
// ═══════════════════════════════════════════════════════════════════

pub mod params {
    // Oscillator params
    pub const FREQ: u32 = 0;
    pub const DETUNE: u32 = 1;
    pub const PHASE: u32 = 2;
    pub const PULSE_WIDTH: u32 = 3;
    
    // Envelope params
    pub const ATTACK: u32 = 0;
    pub const DECAY: u32 = 1;
    pub const SUSTAIN: u32 = 2;
    pub const RELEASE: u32 = 3;
    
    // Gain/mixer params
    pub const GAIN: u32 = 0;
    pub const PAN: u32 = 1;
}

// ═══════════════════════════════════════════════════════════════════
// Registry Population
// ═══════════════════════════════════════════════════════════════════

/// Populate the registry with all standard node types.
pub fn register_standard_nodes(registry: &mut NodeRegistry) {
    register_oscillators(registry);
    register_envelopes(registry);
    register_effects(registry);
    register_utility(registry);
}

fn register_oscillators(registry: &mut NodeRegistry) {
    // Sine Oscillator
    registry.register(
        NodeTypeInfo::new(node_types::SINE_OSC, "Sine", "Oscillators")
            .with_output(PortInfo::audio_output(0, "Out"))
            .with_param(
                ParamInfo::new(params::FREQ, "Frequency")
                    .range(20.0, 20000.0)
                    .default(440.0)
                    .unit(ParamUnit::Hz)
                    .curve(DisplayCurve::Logarithmic),
            )
            .with_param(
                ParamInfo::new(params::DETUNE, "Detune")
                    .range(-100.0, 100.0)
                    .default(0.0)
                    .unit(ParamUnit::Semitones)
                    .curve(DisplayCurve::Symmetric),
            ),
        SimpleNodeFactory::new(|| Box::new(SineOsc::new()), Polyphony::PerVoice).channels(1),
    );

    // Saw Oscillator
    registry.register(
        NodeTypeInfo::new(node_types::SAW_OSC, "Saw", "Oscillators")
            .with_output(PortInfo::audio_output(0, "Out"))
            .with_param(
                ParamInfo::new(params::FREQ, "Frequency")
                    .range(20.0, 20000.0)
                    .default(440.0)
                    .unit(ParamUnit::Hz)
                    .curve(DisplayCurve::Logarithmic),
            )
            .with_param(
                ParamInfo::new(params::DETUNE, "Detune")
                    .range(-100.0, 100.0)
                    .default(0.0)
                    .unit(ParamUnit::Semitones)
                    .curve(DisplayCurve::Symmetric),
            ),
        SimpleNodeFactory::new(|| Box::new(SawOsc::new()), Polyphony::PerVoice).channels(1),
    );

    // Square Oscillator
    registry.register(
        NodeTypeInfo::new(node_types::SQUARE_OSC, "Square", "Oscillators")
            .with_output(PortInfo::audio_output(0, "Out"))
            .with_param(
                ParamInfo::new(params::FREQ, "Frequency")
                    .range(20.0, 20000.0)
                    .default(440.0)
                    .unit(ParamUnit::Hz)
                    .curve(DisplayCurve::Logarithmic),
            )
            .with_param(
                ParamInfo::new(params::PULSE_WIDTH, "Pulse Width")
                    .range(0.01, 0.99)
                    .default(0.5)
                    .unit(ParamUnit::Percent),
            ),
        SimpleNodeFactory::new(|| Box::new(SquareOsc::new()), Polyphony::PerVoice).channels(1),
    );

    // Triangle Oscillator
    registry.register(
        NodeTypeInfo::new(node_types::TRIANGLE_OSC, "Triangle", "Oscillators")
            .with_output(PortInfo::audio_output(0, "Out"))
            .with_param(
                ParamInfo::new(params::FREQ, "Frequency")
                    .range(20.0, 20000.0)
                    .default(440.0)
                    .unit(ParamUnit::Hz)
                    .curve(DisplayCurve::Logarithmic),
            ),
        SimpleNodeFactory::new(|| Box::new(TriangleOsc::new()), Polyphony::PerVoice).channels(1),
    );
}

fn register_envelopes(registry: &mut NodeRegistry) {
    registry.register(
        NodeTypeInfo::new(node_types::ADSR_ENV, "ADSR", "Envelopes")
            .with_input(PortInfo::audio_input(0, "In"))
            .with_output(PortInfo::audio_output(0, "Out"))
            .with_param(
                ParamInfo::new(params::ATTACK, "Attack")
                    .range(0.001, 10.0)
                    .default(0.01)
                    .unit(ParamUnit::Seconds)
                    .curve(DisplayCurve::Logarithmic),
            )
            .with_param(
                ParamInfo::new(params::DECAY, "Decay")
                    .range(0.001, 10.0)
                    .default(0.1)
                    .unit(ParamUnit::Seconds)
                    .curve(DisplayCurve::Logarithmic),
            )
            .with_param(
                ParamInfo::new(params::SUSTAIN, "Sustain")
                    .range(0.0, 1.0)
                    .default(0.7)
                    .unit(ParamUnit::Percent),
            )
            .with_param(
                ParamInfo::new(params::RELEASE, "Release")
                    .range(0.001, 10.0)
                    .default(0.3)
                    .unit(ParamUnit::Seconds)
                    .curve(DisplayCurve::Logarithmic),
            ),
        SimpleNodeFactory::new(|| Box::new(AdsrEnvelope::new()), Polyphony::PerVoice).channels(1),
    );
}

fn register_effects(registry: &mut NodeRegistry) {
    // Gain
    registry.register(
        NodeTypeInfo::new(node_types::GAIN, "Gain", "Effects")
            .with_input(PortInfo::audio_input(0, "In"))
            .with_output(PortInfo::audio_output(0, "Out"))
            .with_param(
                ParamInfo::new(params::GAIN, "Gain")
                    .range(-60.0, 12.0)
                    .default(0.0)
                    .unit(ParamUnit::Db)
                    .curve(DisplayCurve::Logarithmic),
            ),
        SimpleNodeFactory::new(|| Box::new(GainNode::new()), Polyphony::Global).channels(2),
    );

    // Pan
    registry.register(
        NodeTypeInfo::new(node_types::PAN, "Pan", "Effects")
            .with_input(PortInfo::audio_input(0, "In"))
            .with_output(PortInfo::audio_output(0, "Out").stereo())
            .with_param(
                ParamInfo::new(params::PAN, "Pan")
                    .range(-1.0, 1.0)
                    .default(0.0)
                    .unit(ParamUnit::Pan)
                    .curve(DisplayCurve::Symmetric),
            ),
        SimpleNodeFactory::new(|| Box::new(PanNode::new()), Polyphony::Global).channels(2),
    );
}

fn register_utility(registry: &mut NodeRegistry) {
    // Output
    registry.register(
        NodeTypeInfo::new(node_types::OUTPUT, "Output", "Utility")
            .with_input(PortInfo::audio_input(0, "L"))
            .with_input(PortInfo::audio_input(1, "R"))
            .with_param(
                ParamInfo::new(params::GAIN, "Master")
                    .range(-60.0, 6.0)
                    .default(0.0)
                    .unit(ParamUnit::Db),
            ),
        SimpleNodeFactory::new(|| Box::new(OutputNode::new()), Polyphony::Global).channels(2),
    );
}

