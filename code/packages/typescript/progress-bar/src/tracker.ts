/**
 * Tracker — a reusable, text-based progress bar for tracking operations.
 *
 * === The postal worker analogy ===
 *
 * Imagine a post office with a single clerk (the renderer) and a counter
 * window (the `send()` method). Workers walk up one at a time and hand over
 * letters (events). The clerk updates the scoreboard on the wall (the
 * progress bar) after each letter.
 *
 * In Go and Python, this analogy maps to channels and queues — many writers,
 * one reader, no locks needed. But Node.js is single-threaded. There's only
 * ever one worker at the counter at a time (the event loop ensures this).
 * So our design is simpler: `send()` updates state and redraws immediately.
 * No channels, no locks, no background threads — just direct synchronous
 * mutation.
 *
 * This is actually an advantage: the code is simpler and easier to test.
 * The single-threaded model means we get thread safety for free — not because
 * we use clever synchronization, but because concurrency conflicts are
 * impossible.
 *
 * === Usage ===
 *
 * Flat (simple) mode:
 *
 *   const t = new Tracker(21, process.stderr);
 *   t.start();
 *   t.send({ type: EventType.Started, name: "pkg-a" });
 *   t.send({ type: EventType.Finished, name: "pkg-a", status: "built" });
 *   t.send({ type: EventType.Skipped, name: "pkg-b" });
 *   t.stop();
 *
 * Hierarchical mode (e.g., build levels):
 *
 *   const parent = new Tracker(3, process.stderr, "Level");
 *   parent.start();
 *   const child = parent.child(7, "Package");
 *   child.send({ type: EventType.Started, name: "pkg-a" });
 *   child.send({ type: EventType.Finished, name: "pkg-a", status: "built" });
 *   child.finish();   // advances parent by 1
 *   parent.stop();
 */

// ---------------------------------------------------------------------------
// Event types — what can happen to a tracked item
// ---------------------------------------------------------------------------

/**
 * EventType distinguishes the three things that can happen to an item.
 *
 * Think of it like a traffic light:
 *
 *   Started  = green  (item is actively being processed)
 *   Finished = red    (item is done — success or failure)
 *   Skipped  = yellow (item was bypassed without processing)
 *
 * We use a string enum rather than numeric because string values are
 * self-documenting in logs and test output. When you see `"started"` in a
 * debug dump, you know what it means without looking up a number.
 */
export enum EventType {
  Started = "started",
  Finished = "finished",
  Skipped = "skipped",
}

// ---------------------------------------------------------------------------
// Event — the message that workers send to the tracker
// ---------------------------------------------------------------------------

/**
 * Event is the message that workers send to the tracker.
 *
 * It's deliberately minimal — just three fields:
 *
 *   type   — what happened (Started, Finished, Skipped)
 *   name   — human-readable identifier (e.g., "python/logic-gates")
 *   status — outcome label, only meaningful for Finished events
 *            (e.g., "built", "failed", "cached")
 *
 * The `status` field is optional because Started and Skipped events don't
 * need one. TypeScript's optional property syntax (`status?`) expresses
 * this naturally.
 */
export interface Event {
  type: EventType;
  name: string;
  status?: string;
}

// ---------------------------------------------------------------------------
// Writable — abstraction over output streams
// ---------------------------------------------------------------------------

/**
 * Writable is a minimal interface for anything that can receive text output.
 *
 * Why not just use `process.stderr`? Two reasons:
 *
 * 1. **Testability**: In tests, we pass a mock writer that collects output
 *    into a string array. This lets us assert on exact output without
 *    capturing stderr.
 *
 * 2. **Portability**: The Tracker doesn't depend on Node.js globals. You
 *    could use it in a browser (with a DOM-based writer) or in Deno/Bun
 *    without changes.
 *
 * This is the Dependency Inversion Principle in action: high-level modules
 * (Tracker) depend on abstractions (Writable), not concrete implementations
 * (process.stderr).
 */
export interface Writable {
  write(s: string): void;
}

// ---------------------------------------------------------------------------
// Tracker — the progress bar engine
// ---------------------------------------------------------------------------

/**
 * Tracker receives events from operations and renders a text-based progress
 * bar.
 *
 * === State tracking ===
 *
 * The tracker maintains three pieces of state:
 *
 *   completed — count of items that are Finished or Skipped
 *   building  — map of item names currently in-flight (Started but not Finished)
 *   total     — the target count (set at creation time)
 *
 * Truth table for state transitions:
 *
 *   | Event    | completed | building     |
 *   |----------|-----------|--------------|
 *   | Started  | unchanged | add name     |
 *   | Finished | +1        | remove name  |
 *   | Skipped  | +1        | unchanged    |
 *
 * === Rendering ===
 *
 * Every call to `send()` updates state and then calls `draw()`. This is
 * the simplest approach: no timers, no debouncing, no background threads.
 *
 * The progress bar uses Unicode block characters:
 *
 *   █ (U+2588) — filled portion
 *   ░ (U+2591) — empty portion
 *
 * We use `\r` (carriage return) to overwrite the current line. This works
 * on all platforms — Windows cmd, PowerShell, Git Bash, and Unix terminals.
 * No ANSI escape codes needed.
 */
export class Tracker {
  /** Total number of items to track. */
  private total: number;

  /** Number of items completed (Finished + Skipped). */
  private completed: number = 0;

  /** Set of item names currently in-flight. We use a Map for ordered iteration. */
  private building: Map<string, boolean> = new Map();

  /** Output destination for the progress bar. */
  private writer: Writable;

  /** Timestamp of when tracking started, in milliseconds since epoch. */
  private startTime: number = 0;

  /** Optional label prefix (e.g., "Level", "Package"). */
  private label: string;

  /** Parent tracker for hierarchical progress. */
  private parent: Tracker | null;

  /** Whether the tracker has been started. */
  private started: boolean = false;

  /**
   * Create a new Tracker.
   *
   * @param total  - The number of items to track. The bar reaches 100% when
   *                 `completed === total`.
   * @param writer - Where to write the progress bar. Pass `process.stderr`
   *                 for terminal output, or a mock for testing.
   * @param label  - Optional prefix label. Used in hierarchical mode to show
   *                 context (e.g., "Level 2/3"). Pass "" for flat mode.
   */
  constructor(total: number, writer: Writable, label: string = "") {
    this.total = total;
    this.writer = writer;
    this.label = label;
    this.parent = null;
  }

  /**
   * Start the tracker. Records the start time for elapsed duration display.
   *
   * Must be called before `send()`. In the Go version, this launches a
   * background goroutine. Here, it just records the start time — there's
   * nothing to launch because Node.js processes events synchronously.
   */
  start(): void {
    this.startTime = Date.now();
    this.started = true;
  }

  /**
   * Send an event to the tracker.
   *
   * This is the main API. Workers call `send()` whenever something happens
   * to a tracked item. The tracker updates its internal state and redraws
   * the progress bar.
   *
   * Unlike the Go version (which writes to a channel), this is synchronous.
   * The single-threaded event loop guarantees that no two `send()` calls
   * can interleave, so there are no race conditions.
   *
   * @param event - The event to process.
   */
  send(event: Event): void {
    switch (event.type) {
      case EventType.Started:
        this.building.set(event.name, true);
        break;
      case EventType.Finished:
        this.building.delete(event.name);
        this.completed++;
        break;
      case EventType.Skipped:
        this.completed++;
        break;
    }
    this.draw();
  }

  /**
   * Create a child tracker for hierarchical progress.
   *
   * The child shares the parent's writer and start time. When the child
   * calls `finish()`, it advances the parent's completed count by 1.
   *
   * Example: a build system has 3 dependency levels, each with N packages.
   * The parent tracks levels (total=3, label="Level"), and each child
   * tracks packages within that level (total=N, label="Package").
   *
   *   parent = new Tracker(3, writer, "Level");
   *   child = parent.child(7, "Package");
   *   // Display: Level 1/3  [████░░░░]  3/7  Building: pkg-a  (2.1s)
   *
   * @param total - Number of items in this child's scope.
   * @param label - Label for the child's progress line.
   * @returns A new child Tracker linked to this parent.
   */
  child(total: number, label: string): Tracker {
    const c = new Tracker(total, this.writer, label);
    c.startTime = this.startTime;
    c.parent = this;
    c.started = true;
    return c;
  }

  /**
   * Mark this child tracker as complete and advance the parent.
   *
   * This is the counterpart to `child()`. Call it when all items in the
   * child's scope are done. It sends a Finished event to the parent,
   * which advances the parent's completed count by 1.
   *
   * If this tracker has no parent, `finish()` is a no-op — use `stop()`
   * instead for top-level trackers.
   */
  finish(): void {
    if (this.parent !== null) {
      this.parent.send({ type: EventType.Finished, name: this.label });
    }
  }

  /**
   * Stop the tracker and print a final newline.
   *
   * The final newline preserves the last progress line in the terminal
   * scrollback. Without it, the next shell prompt would overwrite the
   * progress bar (because we use `\r` without `\n`).
   */
  stop(): void {
    this.writer.write("\n");
  }

  // -------------------------------------------------------------------------
  // Getters for testing — expose internal state read-only
  // -------------------------------------------------------------------------

  /** Get the current completed count. Useful for testing. */
  getCompleted(): number {
    return this.completed;
  }

  /** Get the current in-flight names. Useful for testing. */
  getBuilding(): string[] {
    return Array.from(this.building.keys()).sort();
  }

  /** Get the total. Useful for testing. */
  getTotal(): number {
    return this.total;
  }

  // -------------------------------------------------------------------------
  // Internal: draw the progress bar
  // -------------------------------------------------------------------------

  /**
   * Compose and write one progress line to the writer.
   *
   * The line format depends on whether we have a parent (hierarchical)
   * or not (flat):
   *
   * Flat:
   *   \r[████████░░░░░░░░░░░░]  7/21  Building: pkg-a, pkg-b  (12.3s)
   *
   * Hierarchical:
   *   \rLevel 2/3  [████░░░░░░░░░░░░░░░░]  5/12  Building: pkg-a  (8.2s)
   *
   * The bar is 20 characters wide. The number of filled characters is
   * proportional to completed/total:
   *
   *   filled = Math.floor((completed * 20) / total)
   *
   * Integer division (via Math.floor) naturally rounds down, so the bar
   * only shows 100% when all items are truly complete.
   */
  private draw(): void {
    const elapsed = (Date.now() - this.startTime) / 1000;

    // --- Build the progress bar ---
    const barWidth = 20;
    let filled = 0;
    if (this.total > 0) {
      filled = Math.floor((this.completed * barWidth) / this.total);
    }
    if (filled > barWidth) {
      filled = barWidth;
    }
    const bar =
      "\u2588".repeat(filled) + "\u2591".repeat(barWidth - filled);

    // --- Build the in-flight names list ---
    const activity = formatActivity(
      this.building,
      this.completed,
      this.total,
    );

    // --- Compose the line ---
    let line: string;
    if (this.parent !== null) {
      // Hierarchical: show parent label and count.
      const parentCompleted = this.parent.completed + 1; // +1 because this child is "current"
      line = `\r${this.parent.label} ${parentCompleted}/${this.parent.total}  [${bar}]  ${this.completed}/${this.total}  ${activity}  (${elapsed.toFixed(1)}s)`;
    } else if (this.label !== "") {
      // Labeled flat tracker (used as parent — shows own state).
      line = `\r${this.label} ${this.completed}/${this.total}  [${bar}]  ${activity}  (${elapsed.toFixed(1)}s)`;
    } else {
      // Flat mode: just the bar.
      line = `\r[${bar}]  ${this.completed}/${this.total}  ${activity}  (${elapsed.toFixed(1)}s)`;
    }

    // Pad to 80 characters to overwrite any previous longer line.
    this.writer.write(line.padEnd(80));
  }
}

// ---------------------------------------------------------------------------
// NullTracker — a no-op implementation for when progress is disabled
// ---------------------------------------------------------------------------

/**
 * NullTracker is a drop-in replacement for Tracker that does nothing.
 *
 * This implements the Null Object pattern: instead of checking `if (tracker)`
 * everywhere, callers can use NullTracker and call methods unconditionally.
 * All methods are no-ops.
 *
 * In the Go version, this is achieved with nil receiver checks. In TypeScript,
 * we use a separate class because `null` doesn't have methods.
 *
 * Example:
 *
 *   const tracker = verbose ? new Tracker(10, process.stderr) : new NullTracker();
 *   tracker.start();           // does nothing if NullTracker
 *   tracker.send(someEvent);   // does nothing if NullTracker
 *   tracker.stop();            // does nothing if NullTracker
 */
export class NullTracker {
  start(): void {
    /* no-op */
  }
  send(_event: Event): void {
    /* no-op */
  }
  child(_total: number, _label: string): NullTracker {
    return new NullTracker();
  }
  finish(): void {
    /* no-op */
  }
  stop(): void {
    /* no-op */
  }
  getCompleted(): number {
    return 0;
  }
  getBuilding(): string[] {
    return [];
  }
  getTotal(): number {
    return 0;
  }
}

// ---------------------------------------------------------------------------
// Helper: format the activity string
// ---------------------------------------------------------------------------

/**
 * Build the "Building: pkg-a, pkg-b" or "waiting..." or "done" string
 * from the current in-flight set.
 *
 * The rules:
 *
 *   | In-flight count | Completed vs Total | Output                       |
 *   |-----------------|--------------------|------------------------------|
 *   | 0               | completed < total  | "waiting..."                 |
 *   | 0               | completed >= total | "done"                       |
 *   | 1-3             | any                | "Building: a, b, c"          |
 *   | 4+              | any                | "Building: a, b, c +N more"  |
 *
 * Names are sorted alphabetically for deterministic output. This matters
 * for testing — without sorting, the order would depend on Map iteration
 * order (which is insertion order in JS, but still non-deterministic from
 * the caller's perspective).
 *
 * @param building  - Map of currently in-flight item names.
 * @param completed - Number of completed items.
 * @param total     - Total number of items.
 * @returns The formatted activity string.
 */
export function formatActivity(
  building: Map<string, boolean>,
  completed: number,
  total: number,
): string {
  if (building.size === 0) {
    if (completed >= total) {
      return "done";
    }
    return "waiting...";
  }

  const names = Array.from(building.keys()).sort();
  const maxNames = 3;

  if (names.length <= maxNames) {
    return "Building: " + names.join(", ");
  }

  const shown = names.slice(0, maxNames).join(", ");
  return `Building: ${shown} +${names.length - maxNames} more`;
}
