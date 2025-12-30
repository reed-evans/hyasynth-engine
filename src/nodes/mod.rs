// Standard node types for the synthesizer.

mod audio_player;
mod effects;
mod envelope;
mod filters;
mod modulation;
mod oscillators;
mod utility;

pub use audio_player::*;
pub use effects::*;
pub use envelope::*;
pub use filters::*;
pub use modulation::*;
pub use oscillators::*;
pub use utility::*;

use crate::node::Polyphony;
use crate::node_factory::{NodeRegistry, SimpleNodeFactory};
use crate::state::{DisplayCurve, NodeTypeInfo, ParamInfo, ParamUnit, PortInfo};

// ═══════════════════════════════════════════════════════════════════
// Node Type IDs
// ═══════════════════════════════════════════════════════════════════

pub mod node_types {
    // Oscillators (1-9)
    pub const SINE_OSC: u32 = 1;
    pub const SAW_OSC: u32 = 2;
    pub const SQUARE_OSC: u32 = 3;
    pub const TRIANGLE_OSC: u32 = 4;

    // Envelopes (10-19)
    pub const ADSR_ENV: u32 = 10;

    // Effects (20-39)
    pub const GAIN: u32 = 20;
    pub const PAN: u32 = 21;
    pub const MIXER: u32 = 22;
    pub const DELAY: u32 = 23;
    pub const REVERB: u32 = 24;

    // Filters (40-49)
    pub const LOWPASS: u32 = 40;
    pub const HIGHPASS: u32 = 41;
    pub const BANDPASS: u32 = 42;
    pub const NOTCH: u32 = 43;

    // Modulators (50-59)
    pub const LFO: u32 = 50;

    // Samplers (60-69)
    pub const AUDIO_PLAYER: u32 = 60;

    // Utility (100+)
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

    // Filter params
    pub const CUTOFF: u32 = 0;
    pub const RESONANCE: u32 = 1;

    // LFO params
    pub const RATE: u32 = 0;
    pub const DEPTH: u32 = 1;
    pub const WAVEFORM: u32 = 2;

    // Delay params
    pub const TIME: u32 = 0;
    pub const FEEDBACK: u32 = 1;
    pub const MIX: u32 = 2;

    // Reverb params
    // Uses: DECAY (0), DAMPING (1), MIX (2)
    pub const DAMPING: u32 = 1;
}

// ═══════════════════════════════════════════════════════════════════
// Registry Population
// ═══════════════════════════════════════════════════════════════════

/// Populate the registry with all standard node types.
pub fn register_standard_nodes(registry: &mut NodeRegistry) {
    register_oscillators(registry);
    register_envelopes(registry);
    register_filters(registry);
    register_modulators(registry);
    register_effects(registry);
    register_samplers(registry);
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

fn register_filters(registry: &mut NodeRegistry) {
    // Lowpass Filter
    registry.register(
        NodeTypeInfo::new(node_types::LOWPASS, "Lowpass", "Filters")
            .with_input(PortInfo::audio_input(0, "In"))
            .with_output(PortInfo::audio_output(0, "Out"))
            .with_param(
                ParamInfo::new(params::CUTOFF, "Cutoff")
                    .range(20.0, 20000.0)
                    .default(1000.0)
                    .unit(ParamUnit::Hz)
                    .curve(DisplayCurve::Logarithmic),
            )
            .with_param(
                ParamInfo::new(params::RESONANCE, "Resonance")
                    .range(0.0, 1.0)
                    .default(0.5)
                    .unit(ParamUnit::Percent),
            ),
        SimpleNodeFactory::new(|| Box::new(SvfFilter::lowpass()), Polyphony::PerVoice).channels(1),
    );

    // Highpass Filter
    registry.register(
        NodeTypeInfo::new(node_types::HIGHPASS, "Highpass", "Filters")
            .with_input(PortInfo::audio_input(0, "In"))
            .with_output(PortInfo::audio_output(0, "Out"))
            .with_param(
                ParamInfo::new(params::CUTOFF, "Cutoff")
                    .range(20.0, 20000.0)
                    .default(200.0)
                    .unit(ParamUnit::Hz)
                    .curve(DisplayCurve::Logarithmic),
            )
            .with_param(
                ParamInfo::new(params::RESONANCE, "Resonance")
                    .range(0.0, 1.0)
                    .default(0.5)
                    .unit(ParamUnit::Percent),
            ),
        SimpleNodeFactory::new(|| Box::new(SvfFilter::highpass()), Polyphony::PerVoice).channels(1),
    );

    // Bandpass Filter
    registry.register(
        NodeTypeInfo::new(node_types::BANDPASS, "Bandpass", "Filters")
            .with_input(PortInfo::audio_input(0, "In"))
            .with_output(PortInfo::audio_output(0, "Out"))
            .with_param(
                ParamInfo::new(params::CUTOFF, "Center")
                    .range(20.0, 20000.0)
                    .default(1000.0)
                    .unit(ParamUnit::Hz)
                    .curve(DisplayCurve::Logarithmic),
            )
            .with_param(
                ParamInfo::new(params::RESONANCE, "Q")
                    .range(0.0, 1.0)
                    .default(0.5)
                    .unit(ParamUnit::Percent),
            ),
        SimpleNodeFactory::new(|| Box::new(SvfFilter::bandpass()), Polyphony::PerVoice).channels(1),
    );

    // Notch Filter
    registry.register(
        NodeTypeInfo::new(node_types::NOTCH, "Notch", "Filters")
            .with_input(PortInfo::audio_input(0, "In"))
            .with_output(PortInfo::audio_output(0, "Out"))
            .with_param(
                ParamInfo::new(params::CUTOFF, "Frequency")
                    .range(20.0, 20000.0)
                    .default(1000.0)
                    .unit(ParamUnit::Hz)
                    .curve(DisplayCurve::Logarithmic),
            )
            .with_param(
                ParamInfo::new(params::RESONANCE, "Width")
                    .range(0.0, 1.0)
                    .default(0.5)
                    .unit(ParamUnit::Percent),
            ),
        SimpleNodeFactory::new(|| Box::new(SvfFilter::notch()), Polyphony::PerVoice).channels(1),
    );
}

fn register_modulators(registry: &mut NodeRegistry) {
    // LFO
    registry.register(
        NodeTypeInfo::new(node_types::LFO, "LFO", "Modulators")
            .with_output(PortInfo::audio_output(0, "Out"))
            .with_param(
                ParamInfo::new(params::RATE, "Rate")
                    .range(0.01, 100.0)
                    .default(1.0)
                    .unit(ParamUnit::Hz)
                    .curve(DisplayCurve::Logarithmic),
            )
            .with_param(
                ParamInfo::new(params::DEPTH, "Depth")
                    .range(0.0, 1.0)
                    .default(1.0)
                    .unit(ParamUnit::Percent),
            )
            .with_param(
                ParamInfo::new(params::WAVEFORM, "Wave")
                    .range(0.0, 4.0)
                    .default(0.0)
                    .unit(ParamUnit::None),
            ),
        SimpleNodeFactory::new(|| Box::new(Lfo::new()), Polyphony::Global).channels(1),
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

    // Delay
    registry.register(
        NodeTypeInfo::new(node_types::DELAY, "Delay", "Effects")
            .with_input(PortInfo::audio_input(0, "In").stereo())
            .with_output(PortInfo::audio_output(0, "Out").stereo())
            .with_param(
                ParamInfo::new(params::TIME, "Time")
                    .range(0.001, 2.0)
                    .default(0.25)
                    .unit(ParamUnit::Seconds)
                    .curve(DisplayCurve::Logarithmic),
            )
            .with_param(
                ParamInfo::new(params::FEEDBACK, "Feedback")
                    .range(0.0, 0.99)
                    .default(0.4)
                    .unit(ParamUnit::Percent),
            )
            .with_param(
                ParamInfo::new(params::MIX, "Mix")
                    .range(0.0, 1.0)
                    .default(0.5)
                    .unit(ParamUnit::Percent),
            ),
        SimpleNodeFactory::new(|| Box::new(DelayNode::new()), Polyphony::Global).channels(2),
    );

    // Reverb
    registry.register(
        NodeTypeInfo::new(node_types::REVERB, "Reverb", "Effects")
            .with_input(PortInfo::audio_input(0, "In").stereo())
            .with_output(PortInfo::audio_output(0, "Out").stereo())
            .with_param(
                ParamInfo::new(params::DECAY, "Decay")
                    .range(0.0, 0.99)
                    .default(0.5)
                    .unit(ParamUnit::Percent),
            )
            .with_param(
                ParamInfo::new(params::DAMPING, "Damping")
                    .range(0.0, 1.0)
                    .default(0.5)
                    .unit(ParamUnit::Percent),
            )
            .with_param(
                ParamInfo::new(params::MIX, "Mix")
                    .range(0.0, 1.0)
                    .default(0.3)
                    .unit(ParamUnit::Percent),
            ),
        SimpleNodeFactory::new(|| Box::new(ReverbNode::new()), Polyphony::Global).channels(2),
    );
}

fn register_samplers(registry: &mut NodeRegistry) {
    // Audio Player
    registry.register(
        NodeTypeInfo::new(node_types::AUDIO_PLAYER, "Audio Player", "Samplers")
            .with_input(PortInfo::audio_input(0, "In").stereo())
            .with_output(PortInfo::audio_output(0, "Out").stereo())
            .with_param(
                ParamInfo::new(params::GAIN, "Gain")
                    .range(0.0, 2.0)
                    .default(1.0)
                    .unit(ParamUnit::None),
            ),
        SimpleNodeFactory::new(|| Box::new(AudioPlayerNode::new(2)), Polyphony::Global).channels(2),
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
