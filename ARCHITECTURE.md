# Hyasynth Engine Architecture

## Overview

Hyasynth is a real-time audio engine designed for DAW-like applications (similar to Bitwig/Ableton). It features a strict separation between UI state (declarative) and engine state (real-time), connected via lock-free communication channels.

## Key Concepts

### Thread Model

```
┌─────────────────┐                    ┌──────────────────┐
│   UI Thread     │                    │  Audio Thread    │
│                 │                    │                  │
│  SessionHandle  │◄──commands/results──►│ EngineHandle    │
│  + Session      │                    │ + EngineController│
│                 │                    │   + Engine       │
└─────────────────┘                    └──────────────────┘
```

### Declarative State Layer

The UI manipulates **declarative state** that describes what the audio should do:

- **`Session`**: Top-level state (graph, arrangement, transport, audio pool)
- **`GraphDef`**: User-defined DSP graph (nodes, connections, parameters)
- **`Arrangement`**: Musical content (clips, tracks, scenes)
- **`ClipDef`**: Note sequences or audio regions
- **`TrackDef`**: Routing, volume, pan, mute/solo
- **`AudioPool`**: Shared audio sample data

### Runtime State Layer

The audio thread owns **runtime state** that performs real-time processing:

- **`Engine`**: Real-time DSP processor
- **`Graph`**: Compiled, optimized node graph
- **`Scheduler`**: Musical time → sample time converter
- **`ClipPlayback`**: Clip state tracker and event generator
- **`VoiceAllocator`**: Polyphonic voice management

### Communication

**Commands flow UI → Engine:**
```rust
pub enum Command {
    // Graph structure
    AddNode, RemoveNode, Connect, SetParam,
    
    // Arrangement
    CreateTrack, CreateClip, LaunchScene,
    
    // Transport
    Play, Stop, SetTempo,
    
    // Compilation
    RecompileGraph,
}
```

**Readback flows Engine → UI:**
```rust
pub struct EngineReadback {
    sample_position: u64,
    beat_position: f64,
    active_voices: usize,
    output_peaks: [f32; 2],
}
```

## Using the Engine

### 1. UI Thread Setup

```rust
use hyasynth::{Session, SessionHandle, create_bridge};

// Create declarative state
let mut session = Session::new("My Project");
session.sample_rate = 48000.0;
session.max_voices = 16;
session.transport.bpm = 120.0;

// Create communication bridge
let (mut session_handle, engine_handle) = create_bridge(session);

// UI manipulates state via SessionHandle
session_handle.play();
session_handle.create_track("Lead Synth");
session_handle.note_on(60, 0.8);

// Poll for readback
let readback = session_handle.readback();
println!("Position: {}", readback.sample_position);
```

### 2. Audio Thread Setup

```rust
use hyasynth::{create_engine_controller, register_standard_nodes, NodeRegistry};

// Setup node registry (once at startup)
let mut registry = NodeRegistry::new();
register_standard_nodes(&mut registry);

// Create engine controller
let mut controller = create_engine_controller(
    session,
    registry,
    48000.0,  // sample rate
    512,      // max block size
);
```

### 3. Audio Callback

```rust
fn audio_callback(
    output: &mut [f32],
    controller: &mut EngineController,
    engine_handle: &EngineHandle,
    session: &Session,
) {
    // Process block
    controller.process_block(output, session, engine_handle);
}
```

## Graph Compilation

The engine uses a **derived graph compilation** approach:

1. **User Graph** (`GraphDef`): Instruments and effects added by the user
2. **Arrangement**: Tracks, clips, scenes
3. **Runtime Graph** (auto-generated):
   - User nodes (instruments, effects)
   - Per-track mixer nodes (volume, pan)
   - Master bus (mixer summing all tracks)
   - Output node

```
Track 1: [Instrument] → [Volume] → [Pan] ─┐
Track 2: [Instrument] → [Volume] → [Pan] ─┼→ [Master Bus] → [Output]
Track 3: [Instrument] → [Volume] → [Pan] ─┘
```

### When to Recompile

- **Full recompilation** (expensive): Adding/removing tracks, changing routing
- **Parameter updates** (cheap): Volume, pan, mute, instrument parameters

```rust
// Structural change → full recompilation
session_handle.create_track("New Track");
session_handle.recompile_graph();

// Parameter change → direct update
session_handle.set_track_volume(track_id, 0.8);
// Automatically sends SetParam commands, no recompilation needed
```

## Clip Playback

Clips generate **musical events** which the scheduler converts to sample-accurate engine events:

```
ClipDef (notes/audio) 
    ↓
ClipPlayback (tracks playing clips)
    ↓
MusicalEvent (beat time)
    ↓
Scheduler (musical → sample time)
    ↓
Event (sample offset)
    ↓
Engine (DSP processing)
```

### Event Types

**MIDI Events:**
- `NoteOn` / `NoteOff`: Global (polyphonic voice allocation)
- `NoteOnTarget` / `NoteOffTarget`: Targeted to specific node

**Audio Events:**
- `AudioStart`: Start playing an audio region
- `AudioStop`: Stop playing an audio region

**Parameter Events:**
- `ParamChange`: Update node parameter

## Node System

### Node Trait

```rust
pub trait Node: Send {
    fn prepare(&mut self, sample_rate: f64, max_block: usize);
    
    fn process(
        &mut self,
        ctx: &ProcessContext,
        inputs: &[&AudioBuffer],
        output: &mut AudioBuffer,
    ) -> bool; // Returns true if silent
    
    fn set_param(&mut self, param_id: u32, value: f32);
    fn reset(&mut self);
    
    // Audio playback (optional, for sampler nodes)
    fn start_audio(&mut self, audio_id, start, duration, gain);
    fn stop_audio(&mut self, audio_id);
    fn load_audio(&mut self, data: SharedAudioData);
}
```

### Standard Nodes

**Oscillators:**
- `SineOsc`, `SawOsc`, `SquareOsc`, `TriangleOsc`, `NoiseOsc`

**Envelopes:**
- `AdsrEnv`

**Filters:**
- `LowpassFilter`, `HighpassFilter`, `BandpassFilter`, `NotchFilter`

**Modulation:**
- `LfoNode` (sine, triangle, saw, square, sample & hold)

**Effects:**
- `GainNode`, `PanNode`, `MixerNode`
- `DelayNode`, `ReverbNode`

**Audio Playback:**
- `AudioPlayerNode` (for audio clips)

**Utility:**
- `OutputNode`

### Polyphony

Nodes can be:
- **Global**: One instance shared across all voices (effects, LFOs)
- **PerVoice**: One instance per voice (oscillators, envelopes)

## Performance Considerations

### Lock-Free Design

- No mutexes in audio thread
- Commands use MPSC channels (producer: UI, consumer: audio)
- Readback uses atomics

### Pre-Allocated Buffers

- All DSP buffers pre-allocated at startup
- Execution plans reuse scratch space
- No allocations in audio callback

### Graph Optimization

- Topological sort determines processing order
- Nodes that produce silence are skipped
- Per-voice nodes only process active voices

## FFI Layer

The engine exposes C-compatible bindings for Swift/iOS:

```c
// Create session
SessionHandle* session_create(const char* name);

// Graph manipulation
void session_add_node(SessionHandle*, uint32_t type_id, float x, float y);
void session_connect(SessionHandle*, uint32_t src, /*...*/);

// Arrangement
ClipId session_create_clip(SessionHandle*, const char* name, float length);
TrackId session_create_track(SessionHandle*, const char* name);
void session_launch_clip(SessionHandle*, TrackId, ClipId);

// Transport
void session_play(SessionHandle*);
void session_set_tempo(SessionHandle*, float bpm);
```

See `include/hyasynth.h` and `swift/Hyasynth.swift` for complete API.

## Testing Strategy

- **Unit tests**: Individual node processing, graph compilation
- **Integration tests**: Full engine pipeline
- **Example**: See `src/main.rs` for working demo

## Future Work

- MIDI input capture for recording
- Automation lanes
- Plugin hosting (VST3/CLAP)
- Multi-threaded node processing
- GPU acceleration for effects

