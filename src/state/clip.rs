// Clip definitions and the unified event model.
//
// This module contains the core data structures for representing
// musical content: MIDI notes and audio regions, unified into a
// single event stream model.
//
// Key concepts:
// - NoteDef: A MIDI note event
// - AudioRegionDef: A reference to audio in the pool
// - ClipEvent: Either a Note or AudioRegion (unified event model)
// - ClipDef: A container holding a stream of ClipEvents

use std::collections::HashMap;
use std::sync::Arc;

/// Unique identifier for a clip.
pub type ClipId = u32;

/// Unique identifier for an audio pool entry.
pub type AudioPoolId = u32;

// ═══════════════════════════════════════════════════════════════════════════
// Audio Pool - Storage for recorded/imported audio
// ═══════════════════════════════════════════════════════════════════════════

/// Audio sample data stored in the pool.
///
/// This is the actual waveform data that can be referenced by multiple clips.
/// Using Arc allows cheap cloning and sharing across clips.
#[derive(Debug, Clone)]
pub struct AudioPoolEntry {
    /// Unique ID in the pool.
    pub id: AudioPoolId,

    /// Display name (usually the file name).
    pub name: String,

    /// Sample rate of the audio.
    pub sample_rate: f64,

    /// Number of channels (1 = mono, 2 = stereo).
    pub channels: usize,

    /// Total number of frames.
    pub frames: usize,

    /// The actual sample data (interleaved if stereo).
    /// Wrapped in Arc for efficient sharing across clips.
    pub samples: Arc<Vec<f32>>,
}

impl AudioPoolEntry {
    pub fn new(
        id: AudioPoolId,
        name: impl Into<String>,
        sample_rate: f64,
        channels: usize,
        samples: Vec<f32>,
    ) -> Self {
        let frames = samples.len() / channels;
        Self {
            id,
            name: name.into(),
            sample_rate,
            channels,
            frames,
            samples: Arc::new(samples),
        }
    }

    /// Duration in seconds.
    pub fn duration_seconds(&self) -> f64 {
        self.frames as f64 / self.sample_rate
    }

    /// Duration in beats at a given tempo.
    pub fn duration_beats(&self, bpm: f64) -> f64 {
        self.duration_seconds() * bpm / 60.0
    }
}

/// The audio pool stores all recorded/imported audio.
#[derive(Debug, Clone, Default)]
pub struct AudioPool {
    entries: HashMap<AudioPoolId, AudioPoolEntry>,
    next_id: AudioPoolId,
}

impl AudioPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add audio to the pool, returning its ID.
    pub fn add(
        &mut self,
        name: impl Into<String>,
        sample_rate: f64,
        channels: usize,
        samples: Vec<f32>,
    ) -> AudioPoolId {
        let id = self.next_id;
        self.next_id += 1;
        self.entries
            .insert(id, AudioPoolEntry::new(id, name, sample_rate, channels, samples));
        id
    }

    /// Get audio by ID.
    pub fn get(&self, id: AudioPoolId) -> Option<&AudioPoolEntry> {
        self.entries.get(&id)
    }

    /// Remove audio from the pool.
    pub fn remove(&mut self, id: AudioPoolId) -> Option<AudioPoolEntry> {
        self.entries.remove(&id)
    }

    /// Iterate over all entries.
    pub fn iter(&self) -> impl Iterator<Item = &AudioPoolEntry> {
        self.entries.values()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Clip Events - Unified event model
// ═══════════════════════════════════════════════════════════════════════════

/// A MIDI note event within a clip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteDef {
    /// Start position in beats (relative to clip start).
    pub start: f64,

    /// Duration in beats.
    pub duration: f64,

    /// MIDI note number (0-127).
    pub note: u8,

    /// Velocity (0.0 - 1.0).
    pub velocity: f32,
}

impl NoteDef {
    pub fn new(start: f64, duration: f64, note: u8, velocity: f32) -> Self {
        Self {
            start,
            duration,
            note,
            velocity: velocity.clamp(0.0, 1.0),
        }
    }

    /// End position in beats.
    pub fn end(&self) -> f64 {
        self.start + self.duration
    }
}

/// An audio region event within a clip.
///
/// References audio from the pool and specifies how it should be played.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioRegionDef {
    /// Start position in beats (relative to clip start).
    pub start: f64,

    /// Duration in beats (how long to play).
    pub duration: f64,

    /// Reference to audio in the pool.
    pub audio_id: AudioPoolId,

    /// Offset into the source audio (in beats at the clip's tempo).
    /// Allows trimming the start of the audio.
    pub source_offset: f64,

    /// Gain/volume for this region (0.0 - 1.0+).
    pub gain: f32,

    /// Pitch shift in semitones (0 = no change).
    pub pitch_shift: f32,

    /// Whether to time-stretch to fit duration (vs. just trimming).
    pub time_stretch: bool,
}

impl AudioRegionDef {
    pub fn new(start: f64, duration: f64, audio_id: AudioPoolId) -> Self {
        Self {
            start,
            duration,
            audio_id,
            source_offset: 0.0,
            gain: 1.0,
            pitch_shift: 0.0,
            time_stretch: false,
        }
    }

    /// End position in beats.
    pub fn end(&self) -> f64 {
        self.start + self.duration
    }

    /// Builder: set source offset.
    pub fn with_offset(mut self, offset: f64) -> Self {
        self.source_offset = offset;
        self
    }

    /// Builder: set gain.
    pub fn with_gain(mut self, gain: f32) -> Self {
        self.gain = gain;
        self
    }

    /// Builder: set pitch shift.
    pub fn with_pitch(mut self, semitones: f32) -> Self {
        self.pitch_shift = semitones;
        self
    }

    /// Builder: enable time stretching.
    pub fn with_stretch(mut self, enabled: bool) -> Self {
        self.time_stretch = enabled;
        self
    }
}

/// A unified clip event - can be either MIDI or audio.
///
/// This is the core of the unified signal model. Both note events
/// and audio regions are treated as events in the same stream.
#[derive(Debug, Clone, PartialEq)]
pub enum ClipEvent {
    /// A MIDI note event.
    Note(NoteDef),

    /// An audio region event.
    Audio(AudioRegionDef),
}

impl ClipEvent {
    /// Get the start position in beats.
    pub fn start(&self) -> f64 {
        match self {
            ClipEvent::Note(n) => n.start,
            ClipEvent::Audio(a) => a.start,
        }
    }

    /// Get the end position in beats.
    pub fn end(&self) -> f64 {
        match self {
            ClipEvent::Note(n) => n.end(),
            ClipEvent::Audio(a) => a.end(),
        }
    }

    /// Get the duration in beats.
    pub fn duration(&self) -> f64 {
        match self {
            ClipEvent::Note(n) => n.duration,
            ClipEvent::Audio(a) => a.duration,
        }
    }

    /// Check if this event overlaps a time range.
    pub fn overlaps(&self, start: f64, end: f64) -> bool {
        self.start() < end && self.end() > start
    }

    /// Is this a note event?
    pub fn is_note(&self) -> bool {
        matches!(self, ClipEvent::Note(_))
    }

    /// Is this an audio event?
    pub fn is_audio(&self) -> bool {
        matches!(self, ClipEvent::Audio(_))
    }

    /// Get as a note (if it is one).
    pub fn as_note(&self) -> Option<&NoteDef> {
        match self {
            ClipEvent::Note(n) => Some(n),
            _ => None,
        }
    }

    /// Get as an audio region (if it is one).
    pub fn as_audio(&self) -> Option<&AudioRegionDef> {
        match self {
            ClipEvent::Audio(a) => Some(a),
            _ => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Clips - Unified containers for events
// ═══════════════════════════════════════════════════════════════════════════

/// A clip containing a stream of events (MIDI and/or audio).
///
/// Clips can contain any mix of note and audio events. They can be
/// placed on tracks at specific positions, or triggered from the
/// session view's clip launcher.
#[derive(Debug, Clone)]
pub struct ClipDef {
    /// Unique clip ID.
    pub id: ClipId,

    /// Display name.
    pub name: String,

    /// Length in beats.
    pub length: f64,

    /// All events in this clip (sorted by start time).
    pub events: Vec<ClipEvent>,

    /// Color for UI display (RGBA).
    pub color: u32,

    /// Whether the clip loops when played.
    pub looping: bool,
}

impl ClipDef {
    pub fn new(id: ClipId, name: impl Into<String>, length: f64) -> Self {
        Self {
            id,
            name: name.into(),
            length,
            events: Vec::new(),
            color: 0xFF5500FF, // Orange default
            looping: true,
        }
    }

    /// Sort events by start time.
    fn sort_events(&mut self) {
        self.events
            .sort_by(|a, b| a.start().partial_cmp(&b.start()).unwrap());
    }

    /// Add an event to the clip.
    pub fn add_event(&mut self, event: ClipEvent) {
        self.events.push(event);
        self.sort_events();
    }

    /// Add a note to the clip (convenience method).
    pub fn add_note(&mut self, note: NoteDef) {
        self.add_event(ClipEvent::Note(note));
    }

    /// Add an audio region to the clip (convenience method).
    pub fn add_audio(&mut self, region: AudioRegionDef) {
        self.add_event(ClipEvent::Audio(region));
    }

    /// Remove an event by index.
    pub fn remove_event(&mut self, index: usize) -> Option<ClipEvent> {
        if index < self.events.len() {
            Some(self.events.remove(index))
        } else {
            None
        }
    }

    /// Remove a note by index (for backwards compatibility).
    pub fn remove_note(&mut self, index: usize) -> Option<NoteDef> {
        // Find the nth note event
        let note_indices: Vec<usize> = self
            .events
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_note())
            .map(|(i, _)| i)
            .collect();

        if index < note_indices.len() {
            if let Some(ClipEvent::Note(n)) = self.events.get(note_indices[index]).cloned() {
                self.events.remove(note_indices[index]);
                return Some(n);
            }
        }
        None
    }

    /// Get all events that overlap a time range.
    pub fn events_in_range(&self, start: f64, end: f64) -> impl Iterator<Item = &ClipEvent> {
        self.events.iter().filter(move |e| e.overlaps(start, end))
    }

    /// Get only note events.
    pub fn notes(&self) -> impl Iterator<Item = &NoteDef> {
        self.events.iter().filter_map(|e| e.as_note())
    }

    /// Get only audio events.
    pub fn audio_regions(&self) -> impl Iterator<Item = &AudioRegionDef> {
        self.events.iter().filter_map(|e| e.as_audio())
    }

    /// Get notes in a time range (for backwards compatibility).
    pub fn notes_in_range(&self, start: f64, end: f64) -> impl Iterator<Item = &NoteDef> {
        self.notes().filter(move |n| n.start < end && n.end() > start)
    }

    /// Get audio regions in a time range.
    pub fn audio_in_range(&self, start: f64, end: f64) -> impl Iterator<Item = &AudioRegionDef> {
        self.audio_regions()
            .filter(move |a| a.start < end && a.end() > start)
    }

    /// Clear all events.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Clear only notes.
    pub fn clear_notes(&mut self) {
        self.events.retain(|e| !e.is_note());
    }

    /// Clear only audio.
    pub fn clear_audio(&mut self) {
        self.events.retain(|e| !e.is_audio());
    }

    /// Check if this clip contains any audio.
    pub fn has_audio(&self) -> bool {
        self.events.iter().any(|e| e.is_audio())
    }

    /// Check if this clip contains any notes.
    pub fn has_notes(&self) -> bool {
        self.events.iter().any(|e| e.is_note())
    }

    /// Get the number of note events.
    pub fn note_count(&self) -> usize {
        self.events.iter().filter(|e| e.is_note()).count()
    }

    /// Get the number of audio events.
    pub fn audio_count(&self) -> usize {
        self.events.iter().filter(|e| e.is_audio()).count()
    }
}

