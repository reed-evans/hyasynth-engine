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

### How To Use With Xcode
```bash
# Build for all targets
cargo build --release --features ios --target aarch64-apple-ios
cargo build --release --features ios --target aarch64-apple-ios-sim

# Create XCFramework
xcodebuild -create-xcframework \
    -library target/aarch64-apple-ios/release/libhyasynth.a \
    -headers include/ \
    -library target/aarch64-apple-ios-sim/release/libhyasynth.a \
    -headers include/ \
    -output Hyasynth.xcframework
```
Move `Hyasynth.xcframework` into your Xcode project (drag into General tab -> Frameworks, Libraries, and Embedded Content section). Then copy `swift/Hyasynth.swift` into your Xcode project.

### Example Swift Usage
```swift
import Hyasynth  // Import via wrapper, but could use bridging header or module map without wrapper

let session = HyasynthSession(name: "MySynth")

// Create a simple synth chain
let (osc, env, out) = session.createSimpleSynth(oscillator: .sawOsc)

// Set parameters
session.setOscParam(osc, .frequency, value: 440.0)
session.setEnvelopeParam(env, .attack, value: 0.01)

// Play a note
session.noteOn(60, velocity: 0.8)
```

### Example Web Usage
```bash
# Build for WebAssembly
wasm-pack build --target web --features web
```

```javascript
import init, {
    hyasynth_init,
    HyasynthSession,
    HyasynthRegistry,
    node_sine_osc,
    node_output
} from './pkg/hyasynth.js';

await init();
hyasynth_init();

const session = new HyasynthSession("My Synth");
const engine = session.create_engine();
const registry = new HyasynthRegistry();

const osc = session.add_node(node_sine_osc(), 0, 0);
const out = session.add_node(node_output(), 100, 0);
session.connect(osc, 0, out, 0);
session.set_output(out);

engine.compile_graph(session, registry, 48000);
```