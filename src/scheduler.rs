// src/scheduler.rs

use crate::event::{Event, MusicalEvent};
use crate::execution_plan::SlicePlan;
use crate::plan_handoff::PlanHandoff;
use crate::transport::{MusicalTransport, Transport};

/// Compiles musical-time intent into sample-accurate execution plans.
///
/// This struct is NOT real-time safe.
/// It must never be accessed from the audio thread.
pub struct Scheduler {
    /// Musical-time transport (beats, tempo, etc.)
    musical_transport: MusicalTransport,
}

impl Scheduler {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            musical_transport: MusicalTransport::new(sample_rate),
        }
    }

    /// Compile the next audio block.
    ///
    /// This function:
    /// - advances musical transport
    /// - applies musical events at correct sample boundaries
    /// - emits event-free, transport-stable slices
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
        plan.slices.clear();

        // Sort events by beat (defensive; caller *should* already do this)
        let mut events = musical_events.to_vec();
        events.sort_by(|a, b| {
            self.musical_transport
                .event_sample_position(a)
                .partial_cmp(&self.musical_transport.event_sample_position(b))
                .unwrap()
        });

        let mut event_index = 0;
        let mut cursor_sample = block_start_sample;

        while cursor_sample < block_end_sample {
            // ------------------------------------------------------------
            // 1. Determine next boundary (event or transport change)
            // ------------------------------------------------------------

            let next_event_sample = events
                .get(event_index)
                .and_then(|e| self.musical_transport.event_sample_position(e))
                .unwrap_or(block_end_sample);

            // TODO: Implement block-leveltransport changes
            // let next_transport_sample = self.musical_transport.next_transport_change_sample();

            let slice_end_sample = next_event_sample
                // .min(next_transport_sample)
                .min(block_end_sample);

            let slice_frames = (slice_end_sample - cursor_sample) as usize;

            // ------------------------------------------------------------
            // Queue all events at this boundary
            // ------------------------------------------------------------

            let mut pre_slice_events = Vec::new();
            while let Some(event) = events.get(event_index) {
                let event_sample = self
                    .musical_transport
                    .event_sample_position(event)
                    .unwrap_or(u64::MAX);

                if event_sample == cursor_sample {
                    pre_slice_events.push(Self::compile_event(event).unwrap());
                    event_index += 1;
                } else {
                    break;
                }
            }

            // ------------------------------------------------------------
            // Emit slice if it has non-zero duration
            // ------------------------------------------------------------

            if slice_frames > 0 {
                let transport: Transport = self.musical_transport.resolve_transport();

                plan.slices.push(SlicePlan {
                    start_sample: cursor_sample,
                    frame_count: slice_frames,
                    transport,
                    events: pre_slice_events,
                });

                self.musical_transport.advance_samples(slice_frames);
                cursor_sample += slice_frames as u64;
            }
        }

        debug_assert!(
            plan.slices.iter().map(|s| s.frame_count).sum::<usize>() == plan.block_frames
        );

        handoff.publish();
    }

    /// Convert a musical event into an engine event.
    ///
    /// This is the ONLY place where musical intent crosses
    /// into engine-executable instructions.
    #[inline]
    fn compile_event(event: &MusicalEvent) -> Option<Event> {
        match event {
            MusicalEvent::NoteOn { note, velocity, .. } => Some(Event::NoteOn {
                note: *note,
                velocity: *velocity,
            }),

            MusicalEvent::NoteOff { note, .. } => Some(Event::NoteOff { note: *note }),

            MusicalEvent::ParamChange {
                param_id, value, ..
            } => Some(Event::ParamChange {
                param_id: *param_id,
                value: *value,
            }),
        }
    }
}
