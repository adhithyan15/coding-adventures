/**
 * Synchronization primitives -- Fence, Semaphore, Event.
 *
 * === The Synchronization Problem ===
 *
 * CPUs and GPUs run asynchronously. When you submit a command buffer, the
 * CPU doesn't wait -- it immediately returns and can do other work. But
 * at some point you need to know: "Has the GPU finished yet?"
 *
 * === Three Levels of Synchronization ===
 *
 *     FENCE (CPU <-> GPU):
 *       CPU submits work with a fence attached, then calls fence.wait()
 *       to block until the GPU signals it.
 *
 *     SEMAPHORE (GPU Queue <-> GPU Queue):
 *       Queue A signals a semaphore when its CB completes.
 *       Queue B waits on that semaphore before starting.
 *
 *     EVENT (GPU <-> GPU, fine-grained):
 *       Set and waited on WITHIN command buffers.
 */

// =========================================================================
// Fence -- CPU waits for GPU
// =========================================================================

/**
 * CPU-to-GPU synchronization primitive.
 *
 * === Fence Lifecycle ===
 *
 *     create(signaled=false)
 *         |
 *     [unsignaled] --submit(fence=F)--> [GPU working]
 *         ^                                    |
 *         |                              GPU finishes
 *         |                                    |
 *         +---- reset() <-- [signaled] <-------+
 *                                |
 *                            wait() returns
 *
 * Fences are reusable -- call reset() to clear the signal, then attach
 * to another submission.
 */
export class Fence {
  private static _nextId = 0;

  private readonly _id: number;
  private _signaled: boolean;
  private _waitCycles: number;

  constructor(signaled = false) {
    this._id = Fence._nextId++;
    this._signaled = signaled;
    this._waitCycles = 0;
  }

  /** Unique identifier for this fence. */
  get fenceId(): number {
    return this._id;
  }

  /** Whether the GPU has signaled this fence. */
  get signaled(): boolean {
    return this._signaled;
  }

  /** Total cycles the CPU spent waiting on this fence. */
  get waitCycles(): number {
    return this._waitCycles;
  }

  /** Signal the fence (called by the runtime when GPU finishes). */
  signal(): void {
    this._signaled = true;
  }

  /**
   * Wait for the fence to be signaled.
   *
   * In our synchronous simulation, the fence is either already signaled
   * (because submit() runs to completion) or it's not.
   */
  wait(_timeoutCycles?: number): boolean {
    return this._signaled;
  }

  /** Reset the fence to unsignaled state for reuse. */
  reset(): void {
    this._signaled = false;
    this._waitCycles = 0;
  }
}

// =========================================================================
// Semaphore -- GPU-to-GPU synchronization
// =========================================================================

/**
 * GPU queue-to-queue synchronization primitive.
 *
 * Fences are for CPU <-> GPU synchronization (CPU blocks until GPU done).
 * Semaphores are for GPU <-> GPU synchronization between different queues.
 * The CPU never waits on a semaphore -- they're entirely GPU-side.
 */
export class Semaphore {
  private static _nextId = 0;

  private readonly _id: number;
  private _signaled: boolean;

  constructor() {
    this._id = Semaphore._nextId++;
    this._signaled = false;
  }

  /** Unique identifier for this semaphore. */
  get semaphoreId(): number {
    return this._id;
  }

  /** Whether this semaphore has been signaled. */
  get signaled(): boolean {
    return this._signaled;
  }

  /** Signal the semaphore (called by runtime after queue completes). */
  signal(): void {
    this._signaled = true;
  }

  /** Reset to unsignaled (called by runtime when consumed by a wait). */
  reset(): void {
    this._signaled = false;
  }
}

// =========================================================================
// Event -- fine-grained GPU-side synchronization
// =========================================================================

/**
 * Fine-grained GPU-side synchronization primitive.
 *
 * Pipeline barriers are implicit -- they're executed inline in a command
 * buffer. Events are explicit -- you set them at one point and wait for
 * them at another, potentially in a different command buffer.
 */
export class Event {
  private static _nextId = 0;

  private readonly _id: number;
  private _signaled: boolean;

  constructor() {
    this._id = Event._nextId++;
    this._signaled = false;
  }

  /** Unique identifier for this event. */
  get eventId(): number {
    return this._id;
  }

  /** Whether this event has been signaled. */
  get signaled(): boolean {
    return this._signaled;
  }

  /** Signal the event. */
  set(): void {
    this._signaled = true;
  }

  /** Clear the event. */
  reset(): void {
    this._signaled = false;
  }

  /** Check if signaled without blocking. */
  status(): boolean {
    return this._signaled;
  }
}
