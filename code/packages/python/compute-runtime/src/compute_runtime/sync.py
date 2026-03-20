"""Synchronization primitives — Fence, Semaphore, Event.

=== The Synchronization Problem ===

CPUs and GPUs run asynchronously. When you submit a command buffer, the
CPU doesn't wait — it immediately returns and can do other work (or submit
more GPU work). But at some point you need to know:

    "Has the GPU finished yet?"

That's what synchronization primitives solve.

=== Three Levels of Synchronization ===

    ┌─────────────────────────────────────────────────────────────────┐
    │  FENCE (CPU ↔ GPU)                                              │
    │                                                                 │
    │  CPU submits work with a fence attached, then calls fence.wait()│
    │  to block until the GPU signals it.                             │
    │                                                                 │
    │  CPU:  [submit(fence=F)]──────────[F.wait()]──[read results]    │
    │  GPU:  ──────────[execute]──[signal F]                          │
    │                                                                 │
    │  Use case: "wait for my kernel to finish so I can read results" │
    └─────────────────────────────────────────────────────────────────┘

    ┌─────────────────────────────────────────────────────────────────┐
    │  SEMAPHORE (GPU Queue ↔ GPU Queue)                              │
    │                                                                 │
    │  Queue A signals a semaphore when its command buffer completes. │
    │  Queue B waits on that semaphore before starting.               │
    │                                                                 │
    │  Transfer Queue: [upload data]──[signal S]                      │
    │  Compute Queue:  ──────────────[wait S]──[run kernel]           │
    │                                                                 │
    │  Use case: "compute waits for transfer to finish"               │
    └─────────────────────────────────────────────────────────────────┘

    ┌─────────────────────────────────────────────────────────────────┐
    │  EVENT (GPU ↔ GPU, fine-grained)                                │
    │                                                                 │
    │  Set and waited on WITHIN command buffers.                      │
    │                                                                 │
    │  CB: [dispatch A]──[set_event E]──[wait_event E]──[dispatch B]  │
    │                                                                 │
    │  Use case: "barrier between two dispatches in the same CB"      │
    └─────────────────────────────────────────────────────────────────┘
"""

from __future__ import annotations


# =========================================================================
# Fence — CPU waits for GPU
# =========================================================================


class Fence:
    """CPU-to-GPU synchronization primitive.

    === Fence Lifecycle ===

        create_fence(signaled=False)
            │
            ▼
        [unsignaled] ──submit(fence=F)──► [GPU working]
            ▲                                    │
            │                              GPU finishes
            │                                    │
            └──── reset() ◄── [signaled] ◄───────┘
                                  │
                              wait() returns

    You attach a fence to a queue submission. When the GPU finishes all
    the command buffers in that submission, it signals the fence. The CPU
    can then call wait() to block until the signal arrives.

    Fences are reusable — call reset() to clear the signal, then attach
    to another submission.
    """

    _next_id: int = 0

    def __init__(self, signaled: bool = False) -> None:
        self._id = Fence._next_id
        Fence._next_id += 1
        self._signaled = signaled
        self._wait_cycles: int = 0

    @property
    def fence_id(self) -> int:
        """Unique identifier for this fence."""
        return self._id

    @property
    def signaled(self) -> bool:
        """Whether the GPU has signaled this fence."""
        return self._signaled

    @property
    def wait_cycles(self) -> int:
        """Total cycles the CPU spent waiting on this fence."""
        return self._wait_cycles

    def signal(self) -> None:
        """Signal the fence (called by the runtime when GPU finishes)."""
        self._signaled = True

    def wait(self, timeout_cycles: int | None = None) -> bool:
        """Wait for the fence to be signaled.

        In a real system, this blocks the CPU thread. In our simulator,
        the fence is either already signaled (because we run synchronously)
        or it's not (which would be a programming error — submitting without
        actually executing).

        Args:
            timeout_cycles: Maximum cycles to wait (None = wait forever).

        Returns:
            True if the fence was signaled, False if timeout expired.
        """
        # In our synchronous simulation, the fence is always signaled
        # by the time we check (because submit() runs the device to
        # completion before returning).
        return self._signaled

    def reset(self) -> None:
        """Reset the fence to unsignaled state for reuse."""
        self._signaled = False
        self._wait_cycles = 0


# =========================================================================
# Semaphore — GPU-to-GPU synchronization
# =========================================================================


class Semaphore:
    """GPU queue-to-queue synchronization primitive.

    === How Semaphores Differ from Fences ===

    Fences are for CPU ↔ GPU synchronization (CPU blocks until GPU done).
    Semaphores are for GPU ↔ GPU synchronization between different queues.

    The CPU never waits on a semaphore — they're entirely GPU-side.

    === Usage Pattern ===

        # Transfer queue signals when upload is done
        transfer_queue.submit(
            [upload_cb],
            signal_semaphores=[sem],
        )

        # Compute queue waits for upload before starting kernel
        compute_queue.submit(
            [compute_cb],
            wait_semaphores=[sem],
        )

    The GPU hardware ensures that the compute queue doesn't start until
    the transfer queue has finished.
    """

    _next_id: int = 0

    def __init__(self) -> None:
        self._id = Semaphore._next_id
        Semaphore._next_id += 1
        self._signaled = False

    @property
    def semaphore_id(self) -> int:
        """Unique identifier for this semaphore."""
        return self._id

    @property
    def signaled(self) -> bool:
        """Whether this semaphore has been signaled."""
        return self._signaled

    def signal(self) -> None:
        """Signal the semaphore (called by runtime after queue completes)."""
        self._signaled = True

    def reset(self) -> None:
        """Reset to unsignaled (called by runtime when consumed by a wait)."""
        self._signaled = False


# =========================================================================
# Event — fine-grained GPU-side synchronization
# =========================================================================


class Event:
    """Fine-grained GPU-side synchronization primitive.

    === Events vs Barriers ===

    Pipeline barriers are implicit — they're executed inline in a command
    buffer. Events are explicit — you set them at one point and wait for
    them at another, potentially in a different command buffer or even
    from the CPU.

    === Usage Patterns ===

    GPU-side (in command buffer):
        cb.cmd_set_event(event, stage=COMPUTE)
        cb.cmd_wait_event(event, src_stage=COMPUTE, dst_stage=COMPUTE)

    CPU-side:
        event.set()        # CPU signals
        event.status()     # CPU checks without blocking
        event.reset()      # CPU clears
    """

    _next_id: int = 0

    def __init__(self) -> None:
        self._id = Event._next_id
        Event._next_id += 1
        self._signaled = False

    @property
    def event_id(self) -> int:
        """Unique identifier for this event."""
        return self._id

    @property
    def signaled(self) -> bool:
        """Whether this event has been signaled."""
        return self._signaled

    def set(self) -> None:
        """Signal the event."""
        self._signaled = True

    def reset(self) -> None:
        """Clear the event."""
        self._signaled = False

    def status(self) -> bool:
        """Check if signaled without blocking."""
        return self._signaled
