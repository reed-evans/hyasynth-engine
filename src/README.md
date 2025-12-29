### Example Engine Usage
```rust
// 1. Register node types (once at startup)
let mut registry = NodeRegistry::new();
registry.register(
    NodeTypeInfo::new(SINE_OSC, "Sine Oscillator", "Generators"),
    SimpleNodeFactory::new(|| Box::new(SineOsc::new()), Polyphony::PerVoice),
);

// 2. UI builds declarative graph
let mut graph_def = GraphDef::new();
let osc = graph_def.add_node(SINE_OSC);
let out = graph_def.add_node(OUTPUT);
graph_def.connect(osc, 0, out, 0);
graph_def.set_param(osc, FREQ_PARAM, 440.0);
graph_def.output_node = Some(out);

// 3. Compile to runtime graph
let graph = compile(&graph_def, &registry, 512, 8)?;

// 4. Run engine
let mut engine = Engine::new(graph, voices);
engine.process_plan(&plan);
```

### Example Swift Usage
```swift
import HyaSynth  // Import via wrapper, but could use bridging header or module map without wrapper

let session = HyaSynthSession(name: "MySynth")

// Create a simple synth chain
let (osc, env, out) = session.createSimpleSynth(oscillator: .sawOsc)

// Set parameters
session.setOscParam(osc, .frequency, value: 440.0)
session.setEnvelopeParam(env, .attack, value: 0.01)

// Play a note
session.noteOn(60, velocity: 0.8)
```

Next Steps for iOS
1. FFI Layer - Expose SessionHandle via uniffi or manual C bindings
2. Node Registry - Populate NodeTypeRegistry with your oscillators, filters, etc. [DONE] (sort of, can always add more)
3. Graph Compiler - Build runtime Graph from GraphDef [DONE]
4. SwiftUI Bindings - Create ObservableObject wrappers