//! Hand-rolled minimal `block_on` — drive a `Future` to completion
//! with no external runtime dependency.
//!
//! ## How it works
//!
//! `block_on` polls a future in a tight loop with a no-op `Waker`.
//! When the future returns `Poll::Ready`, we return its output.
//! When it returns `Poll::Pending`, we yield the OS thread (so any
//! other threads that need CPU can run) and try again.
//!
//! This is sufficient for V1 because the only transport we ship is
//! [`LocalTransport`](crate::LocalTransport), which resolves
//! immediately — its futures never actually `Pend`.  A no-op waker is
//! correct in that case because the future will be ready on every
//! poll.
//!
//! For network transports later (TCP, UnixSocket, ZMQ), this minimal
//! `block_on` is **not** sufficient — those transports need a real
//! reactor that wakes when sockets become readable.  Those reactors
//! live in their own transport crates and supply their own
//! `block_on` (or are driven by the standard async ecosystem if the
//! user opts in).
//!
//! The point is: `executor-protocol` itself remains zero-dep and
//! supports the in-process case without pulling in a runtime.

use core::future::Future;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

/// VTable for the no-op waker.  All four methods are no-ops because:
///
/// - `clone` — return another no-op waker (same vtable, null data).
/// - `wake` — do nothing; the polling loop will try again next iteration.
/// - `wake_by_ref` — same.
/// - `drop` — nothing to drop (no allocation).
const NOOP_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |_| RawWaker::new(ptr::null(), &NOOP_VTABLE), // clone
    |_| {},                                        // wake
    |_| {},                                        // wake_by_ref
    |_| {},                                        // drop
);

/// Construct a `Waker` that does nothing on wake.
fn noop_waker() -> Waker {
    // SAFETY: `NOOP_VTABLE`'s functions are sound — they perform no
    // memory accesses on the `data` pointer (which is null), and the
    // returned `Waker` is `Send + Sync` because both clone and the
    // null pointer are trivially so.
    unsafe { Waker::from_raw(RawWaker::new(ptr::null(), &NOOP_VTABLE)) }
}

/// Drive `f` to completion synchronously.
///
/// This is intended for in-process futures that resolve immediately
/// (or after a small number of polls).  For genuinely-pending futures
/// it busy-waits, which is wasteful — use a real reactor in those
/// contexts.
pub fn block_on<F: Future>(mut f: F) -> F::Output {
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    // SAFETY: we own `f` and never move it again before it's dropped.
    let mut pinned = unsafe { Pin::new_unchecked(&mut f) };
    loop {
        match pinned.as_mut().poll(&mut cx) {
            Poll::Ready(out) => return out,
            Poll::Pending => {
                // Yield the OS thread so other threads can run.  If
                // we are the only thread, this is a near-no-op.
                std::thread::yield_now();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_on_immediate_future() {
        // An immediately-ready future: produces 42.
        struct Imm(i32);
        impl Future for Imm {
            type Output = i32;
            fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<i32> {
                Poll::Ready(self.0)
            }
        }
        let result = block_on(Imm(42));
        assert_eq!(result, 42);
    }

    #[test]
    fn block_on_one_pend_then_ready() {
        // A future that returns Pending exactly once, then Ready.
        struct OneStep {
            polled: bool,
        }
        impl Future for OneStep {
            type Output = &'static str;
            fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<&'static str> {
                if !self.polled {
                    self.polled = true;
                    Poll::Pending
                } else {
                    Poll::Ready("done")
                }
            }
        }
        let result = block_on(OneStep { polled: false });
        assert_eq!(result, "done");
    }
}
