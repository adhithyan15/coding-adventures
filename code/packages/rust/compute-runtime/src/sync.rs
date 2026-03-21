//! Synchronization primitives -- Fence, Semaphore, Event.
//!
//! # The Synchronization Problem
//!
//! CPUs and GPUs run asynchronously. When you submit a command buffer, the
//! CPU doesn't wait -- it immediately returns and can do other work. But at
//! some point you need to know: "Has the GPU finished yet?"
//!
//! That's what synchronization primitives solve.
//!
//! # Three Levels of Synchronization
//!
//! ```text
//! FENCE (CPU <-> GPU):
//!   CPU submits work with a fence attached, then calls fence.wait()
//!   to block until the GPU signals it.
//!
//!   CPU:  [submit(fence=F)]----------[F.wait()]--[read results]
//!   GPU:  ----------[execute]--[signal F]
//!
//! SEMAPHORE (GPU Queue <-> GPU Queue):
//!   Queue A signals a semaphore when its command buffer completes.
//!   Queue B waits on that semaphore before starting.
//!
//!   Transfer Queue: [upload data]--[signal S]
//!   Compute Queue:  ----------------[wait S]--[run kernel]
//!
//! EVENT (GPU <-> GPU, fine-grained):
//!   Set and waited on WITHIN command buffers.
//!
//!   CB: [dispatch A]--[set_event E]--[wait_event E]--[dispatch B]
//! ```

use std::sync::atomic::{AtomicUsize, Ordering};

// =========================================================================
// ID generators
// =========================================================================

static NEXT_FENCE_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_SEMAPHORE_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_EVENT_ID: AtomicUsize = AtomicUsize::new(0);

/// Reset all ID counters (for test isolation).
pub fn reset_sync_ids() {
    NEXT_FENCE_ID.store(0, Ordering::SeqCst);
    NEXT_SEMAPHORE_ID.store(0, Ordering::SeqCst);
    NEXT_EVENT_ID.store(0, Ordering::SeqCst);
}

// =========================================================================
// Fence -- CPU waits for GPU
// =========================================================================

/// CPU-to-GPU synchronization primitive.
///
/// # Fence Lifecycle
///
/// ```text
/// create_fence(signaled=false)
///     |
///     v
/// [unsignaled] --submit(fence=F)--> [GPU working]
///     ^                                    |
///     |                              GPU finishes
///     |                                    |
///     +---- reset() <-- [signaled] <-------+
///                             |
///                         wait() returns
/// ```
///
/// You attach a fence to a queue submission. When the GPU finishes all
/// the command buffers in that submission, it signals the fence. The CPU
/// can then call `wait()` to block until the signal arrives.
///
/// Fences are reusable -- call `reset()` to clear the signal, then attach
/// to another submission.
pub struct Fence {
    id: usize,
    signaled: bool,
    wait_cycles: u64,
}

impl Fence {
    pub fn new(signaled: bool) -> Self {
        Self {
            id: NEXT_FENCE_ID.fetch_add(1, Ordering::SeqCst),
            signaled,
            wait_cycles: 0,
        }
    }

    pub fn fence_id(&self) -> usize {
        self.id
    }

    pub fn signaled(&self) -> bool {
        self.signaled
    }

    pub fn wait_cycles(&self) -> u64 {
        self.wait_cycles
    }

    /// Signal the fence (called by the runtime when GPU finishes).
    pub fn signal(&mut self) {
        self.signaled = true;
    }

    /// Wait for the fence to be signaled.
    ///
    /// In our synchronous simulation, the fence is either already signaled
    /// (because submit() runs to completion) or it's not.
    pub fn wait(&self, _timeout_cycles: Option<u64>) -> bool {
        self.signaled
    }

    /// Reset the fence to unsignaled state for reuse.
    pub fn reset(&mut self) {
        self.signaled = false;
        self.wait_cycles = 0;
    }
}

// =========================================================================
// Semaphore -- GPU-to-GPU synchronization
// =========================================================================

/// GPU queue-to-queue synchronization primitive.
///
/// # How Semaphores Differ from Fences
///
/// Fences are for CPU <-> GPU synchronization (CPU blocks until GPU done).
/// Semaphores are for GPU <-> GPU synchronization between different queues.
/// The CPU never waits on a semaphore -- they're entirely GPU-side.
pub struct Semaphore {
    id: usize,
    signaled: bool,
}

impl Semaphore {
    pub fn new() -> Self {
        Self {
            id: NEXT_SEMAPHORE_ID.fetch_add(1, Ordering::SeqCst),
            signaled: false,
        }
    }

    pub fn semaphore_id(&self) -> usize {
        self.id
    }

    pub fn signaled(&self) -> bool {
        self.signaled
    }

    /// Signal the semaphore (called by runtime after queue completes).
    pub fn signal(&mut self) {
        self.signaled = true;
    }

    /// Reset to unsignaled (called by runtime when consumed by a wait).
    pub fn reset(&mut self) {
        self.signaled = false;
    }
}

impl Default for Semaphore {
    fn default() -> Self {
        Self::new()
    }
}

// =========================================================================
// Event -- fine-grained GPU-side synchronization
// =========================================================================

/// Fine-grained GPU-side synchronization primitive.
///
/// # Events vs Barriers
///
/// Pipeline barriers are implicit -- they're executed inline in a command
/// buffer. Events are explicit -- you set them at one point and wait for
/// them at another, potentially in a different command buffer or even
/// from the CPU.
pub struct Event {
    id: usize,
    signaled: bool,
}

impl Event {
    pub fn new() -> Self {
        Self {
            id: NEXT_EVENT_ID.fetch_add(1, Ordering::SeqCst),
            signaled: false,
        }
    }

    pub fn event_id(&self) -> usize {
        self.id
    }

    pub fn signaled(&self) -> bool {
        self.signaled
    }

    /// Signal the event.
    pub fn set(&mut self) {
        self.signaled = true;
    }

    /// Clear the event.
    pub fn reset(&mut self) {
        self.signaled = false;
    }

    /// Check if signaled without blocking.
    pub fn status(&self) -> bool {
        self.signaled
    }
}

impl Default for Event {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fence_lifecycle() {
        let mut fence = Fence::new(false);
        assert!(!fence.signaled());
        assert!(!fence.wait(None));

        fence.signal();
        assert!(fence.signaled());
        assert!(fence.wait(None));

        fence.reset();
        assert!(!fence.signaled());
    }

    #[test]
    fn test_fence_created_signaled() {
        let fence = Fence::new(true);
        assert!(fence.signaled());
        assert!(fence.wait(None));
    }

    #[test]
    fn test_semaphore_lifecycle() {
        let mut sem = Semaphore::new();
        assert!(!sem.signaled());

        sem.signal();
        assert!(sem.signaled());

        sem.reset();
        assert!(!sem.signaled());
    }

    #[test]
    fn test_event_lifecycle() {
        let mut event = Event::new();
        assert!(!event.signaled());
        assert!(!event.status());

        event.set();
        assert!(event.signaled());
        assert!(event.status());

        event.reset();
        assert!(!event.signaled());
    }

    #[test]
    fn test_unique_ids() {
        reset_sync_ids();
        let f1 = Fence::new(false);
        let f2 = Fence::new(false);
        assert_ne!(f1.fence_id(), f2.fence_id());

        let s1 = Semaphore::new();
        let s2 = Semaphore::new();
        assert_ne!(s1.semaphore_id(), s2.semaphore_id());

        let e1 = Event::new();
        let e2 = Event::new();
        assert_ne!(e1.event_id(), e2.event_id());
    }
}
