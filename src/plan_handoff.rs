use std::sync::atomic::{AtomicUsize, Ordering};

use crate::execution_plan::ExecutionPlan;

/// Lock-free double buffer for execution plans.
///
/// Single producer (scheduler)
/// Single consumer (audio thread)
pub struct PlanHandoff {
    plans: [ExecutionPlan; 2],

    /// Index currently visible to the audio thread
    read_index: AtomicUsize,
}

impl PlanHandoff {
    pub fn new(plan_a: ExecutionPlan, plan_b: ExecutionPlan) -> Self {
        Self {
            plans: [plan_a, plan_b],
            read_index: AtomicUsize::new(0),
        }
    }

    /// Get a mutable reference to the plan NOT currently used by audio.
    ///
    /// Scheduler-only.
    #[inline]
    pub fn write_plan(&mut self) -> &mut ExecutionPlan {
        let read = self.read_index.load(Ordering::Acquire);
        let write = 1 - read;
        &mut self.plans[write]
    }

    /// Publish the written plan to the audio thread.
    ///
    /// Scheduler-only.
    #[inline]
    pub fn publish(&self) {
        let read = self.read_index.load(Ordering::Relaxed);
        let write = 1 - read;
        self.read_index.store(write, Ordering::Release);
    }

    /// Get the currently active plan.
    ///
    /// Audio-thread-safe, lock-free.
    #[inline]
    pub fn read_plan(&self) -> &ExecutionPlan {
        let index = self.read_index.load(Ordering::Acquire);
        &self.plans[index]
    }
}
