// Clip playback engine.
//
// This module handles the conversion of playing clips into musical events.
// It tracks active clips on each track and generates NoteOn/NoteOff and
// audio playback events at the correct beat positions.
//
// Key responsibilities:
// - Track which clips are playing on which tracks
// - Generate note events from MIDI content in clips
// - Generate audio playback events from audio regions in clips
// - Handle clip looping
// - Track active notes for proper note-off generation

use std::collections::HashMap;

use crate::event::MusicalEvent;
use crate::state::{Arrangement, AudioPool, ClipDef, ClipId, NoteDef, TrackId};

/// Unique identifier for an active note (for tracking note-offs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ActiveNote {
    track_id: TrackId,
    clip_id: ClipId,
    target_node: u32,
    note: u8,
}

/// Active note with end time (not hashable due to f64).
#[derive(Debug, Clone, Copy)]
struct ActiveNoteState {
    key: ActiveNote,
    /// The beat position when this note should end.
    end_beat: f64,
}

/// State for an actively playing clip on a track.
#[derive(Debug, Clone)]
struct PlayingClip {
    clip_id: ClipId,
    track_id: TrackId,
    /// Beat position when clip playback started.
    start_beat: f64,
    /// Current playhead position within the clip (0.0 = clip start).
    clip_position: f64,
}

impl PlayingClip {
    fn new(clip_id: ClipId, track_id: TrackId, start_beat: f64) -> Self {
        Self {
            clip_id,
            track_id,
            start_beat,
            clip_position: 0.0,
        }
    }
}

/// Clip playback engine.
///
/// Maintains state about which clips are playing and generates events.
pub struct ClipPlayback {
    /// Currently playing clips (track_id -> PlayingClip).
    playing: HashMap<TrackId, PlayingClip>,

    /// Active notes that need note-off events.
    active_notes: Vec<ActiveNoteState>,

    /// Sample rate (for audio calculations).
    sample_rate: f64,

    /// Scratch buffer for generated events.
    event_buffer: Vec<MusicalEvent>,
}

impl ClipPlayback {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            playing: HashMap::new(),
            active_notes: Vec::with_capacity(32),
            sample_rate,
            event_buffer: Vec::with_capacity(64),
        }
    }

    /// Start playing a clip on a track.
    pub fn start_clip(&mut self, clip_id: ClipId, track_id: TrackId, current_beat: f64) {
        // Stop any currently playing clip on this track
        self.stop_track(track_id, current_beat);

        // Start the new clip
        self.playing
            .insert(track_id, PlayingClip::new(clip_id, track_id, current_beat));
    }

    /// Stop the clip playing on a track.
    pub fn stop_track(&mut self, track_id: TrackId, _current_beat: f64) {
        // Remove the playing clip
        if let Some(playing) = self.playing.remove(&track_id) {
            // Remove active notes from this clip
            self.active_notes
                .retain(|n| !(n.key.track_id == track_id && n.key.clip_id == playing.clip_id));
        }
    }

    /// Stop all clips.
    pub fn stop_all(&mut self) {
        self.playing.clear();
        self.active_notes.clear();
    }

    /// Sync playing clips with arrangement state.
    ///
    /// This updates internal state to match which clips are marked as playing
    /// in the arrangement.
    pub fn sync_with_arrangement(&mut self, arrangement: &Arrangement, current_beat: f64) {
        // Find clips that should start
        for (track_id, clip_id) in &arrangement.playing_clips {
            if !self.playing.contains_key(track_id) {
                self.start_clip(*clip_id, *track_id, current_beat);
            } else if let Some(playing) = self.playing.get(track_id) {
                // Check if a different clip should be playing
                if playing.clip_id != *clip_id {
                    self.start_clip(*clip_id, *track_id, current_beat);
                }
            }
        }

        // Find clips that should stop
        let tracks_to_stop: Vec<_> = self
            .playing
            .keys()
            .filter(|track_id| !arrangement.playing_clips.contains_key(track_id))
            .copied()
            .collect();

        for track_id in tracks_to_stop {
            self.stop_track(track_id, current_beat);
        }
    }

    /// Generate events for a time range.
    ///
    /// This is the main entry point for the scheduler to get events from playing clips.
    ///
    /// # Arguments
    /// * `arrangement` - The arrangement containing clips, tracks, and audio pool
    /// * `start_beat` - Start of the time range
    /// * `end_beat` - End of the time range
    /// * `bpm` - Current tempo (for audio timing)
    ///
    /// # Returns
    /// A slice of generated events (valid until next call).
    pub fn generate_events(
        &mut self,
        arrangement: &Arrangement,
        start_beat: f64,
        end_beat: f64,
        bpm: f64,
    ) -> &[MusicalEvent] {
        self.event_buffer.clear();

        let beat_duration = end_beat - start_beat;

        // Collect track IDs to process (to avoid borrow conflicts)
        let track_ids: Vec<TrackId> = self.playing.keys().copied().collect();

        // Generate events for each playing clip
        for track_id in track_ids {
            let Some(playing) = self.playing.get(&track_id) else {
                continue;
            };

            let Some(clip) = arrangement.get_clip(playing.clip_id) else {
                continue;
            };

            let Some(track) = arrangement.get_track(playing.track_id) else {
                continue;
            };

            // Check if track is audible (handles mute/solo)
            if !arrangement.is_track_audible(playing.track_id) {
                continue;
            }

            // Get target node for this track
            let Some(target_node) = track.target_node else {
                continue;
            };

            // Capture values needed for event generation
            let clip_id = playing.clip_id;
            let clip_position = playing.clip_position;
            let clip_length = clip.length;
            let clip_looping = clip.looping;

            // Generate events from this clip
            self.generate_clip_events_inline(
                track_id,
                clip_id,
                clip_position,
                clip,
                target_node,
                &arrangement.audio_pool,
                start_beat,
                end_beat,
                bpm,
            );

            // Update clip position
            if let Some(playing) = self.playing.get_mut(&track_id) {
                playing.clip_position += beat_duration;

                // Handle looping
                if clip_looping && playing.clip_position >= clip_length {
                    playing.clip_position %= clip_length;
                }
            }
        }

        // Generate note-offs for notes that end in this range
        self.generate_note_offs(start_beat, end_beat);

        &self.event_buffer
    }

    /// Generate events from a single clip (inline version to avoid borrow issues).
    fn generate_clip_events_inline(
        &mut self,
        track_id: TrackId,
        clip_id: ClipId,
        clip_position: f64,
        clip: &ClipDef,
        target_node: u32,
        audio_pool: &AudioPool,
        start_beat: f64,
        end_beat: f64,
        bpm: f64,
    ) {
        let clip_start = clip_position;
        let clip_end = clip_position + (end_beat - start_beat);

        // Handle non-looping clips that have ended
        if !clip.looping && clip_start >= clip.length {
            return;
        }

        // Generate note events
        for note_def in clip.notes() {
            self.generate_note_event_inline(
                track_id,
                clip_id,
                clip_position,
                note_def,
                target_node,
                clip,
                clip_start,
                clip_end,
                start_beat,
            );
        }

        // Generate audio events
        for audio_def in clip.audio_regions() {
            self.generate_audio_event_inline(
                audio_def,
                target_node,
                audio_pool,
                clip,
                clip_start,
                clip_end,
                start_beat,
                bpm,
            );
        }
    }

    /// Generate a note event if it falls within the time range.
    fn generate_note_event_inline(
        &mut self,
        track_id: TrackId,
        clip_id: ClipId,
        _clip_position: f64,
        note: &NoteDef,
        target_node: u32,
        clip: &ClipDef,
        clip_start: f64,
        clip_end: f64,
        block_start_beat: f64,
    ) {
        // Check if note starts in this range
        let note_start = note.start;

        // Handle looping: check if note should trigger
        let should_trigger = if clip.looping {
            // For looping clips, check if the note start falls in the current window
            // considering wrap-around
            let wrapped_start = clip_start % clip.length;
            let wrapped_end = clip_end % clip.length;

            if wrapped_start <= wrapped_end {
                note_start >= wrapped_start && note_start < wrapped_end
            } else {
                // Wrapped around clip boundary
                note_start >= wrapped_start || note_start < wrapped_end
            }
        } else {
            note_start >= clip_start && note_start < clip_end
        };

        if should_trigger {
            // Calculate the absolute beat when this note should trigger
            let offset_in_block = if clip.looping {
                let wrapped_start = clip_start % clip.length;
                if note_start >= wrapped_start {
                    note_start - wrapped_start
                } else {
                    (clip.length - wrapped_start) + note_start
                }
            } else {
                note_start - clip_start
            };

            let absolute_beat = block_start_beat + offset_in_block;

            // Generate note-on
            self.event_buffer.push(MusicalEvent::NoteOnTarget {
                beat: absolute_beat,
                node_id: target_node,
                note: note.note,
                velocity: note.velocity,
            });

            // Track this note for note-off generation
            let end_beat = absolute_beat + note.duration;
            self.active_notes.push(ActiveNoteState {
                key: ActiveNote {
                    track_id,
                    clip_id,
                    target_node,
                    note: note.note,
                },
                end_beat,
            });
        }
    }

    /// Generate an audio playback event if it falls within the time range.
    fn generate_audio_event_inline(
        &mut self,
        audio_def: &crate::state::AudioRegionDef,
        target_node: u32,
        audio_pool: &AudioPool,
        clip: &ClipDef,
        clip_start: f64,
        clip_end: f64,
        block_start_beat: f64,
        bpm: f64,
    ) {
        // Get audio info
        let Some(audio_entry) = audio_pool.get(audio_def.audio_id) else {
            return;
        };

        // Check if audio region starts in this range (similar to note logic)
        let audio_start = audio_def.start;

        let should_trigger = if clip.looping {
            let wrapped_start = clip_start % clip.length;
            let wrapped_end = clip_end % clip.length;

            if wrapped_start <= wrapped_end {
                audio_start >= wrapped_start && audio_start < wrapped_end
            } else {
                audio_start >= wrapped_start || audio_start < wrapped_end
            }
        } else {
            audio_start >= clip_start && audio_start < clip_end
        };

        if should_trigger {
            let offset_in_block = if clip.looping {
                let wrapped_start = clip_start % clip.length;
                if audio_start >= wrapped_start {
                    audio_start - wrapped_start
                } else {
                    (clip.length - wrapped_start) + audio_start
                }
            } else {
                audio_start - clip_start
            };

            let absolute_beat = block_start_beat + offset_in_block;

            // Convert beats to samples
            let beat_to_seconds = 60.0 / bpm;
            let source_offset_seconds = audio_def.source_offset * beat_to_seconds;
            let duration_seconds = audio_def.duration * beat_to_seconds;

            let start_sample = (source_offset_seconds * audio_entry.sample_rate) as u64;
            let duration_samples = (duration_seconds * audio_entry.sample_rate) as u64;

            self.event_buffer.push(MusicalEvent::AudioStart {
                beat: absolute_beat,
                node_id: target_node,
                audio_id: audio_def.audio_id,
                start_sample,
                duration_samples,
                gain: audio_def.gain,
            });
        }
    }

    /// Generate note-off events for notes that end in this range.
    fn generate_note_offs(&mut self, start_beat: f64, end_beat: f64) {
        // Partition: notes ending in this range vs notes to keep
        let mut i = 0;
        while i < self.active_notes.len() {
            let state = &self.active_notes[i];
            if state.end_beat >= start_beat && state.end_beat < end_beat {
                // Generate note-off at the correct beat
                self.event_buffer.push(MusicalEvent::NoteOffTarget {
                    beat: state.end_beat,
                    node_id: state.key.target_node,
                    note: state.key.note,
                });
                // Remove this note (swap-remove for efficiency)
                self.active_notes.swap_remove(i);
                // Don't increment i, check the swapped element
            } else {
                i += 1;
            }
        }
    }

    /// Generate stop events for all active notes (for when stopping playback).
    pub fn generate_stop_events(&mut self, current_beat: f64) -> Vec<MusicalEvent> {
        let events: Vec<_> = self
            .active_notes
            .iter()
            .map(|state| MusicalEvent::NoteOffTarget {
                beat: current_beat,
                node_id: state.key.target_node,
                note: state.key.note,
            })
            .collect();

        self.active_notes.clear();
        events
    }

    /// Check if any clips are currently playing.
    pub fn is_playing(&self) -> bool {
        !self.playing.is_empty()
    }

    /// Get the number of active notes.
    pub fn active_note_count(&self) -> usize {
        self.active_notes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ClipDef, NoteDef};

    fn make_test_arrangement() -> Arrangement {
        let mut arr = Arrangement::new();

        // Create a track
        let track_id = arr.create_track("Test Track");
        arr.set_track_target(track_id, Some(100)); // Target node 100

        // Create a clip with some notes
        let clip_id = arr.create_clip("Test Clip", 4.0); // 4 beats
        if let Some(clip) = arr.get_clip_mut(clip_id) {
            clip.add_note(NoteDef::new(0.0, 1.0, 60, 0.8)); // C4 at beat 0
            clip.add_note(NoteDef::new(1.0, 1.0, 62, 0.7)); // D4 at beat 1
            clip.add_note(NoteDef::new(2.0, 2.0, 64, 0.9)); // E4 at beat 2, 2 beats long
        }

        // Set clip to play
        arr.launch_clip(track_id, clip_id);

        arr
    }

    #[test]
    fn test_clip_playback_sync() {
        let mut playback = ClipPlayback::new(48000.0);
        let arr = make_test_arrangement();

        // Sync should start the clip
        playback.sync_with_arrangement(&arr, 0.0);
        assert!(playback.is_playing());
    }

    #[test]
    fn test_note_generation() {
        let mut playback = ClipPlayback::new(48000.0);
        let arr = make_test_arrangement();

        playback.sync_with_arrangement(&arr, 0.0);

        // Generate events for first beat
        let events = playback.generate_events(&arr, 0.0, 1.0, 120.0);

        // Should have at least one note-on (C4 at beat 0)
        let note_ons: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, MusicalEvent::NoteOnTarget { .. }))
            .collect();

        assert!(!note_ons.is_empty(), "Should generate note-on events");
    }
}
