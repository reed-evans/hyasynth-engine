use crate::event::MusicalEvent;

//
// ===============================
// MARK: Engine-side transport (RT-safe)
// ===============================
//

/// Transport state expressed purely in the sample domain.
///
/// This struct:
/// - is real-time safe
/// - is copyable
/// - is immutable per slice
/// - contains NO musical-time logic
///
/// Owned and consumed by the audio engine.
#[derive(Debug, Copy, Clone)]
pub struct Transport {
    /// Absolute sample position
    pub sample_pos: u64,

    /// Tempo at slice start (already resolved)
    pub bpm: f64,

    /// Sample rate (Hz)
    pub sample_rate: f64,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            sample_pos: 0,
            bpm: 120.0,
            sample_rate: 48_000.0,
        }
    }
}

impl Transport {
    /// Absolute time in seconds.
    #[inline]
    pub fn seconds(&self) -> f64 {
        self.sample_pos as f64 / self.sample_rate
    }

    /// Musical position in beats (derived, read-only).
    ///
    /// NOTE:
    /// This exists ONLY for DSP that needs beat-relative modulation.
    /// Engine code must never advance beats itself.
    #[inline]
    pub fn beats(&self) -> f64 {
        self.seconds() * (self.bpm / 60.0)
    }
}

//
// ===================================
// MARK: Scheduler-side musical transport
// ===================================
//

/// Musical-time transport.
///
/// This struct:
/// - lives ONLY in the scheduler
/// - is NOT real-time safe
/// - owns tempo, beat position, looping, etc.
#[derive(Debug)]
pub struct MusicalTransport {
    /// Current tempo
    bpm: f64,

    /// Sample rate
    sample_rate: f64,

    /// Absolute sample position
    sample_pos: u64,

    /// Musical position in beats
    beat_pos: f64,
}

impl MusicalTransport {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            bpm: 120.0,
            sample_rate,
            sample_pos: 0,
            beat_pos: 0.0,
        }
    }

    // -------------------------------
    // MARK: Scheduler → Engine boundary
    // -------------------------------

    /// Resolve the current musical state into a sample-domain transport.
    ///
    /// Called when constructing a SlicePlan.
    pub fn resolve_transport(&self) -> Transport {
        Transport {
            sample_pos: self.sample_pos,
            bpm: self.bpm,
            sample_rate: self.sample_rate,
        }
    }

    // -------------------------------
    // MARK: Time advancement
    // -------------------------------

    /// Advance musical time by a number of samples.
    ///
    /// Called once per compiled audio block.
    pub fn advance_samples(&mut self, frames: usize) {
        let seconds = frames as f64 / self.sample_rate;
        let beats = seconds * (self.bpm / 60.0);

        self.sample_pos += frames as u64;
        self.beat_pos += beats;
    }

    // -------------------------------
    // MARK: Accessors
    // -------------------------------

    #[inline]
    pub fn sample_position(&self) -> u64 {
        self.sample_pos
    }

    #[inline]
    pub fn beat_position(&self) -> f64 {
        self.beat_pos
    }

    #[inline]
    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    #[inline]
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    // -------------------------------
    // MARK: Mutators (scheduler-controlled)
    // -------------------------------

    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm;
    }

    // -------------------------------
    // MARK: Event compilation helpers
    // -------------------------------

    /// Convert a beat offset (relative to now) into a sample offset.
    #[inline]
    pub fn beat_offset_to_sample_offset(&self, beats: f64) -> usize {
        let seconds = beats * 60.0 / self.bpm;
        (seconds * self.sample_rate) as usize
    }

    /// Compute the sample offset of a musical event within the current block.
    ///
    /// Returns None if the event occurs before the current position.
    pub fn event_sample_offset(&self, event: &MusicalEvent) -> Option<usize> {
        let event_beat = event.beat();

        if event_beat < self.beat_pos {
            return None;
        }

        let delta_beats = event_beat - self.beat_pos;
        Some(self.beat_offset_to_sample_offset(delta_beats))
    }

    pub fn event_sample_position(&self, event: &MusicalEvent) -> Option<u64> {
        self.event_sample_offset(event)
            .map(|offset| self.sample_pos + offset as u64)
    }
}
