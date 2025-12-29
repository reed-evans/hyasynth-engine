// src/event.rs

/// ===============================
/// Scheduler-side musical events
/// ===============================

/// An event expressed in musical time (beats).
///
/// These events:
/// - live ONLY in the scheduler
/// - are NOT real-time safe
/// - may be cloned, reordered, quantized, etc.
#[derive(Debug, Clone)]
pub enum MusicalEvent {
    NoteOn {
        beat: f64,
        note: u8,
        velocity: f32,
    },

    NoteOff {
        beat: f64,
        note: u8,
    },

    ParamChange {
        beat: f64,
        node_id: u32,
        param_id: u32,
        value: f32,
    },
}

impl MusicalEvent {
    pub fn beat(&self) -> f64 {
        match self {
            MusicalEvent::NoteOn { beat, .. } => *beat,
            MusicalEvent::NoteOff { beat, .. } => *beat,
            MusicalEvent::ParamChange { beat, .. } => *beat,
        }
    }
}

/// ===============================
/// Engine-side scheduled events
/// ===============================

/// An event expressed in sample time.
///
/// These events:
/// - are RT-safe
/// - contain NO musical-time information
/// - are dispatched by the engine exactly once
#[derive(Debug, Clone)]
pub enum Event {
    NoteOn { 
        note: u8, 
        velocity: f32,
    },

    NoteOff { 
        note: u8,
    },

    ParamChange { 
        node_id: u32,
        param_id: u32, 
        value: f32,
    },
}
