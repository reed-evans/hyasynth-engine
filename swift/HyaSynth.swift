// Swift wrapper for the Hyasynth audio engine.
// This provides a more ergonomic Swift API over the C FFI.

import HyasynthC

// MARK: - Node Types

public enum NodeType: UInt32 {
    // Oscillators
    case sineOsc = 1
    case sawOsc = 2
    case squareOsc = 3
    case triangleOsc = 4
    
    // Envelopes
    case adsrEnv = 10
    
    // Effects
    case gain = 20
    case pan = 21
    case delay = 23
    case reverb = 24
    
    // Filters
    case lowpass = 40
    case highpass = 41
    case bandpass = 42
    case notch = 43
    
    // Modulators
    case lfo = 50
    
    // Utility
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

public enum FilterParam: UInt32 {
    case cutoff = 0
    case resonance = 1
}

public enum LfoParam: UInt32 {
    case rate = 0
    case depth = 1
    case waveform = 2
}

public enum DelayParam: UInt32 {
    case time = 0
    case feedback = 1
    case mix = 2
}

public enum ReverbParam: UInt32 {
    case decay = 0
    case damping = 1
    case mix = 2
}

// MARK: - Configuration

/// Configuration for creating a Hyasynth session and engine.
public struct HyasynthConfiguration {
    /// Maximum audio block size in frames (e.g., 512, 1024).
    public var maxBlockSize: UInt32

    /// Maximum number of simultaneous voices for polyphony.
    public var maxVoices: UInt32

    /// Sample rate in Hz (e.g., 44100.0, 48000.0).
    public var sampleRate: Double

    /// Create a configuration with custom values.
    public init(maxBlockSize: UInt32 = 512, maxVoices: UInt32 = 16, sampleRate: Double = 48000.0) {
        self.maxBlockSize = maxBlockSize
        self.maxVoices = maxVoices
        self.sampleRate = sampleRate
    }

    /// The default configuration (512 max block, 16 voices, 48kHz).
    public static let `default` = HyasynthConfiguration()

    /// Convert to C struct for FFI.
    internal var cConfig: HyasynthConfig {
        HyasynthConfig(
            max_block_size: maxBlockSize,
            max_voices: maxVoices,
            sample_rate: sampleRate
        )
    }
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

public final class HyasynthSession {
    internal var sessionHandle: OpaquePointer?
    internal var engineHandle: OpaquePointer?

    /// The configuration used to create this session.
    public let configuration: HyasynthConfiguration

    /// Create a session with default configuration.
    public convenience init(name: String = "Untitled") {
        self.init(name: name, configuration: .default)
    }

    /// Create a session with custom configuration.
    ///
    /// - Parameters:
    ///   - name: The session name.
    ///   - configuration: Engine configuration (block size, voices, sample rate).
    public init(name: String = "Untitled", configuration: HyasynthConfiguration) {
        self.configuration = configuration
        var engine: OpaquePointer?
        var config = configuration.cConfig
        sessionHandle = name.withCString { cName in
            session_create_with_config(cName, &config, &engine)
        }
        engineHandle = engine
    }
    
    deinit {
        if let engine = engineHandle {
            engine_destroy(engine)
        }
        if let session = sessionHandle {
            session_destroy(session)
        }
    }
    
    // MARK: - Graph Mutations
    
    @discardableResult
    public func addNode(_ type: NodeType, at position: (x: Float, y: Float) = (0, 0)) -> UInt32 {
        guard let h = sessionHandle else { return UInt32.max }
        return session_add_node(h, type.rawValue, position.x, position.y)
    }
    
    public func removeNode(_ nodeId: UInt32) {
        guard let h = sessionHandle else { return }
        session_remove_node(h, nodeId)
    }
    
    public func connect(from sourceNode: UInt32, port sourcePort: UInt32 = 0,
                        to destNode: UInt32, port destPort: UInt32 = 0) {
        guard let h = sessionHandle else { return }
        session_connect(h, sourceNode, sourcePort, destNode, destPort)
    }
    
    public func disconnect(from sourceNode: UInt32, port sourcePort: UInt32 = 0,
                           to destNode: UInt32, port destPort: UInt32 = 0) {
        guard let h = sessionHandle else { return }
        session_disconnect(h, sourceNode, sourcePort, destNode, destPort)
    }
    
    public func setOutputNode(_ nodeId: UInt32) {
        guard let h = sessionHandle else { return }
        session_set_output(h, nodeId)
    }
    
    public func clearGraph() {
        guard let h = sessionHandle else { return }
        session_clear_graph(h)
    }
    
    // MARK: - Parameters
    
    public func setParam(_ nodeId: UInt32, param: UInt32, value: Float) {
        guard let h = sessionHandle else { return }
        session_set_param(h, nodeId, param, value)
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
        guard let h = sessionHandle else { return }
        session_begin_gesture(h, nodeId, param)
    }
    
    public func endGesture(_ nodeId: UInt32, param: UInt32) {
        guard let h = sessionHandle else { return }
        session_end_gesture(h, nodeId, param)
    }
    
    // MARK: - Transport
    
    public func play() {
        guard let h = sessionHandle else { return }
        session_play(h)
    }
    
    public func stop() {
        guard let h = sessionHandle else { return }
        session_stop(h)
    }
    
    public var isPlaying: Bool {
        guard let h = sessionHandle else { return false }
        return session_is_playing(h)
    }
    
    public var tempo: Double {
        get {
            guard let h = sessionHandle else { return 120.0 }
            return session_get_tempo(h)
        }
        set {
            guard let h = sessionHandle else { return }
            session_set_tempo(h, newValue)
        }
    }
    
    public func seek(toBeat beat: Double) {
        guard let h = sessionHandle else { return }
        session_seek(h, beat)
    }
    
    // MARK: - MIDI
    
    public func noteOn(_ note: UInt8, velocity: Float = 0.8) {
        guard let h = sessionHandle else { return }
        session_note_on(h, note, velocity)
    }
    
    public func noteOff(_ note: UInt8) {
        guard let h = sessionHandle else { return }
        session_note_off(h, note)
    }
    
    // MARK: - Readback
    
    public var engineState: EngineState {
        guard let h = sessionHandle else {
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
        let rb = session_get_readback(h)
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
        guard let h = sessionHandle else { return 0 }
        return session_node_count(h)
    }
    
    public var outputNode: UInt32? {
        guard let h = sessionHandle else { return nil }
        let id = session_get_output_node(h)
        return id == UInt32.max ? nil : id
    }
    
    // MARK: - Engine Handle Access
    
    /// Get the raw engine handle for audio thread integration.
    /// Only use this if you're implementing custom audio hosting.
    public var unsafeEngineHandle: OpaquePointer? {
        return engineHandle
    }
    
    // MARK: - Clips
    
    @discardableResult
    public func createClip(name: String = "Clip", length: Double = 4.0) -> UInt32 {
        guard let h = sessionHandle else { return UInt32.max }
        return name.withCString { session_create_clip(h, $0, length) }
    }
    
    public func deleteClip(_ clipId: UInt32) {
        guard let h = sessionHandle else { return }
        session_delete_clip(h, clipId)
    }
    
    public func addNote(toClip clipId: UInt32, start: Double, duration: Double, note: UInt8, velocity: Float = 0.8) {
        guard let h = sessionHandle else { return }
        session_add_note_to_clip(h, clipId, start, duration, note, velocity)
    }
    
    public func clearClip(_ clipId: UInt32) {
        guard let h = sessionHandle else { return }
        session_clear_clip(h, clipId)
    }
    
    public func getNoteCount(forClip clipId: UInt32) -> UInt32 {
        guard let h = sessionHandle else { return 0 }
        return session_get_clip_note_count(h, clipId)
    }
    
    public func getAudioCount(forClip clipId: UInt32) -> UInt32 {
        guard let h = sessionHandle else { return 0 }
        return session_get_clip_audio_count(h, clipId)
    }
    
    // MARK: - Audio Pool
    
    /// Add audio samples to the pool.
    /// Returns the audio pool ID.
    @discardableResult
    public func addAudioToPool(name: String, sampleRate: Double, channels: UInt32, samples: [Float]) -> UInt32 {
        guard let h = sessionHandle else { return UInt32.max }
        return samples.withUnsafeBufferPointer { buffer in
            name.withCString { cName in
                session_add_audio_to_pool(h, cName, sampleRate, channels, buffer.baseAddress, UInt32(samples.count))
            }
        }
    }
    
    /// Remove audio from the pool.
    public func removeAudioFromPool(_ audioId: UInt32) {
        guard let h = sessionHandle else { return }
        session_remove_audio_from_pool(h, audioId)
    }
    
    /// Add an audio region to a clip.
    public func addAudio(toClip clipId: UInt32, start: Double, duration: Double, audioId: UInt32, offset: Double = 0, gain: Float = 1.0) {
        guard let h = sessionHandle else { return }
        session_add_audio_to_clip(h, clipId, start, duration, audioId, offset, gain)
    }
    
    /// Create a clip containing the full audio at the current tempo.
    @discardableResult
    public func createClipFromAudio(_ audioId: UInt32, bpm: Double = 120.0) -> UInt32? {
        guard let h = sessionHandle else { return nil }
        let id = session_create_clip_from_audio(h, audioId, bpm)
        return id == UInt32.max ? nil : id
    }
    
    /// Get the number of audio entries in the pool.
    public var audioPoolCount: UInt32 {
        guard let h = sessionHandle else { return 0 }
        return session_get_audio_pool_count(h)
    }
    
    // MARK: - Tracks
    
    @discardableResult
    public func createTrack(name: String = "Track") -> UInt32 {
        guard let h = sessionHandle else { return UInt32.max }
        return name.withCString { session_create_track(h, $0) }
    }
    
    public func deleteTrack(_ trackId: UInt32) {
        guard let h = sessionHandle else { return }
        session_delete_track(h, trackId)
    }
    
    public func setTrackVolume(_ trackId: UInt32, volume: Float) {
        guard let h = sessionHandle else { return }
        session_set_track_volume(h, trackId, volume)
    }
    
    public func setTrackPan(_ trackId: UInt32, pan: Float) {
        guard let h = sessionHandle else { return }
        session_set_track_pan(h, trackId, pan)
    }
    
    public func setTrackMute(_ trackId: UInt32, mute: Bool) {
        guard let h = sessionHandle else { return }
        session_set_track_mute(h, trackId, mute)
    }
    
    public func setTrackSolo(_ trackId: UInt32, solo: Bool) {
        guard let h = sessionHandle else { return }
        session_set_track_solo(h, trackId, solo)
    }
    
    public func setTrackTarget(_ trackId: UInt32, nodeId: UInt32?) {
        guard let h = sessionHandle else { return }
        session_set_track_target(h, trackId, nodeId ?? UInt32.max)
    }
    
    public var trackCount: UInt32 {
        guard let h = sessionHandle else { return 0 }
        return session_get_track_count(h)
    }
    
    // MARK: - Scenes
    
    @discardableResult
    public func createScene(name: String = "Scene") -> UInt32 {
        guard let h = sessionHandle else { return UInt32.max }
        return name.withCString { session_create_scene(h, $0) }
    }
    
    public func deleteScene(_ sceneId: UInt32) {
        guard let h = sessionHandle else { return }
        session_delete_scene(h, sceneId)
    }
    
    public func launchScene(_ sceneIndex: UInt32) {
        guard let h = sessionHandle else { return }
        session_launch_scene(h, sceneIndex)
    }
    
    public func launchClip(trackId: UInt32, clipId: UInt32) {
        guard let h = sessionHandle else { return }
        session_launch_clip(h, trackId, clipId)
    }
    
    public func stopClip(trackId: UInt32) {
        guard let h = sessionHandle else { return }
        session_stop_clip(h, trackId)
    }
    
    public func stopAllClips() {
        guard let h = sessionHandle else { return }
        session_stop_all_clips(h)
    }
    
    public var sceneCount: UInt32 {
        guard let h = sessionHandle else { return 0 }
        return session_get_scene_count(h)
    }
    
    public func setClipSlot(trackId: UInt32, sceneIndex: UInt32, clipId: UInt32?) {
        guard let h = sessionHandle else { return }
        session_set_clip_slot(h, trackId, sceneIndex, clipId ?? UInt32.max)
    }
    
    // MARK: - Timeline
    
    public func scheduleClip(trackId: UInt32, clipId: UInt32, startBeat: Double) {
        guard let h = sessionHandle else { return }
        session_schedule_clip(h, trackId, clipId, startBeat)
    }
    
    public func removeClipPlacement(trackId: UInt32, startBeat: Double) {
        guard let h = sessionHandle else { return }
        session_remove_clip_placement(h, trackId, startBeat)
    }
}

// MARK: - Convenience Extensions

public extension HyasynthSession {
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

// MARK: - Node Registry

/// Registry of available node types.
public final class HyasynthRegistry {
    private var handle: OpaquePointer?

    public init() {
        handle = registry_create()
    }

    deinit {
        if let h = handle {
            registry_destroy(h)
        }
    }

    public var count: UInt32 {
        guard let h = handle else { return 0 }
        return registry_count(h)
    }

    internal var unsafeHandle: OpaquePointer? { handle }
}

// MARK: - Audio Engine

/// Audio rendering engine for real-time audio processing.
///
/// This class manages the audio thread side of the engine and provides
/// integration with AVAudioEngine via `AVAudioSourceNode`.
///
/// Usage:
/// ```swift
/// let session = HyasynthSession(name: "My Synth")
/// let registry = HyasynthRegistry()
/// let audioEngine = HyasynthAudioEngine(session: session, registry: registry)
///
/// // Build your graph
/// session.createSimpleSynth()
/// audioEngine.compileGraph()
///
/// // Start audio
/// try audioEngine.start()
/// ```
public final class HyasynthAudioEngine {
    private let session: HyasynthSession
    private let registry: HyasynthRegistry
    private var engineHandle: OpaquePointer?

    /// The sample rate used for audio processing.
    public private(set) var sampleRate: Double = 48000.0

    /// Whether the audio engine is currently running.
    public private(set) var isRunning: Bool = false

    /// Create an audio engine for the given session.
    ///
    /// - Parameters:
    ///   - session: The session containing the graph definition
    ///   - registry: The node registry for creating node instances
    public init(session: HyasynthSession, registry: HyasynthRegistry) {
        self.session = session
        self.registry = registry
        self.engineHandle = session.unsafeEngineHandle
    }

    // MARK: - Graph Compilation

    /// Compile the session's graph and load it into the engine.
    ///
    /// Call this after making structural changes to the graph (adding/removing
    /// nodes, changing connections).
    ///
    /// - Parameter sampleRate: The sample rate to prepare for (default: 48000)
    /// - Returns: `true` if compilation succeeded
    @discardableResult
    public func compileGraph(sampleRate: Double = 48000.0) -> Bool {
        guard let engine = engineHandle,
              let sessionHandle = session.sessionHandle,
              let reg = registry.unsafeHandle else {
            return false
        }

        self.sampleRate = sampleRate
        return engine_compile_graph(sessionHandle, engine, reg, sampleRate)
    }

    /// Prepare the engine for processing at the given sample rate.
    public func prepare(sampleRate: Double = 48000.0) {
        guard let engine = engineHandle else { return }
        self.sampleRate = sampleRate
        engine_prepare(engine, sampleRate)
    }

    /// Reset the engine state (clear buffers, reset oscillators/envelopes).
    public func reset() {
        guard let engine = engineHandle else { return }
        engine_reset(engine)
    }

    // MARK: - Rendering

    /// Process pending commands from the UI thread.
    ///
    /// Call this at the start of your render callback.
    ///
    /// - Returns: `true` if any command requires graph recompilation
    @discardableResult
    public func processCommands() -> Bool {
        guard let engine = engineHandle else { return false }
        return engine_process_commands(engine)
    }

    /// Render audio to separate left/right channel buffers.
    ///
    /// - Parameters:
    ///   - frames: Number of frames to render
    ///   - left: Pointer to left channel buffer
    ///   - right: Pointer to right channel buffer
    public func render(frames: UInt32, left: UnsafeMutablePointer<Float>, right: UnsafeMutablePointer<Float>) {
        guard let engine = engineHandle else {
            // Fill with silence
            left.initialize(repeating: 0, count: Int(frames))
            right.initialize(repeating: 0, count: Int(frames))
            return
        }
        engine_render(engine, frames, left, right)
    }

    /// Render audio to an interleaved stereo buffer.
    ///
    /// Output format: [L0, R0, L1, R1, ...]
    ///
    /// - Parameters:
    ///   - frames: Number of frames to render
    ///   - output: Pointer to interleaved output buffer (must have space for frames * 2 samples)
    public func renderInterleaved(frames: UInt32, output: UnsafeMutablePointer<Float>) {
        guard let engine = engineHandle else {
            output.initialize(repeating: 0, count: Int(frames) * 2)
            return
        }
        engine_render_interleaved(engine, frames, output)
    }

    // MARK: - State

    /// Check if the engine is currently playing.
    public var isPlaying: Bool {
        guard let engine = engineHandle else { return false }
        return engine_is_playing(engine)
    }

    /// Get the current tempo in BPM.
    public var tempo: Double {
        guard let engine = engineHandle else { return 120.0 }
        return engine_get_tempo(engine)
    }

    /// Get the number of active voices.
    public var activeVoices: UInt32 {
        guard let engine = engineHandle else { return 0 }
        return engine_get_active_voices(engine)
    }
}

// MARK: - AVAudioEngine Integration

#if canImport(AVFAudio)
import AVFAudio

/// Audio host that integrates HyasynthAudioEngine with AVAudioEngine.
///
/// This provides a complete audio playback solution for iOS/macOS apps.
///
/// Usage:
/// ```swift
/// let session = HyasynthSession(name: "My Synth")
/// let registry = HyasynthRegistry()
/// let host = HyasynthAudioHost(session: session, registry: registry)
///
/// // Build your graph
/// session.createSimpleSynth()
/// host.compileGraph()
///
/// // Start audio
/// try host.start()
///
/// // Play notes
/// session.noteOn(60, velocity: 0.8)
/// ```
public final class HyasynthAudioHost {
    /// The underlying Hyasynth audio engine.
    public let engine: HyasynthAudioEngine

    /// The session being hosted.
    public let session: HyasynthSession

    /// The AVAudioEngine instance.
    public let avEngine: AVAudioEngine

    /// The source node that generates audio.
    public private(set) var sourceNode: AVAudioSourceNode?

    /// Whether the audio host is currently running.
    public private(set) var isRunning: Bool = false

    /// Create an audio host for the given session.
    ///
    /// - Parameters:
    ///   - session: The session containing the graph definition
    ///   - registry: The node registry for creating node instances
    public init(session: HyasynthSession, registry: HyasynthRegistry) {
        self.session = session
        self.engine = HyasynthAudioEngine(session: session, registry: registry)
        self.avEngine = AVAudioEngine()
    }

    deinit {
        stop()
    }

    /// Compile the session's graph.
    ///
    /// Call this after making structural changes to the graph.
    /// Uses the current audio session's sample rate.
    @discardableResult
    public func compileGraph() -> Bool {
        let sampleRate = avEngine.outputNode.outputFormat(forBus: 0).sampleRate
        return engine.compileGraph(sampleRate: sampleRate > 0 ? sampleRate : 48000.0)
    }

    /// Start audio playback.
    ///
    /// This sets up the AVAudioEngine with a source node that renders
    /// audio from the Hyasynth engine.
    public func start() throws {
        guard !isRunning else { return }

        let outputFormat = avEngine.outputNode.outputFormat(forBus: 0)
        let sampleRate = outputFormat.sampleRate > 0 ? outputFormat.sampleRate : 48000.0

        // Prepare the engine if not already done
        engine.prepare(sampleRate: sampleRate)

        // Create the render format (stereo float)
        guard let format = AVAudioFormat(
            commonFormat: .pcmFormatFloat32,
            sampleRate: sampleRate,
            channels: 2,
            interleaved: false
        ) else {
            throw HyasynthError.audioFormatError
        }

        // Capture self weakly for the render callback
        let hyasynthEngine = engine

        // Create the source node with render callback
        let source = AVAudioSourceNode(format: format) { _, _, frameCount, audioBufferList -> OSStatus in
            let bufferList = UnsafeMutableAudioBufferListPointer(audioBufferList)

            // Process any pending commands
            _ = hyasynthEngine.processCommands()

            // Get buffer pointers
            guard bufferList.count >= 2,
                  let leftBuffer = bufferList[0].mData?.assumingMemoryBound(to: Float.self),
                  let rightBuffer = bufferList[1].mData?.assumingMemoryBound(to: Float.self) else {
                // Fill with silence if buffer setup is wrong
                for buffer in bufferList {
                    if let data = buffer.mData {
                        memset(data, 0, Int(buffer.mDataByteSize))
                    }
                }
                return noErr
            }

            // Render audio
            hyasynthEngine.render(frames: frameCount, left: leftBuffer, right: rightBuffer)

            return noErr
        }

        sourceNode = source

        // Connect nodes: source -> output
        avEngine.attach(source)
        avEngine.connect(source, to: avEngine.mainMixerNode, format: format)

        // Start the engine
        try avEngine.start()
        isRunning = true
    }

    /// Stop audio playback.
    public func stop() {
        guard isRunning else { return }

        avEngine.stop()

        if let source = sourceNode {
            avEngine.detach(source)
            sourceNode = nil
        }

        isRunning = false
    }

    /// Pause audio playback (keeps engine attached).
    public func pause() {
        avEngine.pause()
    }

    /// Resume audio playback after pause.
    public func resume() throws {
        try avEngine.start()
    }
}

/// Errors that can occur in Hyasynth audio operations.
public enum HyasynthError: Error {
    case audioFormatError
    case engineNotReady
    case compilationFailed
}

#endif
