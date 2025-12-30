// src/scheduler.rs

use crate::event::{Event, MusicalEvent};
use crate::execution_plan::SlicePlan;
use crate::plan_handoff::PlanHandoff;
use crate::transport::MusicalTransport;

/// Compiles musical-time intent into sample-accurate execution plans.
///
/// This struct is NOT real-time safe.
/// It must never be accessed from the audio thread.
pub struct Scheduler {
    /// Musical-time transport (beats, tempo, etc.)
    musical_transport: MusicalTransport,

    /// Pre-allocated scratch buffer for sorting events
    event_scratch: Vec<(u64, MusicalEvent)>,

    /// Pre-allocated scratch for compiled events per slice
    compiled_scratch: Vec<Event>,
}

impl Scheduler {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            musical_transport: MusicalTransport::new(sample_rate),
            event_scratch: Vec::with_capacity(64),
            compiled_scratch: Vec::with_capacity(16),
        }
    }

    /// Compile the next audio block.
    pub fn compile_block(
        &mut self,
        handoff: &mut PlanHandoff,
        block_frames: usize,
        musical_events: &[MusicalEvent],
    ) {
        let plan = handoff.write_plan();

        let block_start_sample = self.musical_transport.sample_position();
        let block_end_sample = block_start_sample + block_frames as u64;

        plan.block_start_sample = block_start_sample;
        plan.block_frames = block_frames;
        plan.bpm = self.musical_transport.bpm();
        plan.sample_rate = self.musical_transport.sample_rate();
        plan.slices.clear();

        // Sort events by sample position using scratch buffer
        self.event_scratch.clear();
        for event in musical_events {
            if let Some(sample_pos) = self.musical_transport.event_sample_position(event) {
                if sample_pos >= block_start_sample && sample_pos < block_end_sample {
                    self.event_scratch.push((sample_pos, event.clone()));
                }
            }
        }
        self.event_scratch.sort_by_key(|(pos, _)| *pos);

        // If no events, emit single slice for whole block
        if self.event_scratch.is_empty() {
            plan.slices.push(SlicePlan::new(0, block_frames));
            self.musical_transport.advance_samples(block_frames);
            handoff.publish();
            return;
        }

        // Build slices with events at boundaries
        let mut event_index = 0;
        let mut cursor_frame = 0usize;

        while cursor_frame < block_frames {
            // Collect events at current position
            let cursor_sample = block_start_sample + cursor_frame as u64;
            self.compiled_scratch.clear();

            while event_index < self.event_scratch.len() {
                let (event_sample, _) = &self.event_scratch[event_index];
                if *event_sample == cursor_sample {
                    let (_, event) = &self.event_scratch[event_index];
                    if let Some(compiled) = Self::compile_event(event) {
                        self.compiled_scratch.push(compiled);
                    }
                    event_index += 1;
                } else {
                    break;
                }
            }

            // Find next event boundary (or end of block)
            let next_boundary_frame = self
                .event_scratch
                .get(event_index)
                .map(|(pos, _)| (*pos - block_start_sample) as usize)
                .unwrap_or(block_frames);

            let slice_end_frame = next_boundary_frame.min(block_frames);
            let slice_frames = slice_end_frame - cursor_frame;

            // Emit slice (may have 0 events)
            if slice_frames > 0 {
                let mut slice = SlicePlan::new(cursor_frame, slice_frames);
                slice.events.extend(self.compiled_scratch.drain(..));
                plan.slices.push(slice);
                cursor_frame = slice_end_frame;
            } else {
                // Events at same position as end - attach to last slice if possible
                if !self.compiled_scratch.is_empty() {
                    if let Some(last) = plan.slices.last_mut() {
                        last.events.extend(self.compiled_scratch.drain(..));
                    }
                }
                cursor_frame = slice_end_frame;
            }
        }

        // Advance transport
        self.musical_transport.advance_samples(block_frames);

        debug_assert!(
            plan.slices.iter().map(|s| s.frame_count).sum::<usize>() == plan.block_frames,
            "Slice frames don't sum to block frames: {} != {}",
            plan.slices.iter().map(|s| s.frame_count).sum::<usize>(),
            plan.block_frames
        );

        handoff.publish();
    }

    /// Convert a musical event into an engine event.
    #[inline]
    fn compile_event(event: &MusicalEvent) -> Option<Event> {
        match event {
            MusicalEvent::NoteOn { note, velocity, .. } => Some(Event::NoteOn {
                note: *note,
                velocity: *velocity,
            }),

            MusicalEvent::NoteOff { note, .. } => Some(Event::NoteOff { note: *note }),

            MusicalEvent::NoteOnTarget {
                node_id,
                note,
                velocity,
                ..
            } => Some(Event::NoteOnTarget {
                node_id: *node_id,
                note: *note,
                velocity: *velocity,
            }),

            MusicalEvent::NoteOffTarget { node_id, note, .. } => {
                Some(Event::NoteOffTarget {
                    node_id: *node_id,
                    note: *note,
                })
            }

            MusicalEvent::ParamChange {
                node_id,
                param_id,
                value,
                ..
            } => Some(Event::ParamChange {
                node_id: *node_id,
                param_id: *param_id,
                value: *value,
            }),

            MusicalEvent::AudioStart {
                node_id,
                audio_id,
                start_sample,
                duration_samples,
                gain,
                ..
            } => Some(Event::AudioStart {
                node_id: *node_id,
                audio_id: *audio_id,
                start_sample: *start_sample,
                duration_samples: *duration_samples,
                gain: *gain,
            }),

            MusicalEvent::AudioStop {
                node_id, audio_id, ..
            } => Some(Event::AudioStop {
                node_id: *node_id,
                audio_id: *audio_id,
            }),
        }
    }

    /// Get current beat position
    pub fn beat_position(&self) -> f64 {
        self.musical_transport.beat_position()
    }

    /// Set tempo
    pub fn set_bpm(&mut self, bpm: f64) {
        self.musical_transport.set_bpm(bpm);
    }
}
