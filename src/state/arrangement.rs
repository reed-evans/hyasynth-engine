// src/state/arrangement.rs
//
// Arrangement, tracks, and scenes for session/arrangement views.
//
// This module builds on the unified signal model defined in `clip.rs`
// to provide the full arrangement structure: tracks, scenes, timeline
// placements, and clip scheduling.

use std::collections::HashMap;

use super::clip::{
    AudioPool, AudioPoolEntry, AudioPoolId, AudioRegionDef, ClipDef, ClipId, NoteDef,
};

/// Unique identifier for a track.
pub type TrackId = u32;

/// Unique identifier for a scene.
pub type SceneId = u32;

// ═══════════════════════════════════════════════════════════════════════════
// Tracks
// ═══════════════════════════════════════════════════════════════════════════

/// A track in the arrangement.
///
/// Tracks are vertical lanes that:
/// - Route to a specific node in the graph (for audio output)
/// - Contain clips in the session view (clip slots)
/// - Contain clip placements in the arrangement view
#[derive(Debug, Clone)]
pub struct TrackDef {
    /// Unique track ID.
    pub id: TrackId,

    /// Display name.
    pub name: String,

    /// Volume (0.0 - 1.0, where 1.0 = 0dB).
    pub volume: f32,

    /// Pan (-1.0 = left, 0.0 = center, 1.0 = right).
    pub pan: f32,

    /// Muted state.
    pub mute: bool,

    /// Solo state.
    pub solo: bool,

    /// Armed for recording.
    pub armed: bool,

    /// Color for UI display (RGBA).
    pub color: u32,

    /// The node ID this track routes to (for MIDI output).
    pub target_node: Option<u32>,

    /// Clip slots for session view (index = scene index).
    /// None means empty slot.
    pub clip_slots: Vec<Option<ClipId>>,
}

impl TrackDef {
    pub fn new(id: TrackId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            volume: 0.8,
            pan: 0.0,
            mute: false,
            solo: false,
            armed: false,
            color: 0x3388FFFF, // Blue default
            target_node: None,
            clip_slots: Vec::new(),
        }
    }

    /// Ensure we have enough clip slots for the given scene count.
    pub fn ensure_slots(&mut self, scene_count: usize) {
        while self.clip_slots.len() < scene_count {
            self.clip_slots.push(None);
        }
    }

    /// Set a clip in a slot.
    pub fn set_clip_slot(&mut self, scene_index: usize, clip_id: Option<ClipId>) {
        self.ensure_slots(scene_index + 1);
        self.clip_slots[scene_index] = clip_id;
    }

    /// Get the clip in a slot.
    pub fn get_clip_slot(&self, scene_index: usize) -> Option<ClipId> {
        self.clip_slots.get(scene_index).copied().flatten()
    }
}

/// A clip placement in the arrangement timeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipPlacement {
    /// The clip to play.
    pub clip_id: ClipId,

    /// Start position on the timeline (in beats).
    pub start_beat: f64,

    /// Optional end position (for trimmed clips).
    /// If None, plays the full clip length.
    pub end_beat: Option<f64>,

    /// Offset into the clip (for starting mid-clip).
    pub clip_offset: f64,
}

impl ClipPlacement {
    pub fn new(clip_id: ClipId, start_beat: f64) -> Self {
        Self {
            clip_id,
            start_beat,
            end_beat: None,
            clip_offset: 0.0,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Scenes
// ═══════════════════════════════════════════════════════════════════════════

/// A scene (horizontal row of clips in session view).
///
/// Launching a scene triggers all clips in that row simultaneously.
#[derive(Debug, Clone)]
pub struct SceneDef {
    /// Unique scene ID.
    pub id: SceneId,

    /// Display name.
    pub name: String,

    /// Tempo for this scene (optional, overrides session tempo).
    pub tempo: Option<f64>,

    /// Color for UI display (RGBA).
    pub color: u32,
}

impl SceneDef {
    pub fn new(id: SceneId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            tempo: None,
            color: 0x888888FF, // Gray default
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Arrangement
// ═══════════════════════════════════════════════════════════════════════════

/// The complete arrangement state.
///
/// Contains all clips, tracks, scenes, timeline placements, and audio pool.
#[derive(Debug, Clone, Default)]
pub struct Arrangement {
    /// Audio pool - stores all recorded/imported audio samples.
    pub audio_pool: AudioPool,

    /// All clips in the project.
    pub clips: HashMap<ClipId, ClipDef>,

    /// All tracks.
    pub tracks: Vec<TrackDef>,

    /// All scenes.
    pub scenes: Vec<SceneDef>,

    /// Clip placements on the timeline (per track).
    /// Key is track ID.
    pub timeline: HashMap<TrackId, Vec<ClipPlacement>>,

    /// Currently playing clips in session view (track_id -> clip_id).
    pub playing_clips: HashMap<TrackId, ClipId>,

    /// Currently launched scene (if any).
    pub active_scene: Option<SceneId>,

    /// Next available clip ID.
    next_clip_id: ClipId,

    /// Next available track ID.
    next_track_id: TrackId,

    /// Next available scene ID.
    next_scene_id: SceneId,
}

impl Arrangement {
    pub fn new() -> Self {
        Self::default()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Clip Management
    // ─────────────────────────────────────────────────────────────────────────

    /// Create a new empty clip.
    pub fn create_clip(&mut self, name: impl Into<String>, length: f64) -> ClipId {
        let id = self.next_clip_id;
        self.next_clip_id += 1;
        self.clips.insert(id, ClipDef::new(id, name, length));
        id
    }

    /// Get a clip by ID.
    pub fn get_clip(&self, id: ClipId) -> Option<&ClipDef> {
        self.clips.get(&id)
    }

    /// Get a mutable clip by ID.
    pub fn get_clip_mut(&mut self, id: ClipId) -> Option<&mut ClipDef> {
        self.clips.get_mut(&id)
    }

    /// Delete a clip.
    pub fn delete_clip(&mut self, id: ClipId) -> Option<ClipDef> {
        // Remove from all track slots
        for track in &mut self.tracks {
            for slot in &mut track.clip_slots {
                if *slot == Some(id) {
                    *slot = None;
                }
            }
        }

        // Remove from timeline
        for placements in self.timeline.values_mut() {
            placements.retain(|p| p.clip_id != id);
        }

        // Remove from playing clips
        self.playing_clips.retain(|_, clip_id| *clip_id != id);

        self.clips.remove(&id)
    }

    /// Add a note to a clip.
    pub fn add_note_to_clip(&mut self, clip_id: ClipId, note: NoteDef) -> bool {
        if let Some(clip) = self.clips.get_mut(&clip_id) {
            clip.add_note(note);
            true
        } else {
            false
        }
    }

    /// Add an audio region to a clip.
    pub fn add_audio_to_clip(&mut self, clip_id: ClipId, region: AudioRegionDef) -> bool {
        if let Some(clip) = self.clips.get_mut(&clip_id) {
            clip.add_audio(region);
            true
        } else {
            false
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Audio Pool Management
    // ─────────────────────────────────────────────────────────────────────────

    /// Add audio samples to the pool.
    pub fn add_audio_to_pool(
        &mut self,
        name: impl Into<String>,
        sample_rate: f64,
        channels: usize,
        samples: Vec<f32>,
    ) -> AudioPoolId {
        self.audio_pool.add(name, sample_rate, channels, samples)
    }

    /// Get audio from the pool.
    pub fn get_audio(&self, id: AudioPoolId) -> Option<&AudioPoolEntry> {
        self.audio_pool.get(id)
    }

    /// Remove audio from the pool.
    /// Note: This does NOT remove audio regions that reference this audio.
    pub fn remove_audio(&mut self, id: AudioPoolId) -> Option<AudioPoolEntry> {
        self.audio_pool.remove(id)
    }

    /// Create a clip from audio (convenience method).
    ///
    /// Creates a new clip containing a single audio region that spans
    /// the full duration of the audio at the given tempo.
    pub fn create_clip_from_audio(&mut self, audio_id: AudioPoolId, bpm: f64) -> Option<ClipId> {
        let audio = self.audio_pool.get(audio_id)?;
        let duration_beats = audio.duration_beats(bpm);
        let name = audio.name.clone();

        let clip_id = self.create_clip(name, duration_beats);
        if let Some(clip) = self.get_clip_mut(clip_id) {
            clip.add_audio(AudioRegionDef::new(0.0, duration_beats, audio_id));
        }
        Some(clip_id)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Track Management
    // ─────────────────────────────────────────────────────────────────────────

    /// Create a new track.
    pub fn create_track(&mut self, name: impl Into<String>) -> TrackId {
        let id = self.next_track_id;
        self.next_track_id += 1;

        let mut track = TrackDef::new(id, name);
        track.ensure_slots(self.scenes.len());

        self.tracks.push(track);
        self.timeline.insert(id, Vec::new());
        id
    }

    /// Get a track by ID.
    pub fn get_track(&self, id: TrackId) -> Option<&TrackDef> {
        self.tracks.iter().find(|t| t.id == id)
    }

    /// Get a mutable track by ID.
    pub fn get_track_mut(&mut self, id: TrackId) -> Option<&mut TrackDef> {
        self.tracks.iter_mut().find(|t| t.id == id)
    }

    /// Get track by index.
    pub fn get_track_by_index(&self, index: usize) -> Option<&TrackDef> {
        self.tracks.get(index)
    }

    /// Delete a track.
    pub fn delete_track(&mut self, id: TrackId) -> Option<TrackDef> {
        if let Some(pos) = self.tracks.iter().position(|t| t.id == id) {
            self.timeline.remove(&id);
            self.playing_clips.remove(&id);
            Some(self.tracks.remove(pos))
        } else {
            None
        }
    }

    /// Set track volume.
    pub fn set_track_volume(&mut self, id: TrackId, volume: f32) {
        if let Some(track) = self.get_track_mut(id) {
            track.volume = volume.clamp(0.0, 1.0);
        }
    }

    /// Set track pan.
    pub fn set_track_pan(&mut self, id: TrackId, pan: f32) {
        if let Some(track) = self.get_track_mut(id) {
            track.pan = pan.clamp(-1.0, 1.0);
        }
    }

    /// Set track mute.
    pub fn set_track_mute(&mut self, id: TrackId, mute: bool) {
        if let Some(track) = self.get_track_mut(id) {
            track.mute = mute;
        }
    }

    /// Set track solo.
    pub fn set_track_solo(&mut self, id: TrackId, solo: bool) {
        if let Some(track) = self.get_track_mut(id) {
            track.solo = solo;
        }
    }

    /// Set track target node.
    pub fn set_track_target(&mut self, id: TrackId, node_id: Option<u32>) {
        if let Some(track) = self.get_track_mut(id) {
            track.target_node = node_id;
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Scene Management
    // ─────────────────────────────────────────────────────────────────────────

    /// Create a new scene.
    pub fn create_scene(&mut self, name: impl Into<String>) -> SceneId {
        let id = self.next_scene_id;
        self.next_scene_id += 1;
        self.scenes.push(SceneDef::new(id, name));

        // Ensure all tracks have enough slots
        for track in &mut self.tracks {
            track.ensure_slots(self.scenes.len());
        }

        id
    }

    /// Get a scene by ID.
    pub fn get_scene(&self, id: SceneId) -> Option<&SceneDef> {
        self.scenes.iter().find(|s| s.id == id)
    }

    /// Get scene by index.
    pub fn get_scene_by_index(&self, index: usize) -> Option<&SceneDef> {
        self.scenes.get(index)
    }

    /// Delete a scene.
    pub fn delete_scene(&mut self, id: SceneId) -> Option<SceneDef> {
        if let Some(pos) = self.scenes.iter().position(|s| s.id == id) {
            // Remove clip slots from all tracks at this index
            for track in &mut self.tracks {
                if pos < track.clip_slots.len() {
                    track.clip_slots.remove(pos);
                }
            }

            if self.active_scene == Some(id) {
                self.active_scene = None;
            }

            Some(self.scenes.remove(pos))
        } else {
            None
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Clip Slots (Session View)
    // ─────────────────────────────────────────────────────────────────────────

    /// Assign a clip to a slot.
    pub fn set_clip_slot(
        &mut self,
        track_id: TrackId,
        scene_index: usize,
        clip_id: Option<ClipId>,
    ) {
        if let Some(track) = self.get_track_mut(track_id) {
            track.set_clip_slot(scene_index, clip_id);
        }
    }

    /// Get the clip in a slot.
    pub fn get_clip_slot(&self, track_id: TrackId, scene_index: usize) -> Option<ClipId> {
        self.get_track(track_id)
            .and_then(|t| t.get_clip_slot(scene_index))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Timeline (Arrangement View)
    // ─────────────────────────────────────────────────────────────────────────

    /// Schedule a clip on the timeline.
    pub fn schedule_clip(&mut self, track_id: TrackId, clip_id: ClipId, start_beat: f64) {
        if let Some(placements) = self.timeline.get_mut(&track_id) {
            placements.push(ClipPlacement::new(clip_id, start_beat));
            placements.sort_by(|a, b| a.start_beat.partial_cmp(&b.start_beat).unwrap());
        }
    }

    /// Remove a clip placement from the timeline.
    pub fn remove_clip_placement(&mut self, track_id: TrackId, start_beat: f64) {
        if let Some(placements) = self.timeline.get_mut(&track_id) {
            placements.retain(|p| (p.start_beat - start_beat).abs() > 0.001);
        }
    }

    /// Get clip placements in a time range for a track.
    pub fn placements_in_range(
        &self,
        track_id: TrackId,
        start: f64,
        end: f64,
    ) -> Vec<&ClipPlacement> {
        self.timeline
            .get(&track_id)
            .map(|placements| {
                placements
                    .iter()
                    .filter(|p| {
                        // Get clip length to check overlap
                        let clip_end = self
                            .clips
                            .get(&p.clip_id)
                            .map(|c| p.start_beat + c.length)
                            .unwrap_or(p.start_beat);
                        p.start_beat < end && clip_end > start
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Playback Control
    // ─────────────────────────────────────────────────────────────────────────

    /// Launch a clip on a track (session view).
    pub fn launch_clip(&mut self, track_id: TrackId, clip_id: ClipId) {
        self.playing_clips.insert(track_id, clip_id);
    }

    /// Stop a clip on a track.
    pub fn stop_clip(&mut self, track_id: TrackId) {
        self.playing_clips.remove(&track_id);
    }

    /// Launch a scene (trigger all clips in that row).
    pub fn launch_scene(&mut self, scene_index: usize) {
        if scene_index >= self.scenes.len() {
            return;
        }

        self.active_scene = Some(self.scenes[scene_index].id);

        // Launch all clips in this scene row
        for track in &self.tracks {
            if let Some(clip_id) = track.get_clip_slot(scene_index) {
                self.playing_clips.insert(track.id, clip_id);
            } else {
                self.playing_clips.remove(&track.id);
            }
        }
    }

    /// Stop all clips.
    pub fn stop_all(&mut self) {
        self.playing_clips.clear();
        self.active_scene = None;
    }

    /// Check if any solo tracks are active.
    pub fn has_solo(&self) -> bool {
        self.tracks.iter().any(|t| t.solo)
    }

    /// Check if a track should be audible (considering mute/solo).
    pub fn is_track_audible(&self, track_id: TrackId) -> bool {
        if let Some(track) = self.get_track(track_id) {
            if track.mute {
                return false;
            }
            if self.has_solo() {
                return track.solo;
            }
            true
        } else {
            false
        }
    }
}
