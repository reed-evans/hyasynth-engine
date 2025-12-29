// HyaSynth.swift
//
// Swift wrapper for the HyaSynth audio engine.
// This provides a more ergonomic Swift API over the C FFI.

import HyaSynthC

// MARK: - Node Types

public enum NodeType: UInt32 {
    case sineOsc = 1
    case sawOsc = 2
    case squareOsc = 3
    case triangleOsc = 4
    case adsrEnv = 10
    case gain = 20
    case pan = 21
    case output = 100
}

// MARK: - Parameter IDs

public enum OscParam: UInt32 {
    case frequency = 0
    case detune = 1
    case pulseWidth = 3
}

public enum EnvelopeParam: UInt32 {
    case attack = 0
    case decay = 1
    case sustain = 2
    case release = 3
}

public enum GainParam: UInt32 {
    case gain = 0
    case pan = 1
}

// MARK: - Engine Readback

public struct EngineState {
    public let samplePosition: UInt64
    public let beatPosition: Double
    public let cpuLoad: Float
    public let activeVoices: UInt32
    public let peakLeft: Float
    public let peakRight: Float
    public let isRunning: Bool
}

// MARK: - Session (UI-side handle)

public final class HyaSynthSession {
    private var handle: OpaquePointer?
    private var engineHandle: OpaquePointer?
    
    public init(name: String = "Untitled") {
        var engine: OpaquePointer?
        handle = name.withCString { cName in
            hya_session_create(cName, &engine)
        }
        engineHandle = engine
    }
    
    deinit {
        if let engine = engineHandle {
            hya_engine_destroy(engine)
        }
        if let session = handle {
            hya_session_destroy(session)
        }
    }
    
    // MARK: - Graph Mutations
    
    @discardableResult
    public func addNode(_ type: NodeType, at position: (x: Float, y: Float) = (0, 0)) -> UInt32 {
        guard let h = handle else { return UInt32.max }
        return hya_session_add_node(h, type.rawValue, position.x, position.y)
    }
    
    public func removeNode(_ nodeId: UInt32) {
        guard let h = handle else { return }
        hya_session_remove_node(h, nodeId)
    }
    
    public func connect(from sourceNode: UInt32, port sourcePort: UInt32 = 0,
                        to destNode: UInt32, port destPort: UInt32 = 0) {
        guard let h = handle else { return }
        hya_session_connect(h, sourceNode, sourcePort, destNode, destPort)
    }
    
    public func disconnect(from sourceNode: UInt32, port sourcePort: UInt32 = 0,
                           to destNode: UInt32, port destPort: UInt32 = 0) {
        guard let h = handle else { return }
        hya_session_disconnect(h, sourceNode, sourcePort, destNode, destPort)
    }
    
    public func setOutputNode(_ nodeId: UInt32) {
        guard let h = handle else { return }
        hya_session_set_output(h, nodeId)
    }
    
    public func clearGraph() {
        guard let h = handle else { return }
        hya_session_clear_graph(h)
    }
    
    // MARK: - Parameters
    
    public func setParam(_ nodeId: UInt32, param: UInt32, value: Float) {
        guard let h = handle else { return }
        hya_session_set_param(h, nodeId, param, value)
    }
    
    public func setOscParam(_ nodeId: UInt32, _ param: OscParam, value: Float) {
        setParam(nodeId, param: param.rawValue, value: value)
    }
    
    public func setEnvelopeParam(_ nodeId: UInt32, _ param: EnvelopeParam, value: Float) {
        setParam(nodeId, param: param.rawValue, value: value)
    }
    
    public func setGainParam(_ nodeId: UInt32, _ param: GainParam, value: Float) {
        setParam(nodeId, param: param.rawValue, value: value)
    }
    
    public func beginGesture(_ nodeId: UInt32, param: UInt32) {
        guard let h = handle else { return }
        hya_session_begin_gesture(h, nodeId, param)
    }
    
    public func endGesture(_ nodeId: UInt32, param: UInt32) {
        guard let h = handle else { return }
        hya_session_end_gesture(h, nodeId, param)
    }
    
    // MARK: - Transport
    
    public func play() {
        guard let h = handle else { return }
        hya_session_play(h)
    }
    
    public func stop() {
        guard let h = handle else { return }
        hya_session_stop(h)
    }
    
    public var isPlaying: Bool {
        guard let h = handle else { return false }
        return hya_session_is_playing(h)
    }
    
    public var tempo: Double {
        get {
            guard let h = handle else { return 120.0 }
            return hya_session_get_tempo(h)
        }
        set {
            guard let h = handle else { return }
            hya_session_set_tempo(h, newValue)
        }
    }
    
    public func seek(toBeat beat: Double) {
        guard let h = handle else { return }
        hya_session_seek(h, beat)
    }
    
    // MARK: - MIDI
    
    public func noteOn(_ note: UInt8, velocity: Float = 0.8) {
        guard let h = handle else { return }
        hya_session_note_on(h, note, velocity)
    }
    
    public func noteOff(_ note: UInt8) {
        guard let h = handle else { return }
        hya_session_note_off(h, note)
    }
    
    // MARK: - Readback
    
    public var engineState: EngineState {
        guard let h = handle else {
            return EngineState(
                samplePosition: 0,
                beatPosition: 0,
                cpuLoad: 0,
                activeVoices: 0,
                peakLeft: 0,
                peakRight: 0,
                isRunning: false
            )
        }
        let rb = hya_session_get_readback(h)
        return EngineState(
            samplePosition: rb.sample_position,
            beatPosition: rb.beat_position,
            cpuLoad: rb.cpu_load,
            activeVoices: rb.active_voices,
            peakLeft: rb.peak_left,
            peakRight: rb.peak_right,
            isRunning: rb.running
        )
    }
    
    public var nodeCount: UInt32 {
        guard let h = handle else { return 0 }
        return hya_session_node_count(h)
    }
    
    public var outputNode: UInt32? {
        guard let h = handle else { return nil }
        let id = hya_session_get_output_node(h)
        return id == UInt32.max ? nil : id
    }
    
    // MARK: - Engine Handle Access
    
    /// Get the raw engine handle for audio thread integration.
    /// Only use this if you're implementing custom audio hosting.
    public var unsafeEngineHandle: OpaquePointer? {
        return engineHandle
    }
}

// MARK: - Convenience Extensions

public extension HyaSynthSession {
    /// Create a simple synth chain: Oscillator -> Envelope -> Output
    func createSimpleSynth(oscillator: NodeType = .sineOsc) -> (osc: UInt32, env: UInt32, out: UInt32) {
        let osc = addNode(oscillator)
        let env = addNode(.adsrEnv)
        let out = addNode(.output)
        
        connect(from: osc, to: env)
        connect(from: env, to: out)
        setOutputNode(out)
        
        return (osc, env, out)
    }
}

