/**
 * Tests for the progress bar tracker.
 *
 * Strategy: we inject a mock writer that collects all output into an array.
 * This lets us assert on exact output without capturing stderr or dealing
 * with timing. The mock writer is the testing equivalent of a flight
 * recorder — it captures everything for post-mortem analysis.
 */

import { describe, it, expect, beforeEach } from "vitest";
import {
  Tracker,
  NullTracker,
  EventType,
  formatActivity,
} from "../src/tracker.js";
import type { Event, Writable } from "../src/tracker.js";

// ---------------------------------------------------------------------------
// Mock writer — captures output for assertions
// ---------------------------------------------------------------------------

/**
 * A mock Writable that stores every write() call in an array.
 * This is far easier to test against than trying to intercept stderr.
 */
class MockWriter implements Writable {
  lines: string[] = [];

  write(s: string): void {
    this.lines.push(s);
  }

  /** Get the last written line, trimmed of padding. */
  last(): string {
    return this.lines[this.lines.length - 1]?.trimEnd() ?? "";
  }

  /** Reset captured output. */
  clear(): void {
    this.lines = [];
  }
}

// ---------------------------------------------------------------------------
// Event counting tests
// ---------------------------------------------------------------------------

describe("Tracker event counting", () => {
  let writer: MockWriter;
  let tracker: Tracker;

  beforeEach(() => {
    writer = new MockWriter();
    tracker = new Tracker(5, writer);
    tracker.start();
  });

  it("should start with zero completed", () => {
    expect(tracker.getCompleted()).toBe(0);
    expect(tracker.getBuilding()).toEqual([]);
    expect(tracker.getTotal()).toBe(5);
  });

  it("should track Started events as in-flight", () => {
    tracker.send({ type: EventType.Started, name: "pkg-a" });
    expect(tracker.getBuilding()).toEqual(["pkg-a"]);
    expect(tracker.getCompleted()).toBe(0);
  });

  it("should increment completed on Finished events", () => {
    tracker.send({ type: EventType.Started, name: "pkg-a" });
    tracker.send({ type: EventType.Finished, name: "pkg-a", status: "built" });
    expect(tracker.getCompleted()).toBe(1);
    expect(tracker.getBuilding()).toEqual([]);
  });

  it("should increment completed on Skipped events", () => {
    tracker.send({ type: EventType.Skipped, name: "pkg-b" });
    expect(tracker.getCompleted()).toBe(1);
    expect(tracker.getBuilding()).toEqual([]);
  });

  it("should track multiple in-flight items", () => {
    tracker.send({ type: EventType.Started, name: "pkg-c" });
    tracker.send({ type: EventType.Started, name: "pkg-a" });
    tracker.send({ type: EventType.Started, name: "pkg-b" });
    // getBuilding() returns sorted names
    expect(tracker.getBuilding()).toEqual(["pkg-a", "pkg-b", "pkg-c"]);
    expect(tracker.getCompleted()).toBe(0);
  });

  it("should handle a full lifecycle", () => {
    // Start 3 items
    tracker.send({ type: EventType.Started, name: "pkg-a" });
    tracker.send({ type: EventType.Started, name: "pkg-b" });
    tracker.send({ type: EventType.Started, name: "pkg-c" });

    // Finish 2, skip 1, leave 2 pending
    tracker.send({ type: EventType.Finished, name: "pkg-a", status: "built" });
    tracker.send({ type: EventType.Skipped, name: "pkg-d" });
    tracker.send({ type: EventType.Finished, name: "pkg-b", status: "built" });

    expect(tracker.getCompleted()).toBe(3);
    expect(tracker.getBuilding()).toEqual(["pkg-c"]);
  });
});

// ---------------------------------------------------------------------------
// Bar rendering format tests
// ---------------------------------------------------------------------------

describe("Tracker bar rendering", () => {
  let writer: MockWriter;

  it("should render flat mode with correct format", () => {
    writer = new MockWriter();
    const tracker = new Tracker(4, writer);
    tracker.start();

    tracker.send({ type: EventType.Started, name: "pkg-a" });
    const line = writer.last();

    // Should contain the bar, count, and activity
    expect(line).toContain("[");
    expect(line).toContain("]");
    expect(line).toContain("0/4");
    expect(line).toContain("Building: pkg-a");
    // Should start with \r for line overwriting
    expect(writer.lines[0]).toMatch(/^\r/);
  });

  it("should show filled bar proportional to progress", () => {
    writer = new MockWriter();
    const tracker = new Tracker(2, writer);
    tracker.start();

    // Complete 1 of 2 — should show half filled (10 of 20 chars)
    tracker.send({ type: EventType.Finished, name: "pkg-a", status: "ok" });
    const line = writer.last();
    // 10 filled blocks + 10 empty blocks
    expect(line).toContain("\u2588".repeat(10) + "\u2591".repeat(10));
  });

  it("should show fully filled bar when all complete", () => {
    writer = new MockWriter();
    const tracker = new Tracker(1, writer);
    tracker.start();

    tracker.send({ type: EventType.Finished, name: "pkg-a", status: "ok" });
    const line = writer.last();
    expect(line).toContain("\u2588".repeat(20));
    expect(line).not.toContain("\u2591");
  });

  it("should show empty bar at start", () => {
    writer = new MockWriter();
    const tracker = new Tracker(5, writer);
    tracker.start();

    tracker.send({ type: EventType.Started, name: "pkg-a" });
    const line = writer.last();
    expect(line).toContain("\u2591".repeat(20));
  });

  it("should show elapsed time", () => {
    writer = new MockWriter();
    const tracker = new Tracker(5, writer);
    tracker.start();

    tracker.send({ type: EventType.Started, name: "pkg-a" });
    const line = writer.last();
    // Should contain something like (0.0s)
    expect(line).toMatch(/\(\d+\.\ds\)/);
  });

  it("should show labeled flat tracker format", () => {
    writer = new MockWriter();
    const tracker = new Tracker(3, writer, "Level");
    tracker.start();

    tracker.send({ type: EventType.Started, name: "pkg-a" });
    const line = writer.last();
    expect(line).toContain("Level 0/3");
  });

  it("should write a newline on stop", () => {
    writer = new MockWriter();
    const tracker = new Tracker(1, writer);
    tracker.start();
    tracker.stop();
    expect(writer.lines[writer.lines.length - 1]).toBe("\n");
  });

  it("should handle zero total gracefully", () => {
    writer = new MockWriter();
    const tracker = new Tracker(0, writer);
    tracker.start();

    tracker.send({ type: EventType.Started, name: "x" });
    const line = writer.last();
    // Should show all empty blocks when total is 0
    expect(line).toContain("\u2591".repeat(20));
  });
});

// ---------------------------------------------------------------------------
// Name display and truncation tests
// ---------------------------------------------------------------------------

describe("formatActivity", () => {
  it("should show 'waiting...' when nothing in-flight and not done", () => {
    const result = formatActivity(new Map(), 0, 5);
    expect(result).toBe("waiting...");
  });

  it("should show 'done' when nothing in-flight and completed >= total", () => {
    const result = formatActivity(new Map(), 5, 5);
    expect(result).toBe("done");
  });

  it("should show 'done' when completed exceeds total", () => {
    const result = formatActivity(new Map(), 10, 5);
    expect(result).toBe("done");
  });

  it("should show single name", () => {
    const map = new Map([["pkg-a", true]]);
    expect(formatActivity(map, 0, 5)).toBe("Building: pkg-a");
  });

  it("should show up to 3 names sorted", () => {
    const map = new Map<string, boolean>([
      ["pkg-c", true],
      ["pkg-a", true],
      ["pkg-b", true],
    ]);
    expect(formatActivity(map, 0, 5)).toBe("Building: pkg-a, pkg-b, pkg-c");
  });

  it("should truncate to 3 names with +N more", () => {
    const map = new Map<string, boolean>([
      ["pkg-d", true],
      ["pkg-c", true],
      ["pkg-a", true],
      ["pkg-b", true],
      ["pkg-e", true],
    ]);
    expect(formatActivity(map, 0, 10)).toBe(
      "Building: pkg-a, pkg-b, pkg-c +2 more",
    );
  });

  it("should show exactly 4 names as 3 + 1 more", () => {
    const map = new Map<string, boolean>([
      ["alpha", true],
      ["beta", true],
      ["gamma", true],
      ["delta", true],
    ]);
    expect(formatActivity(map, 0, 10)).toBe(
      "Building: alpha, beta, delta +1 more",
    );
  });
});

// ---------------------------------------------------------------------------
// Hierarchical progress tests
// ---------------------------------------------------------------------------

describe("Tracker hierarchical mode", () => {
  let writer: MockWriter;

  it("should create a child linked to parent", () => {
    writer = new MockWriter();
    const parent = new Tracker(3, writer, "Level");
    parent.start();

    const child = parent.child(7, "Package");
    expect(child.getTotal()).toBe(7);
    expect(child.getCompleted()).toBe(0);
  });

  it("should render child with parent context", () => {
    writer = new MockWriter();
    const parent = new Tracker(3, writer, "Level");
    parent.start();

    const child = parent.child(7, "Package");
    child.send({ type: EventType.Started, name: "pkg-a" });

    const line = writer.last();
    // Should show parent label with parent count
    expect(line).toContain("Level 1/3");
    // Should show child progress
    expect(line).toContain("0/7");
    expect(line).toContain("Building: pkg-a");
  });

  it("should advance parent on child.finish()", () => {
    writer = new MockWriter();
    const parent = new Tracker(3, writer, "Level");
    parent.start();

    const child = parent.child(7, "Package");
    child.send({ type: EventType.Finished, name: "pkg-a", status: "ok" });
    child.finish();

    expect(parent.getCompleted()).toBe(1);
  });

  it("should support multiple children in sequence", () => {
    writer = new MockWriter();
    const parent = new Tracker(2, writer, "Level");
    parent.start();

    const child1 = parent.child(3, "Package");
    child1.send({ type: EventType.Finished, name: "a", status: "ok" });
    child1.send({ type: EventType.Finished, name: "b", status: "ok" });
    child1.send({ type: EventType.Finished, name: "c", status: "ok" });
    child1.finish();

    expect(parent.getCompleted()).toBe(1);

    const child2 = parent.child(2, "Package");
    child2.send({ type: EventType.Finished, name: "d", status: "ok" });
    child2.send({ type: EventType.Finished, name: "e", status: "ok" });
    child2.finish();

    expect(parent.getCompleted()).toBe(2);
  });

  it("should not advance parent if child has no parent", () => {
    writer = new MockWriter();
    const standalone = new Tracker(5, writer);
    standalone.start();
    // finish() on a non-child tracker is a no-op
    standalone.finish();
    expect(standalone.getCompleted()).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// NullTracker tests
// ---------------------------------------------------------------------------

describe("NullTracker", () => {
  it("should be a no-op for all methods", () => {
    const nt = new NullTracker();

    // None of these should throw
    nt.start();
    nt.send({ type: EventType.Started, name: "pkg-a" });
    nt.send({ type: EventType.Finished, name: "pkg-a", status: "ok" });
    nt.finish();
    nt.stop();
  });

  it("should return a NullTracker from child()", () => {
    const nt = new NullTracker();
    const child = nt.child(10, "Test");
    expect(child).toBeInstanceOf(NullTracker);
  });

  it("should return zero/empty from getters", () => {
    const nt = new NullTracker();
    expect(nt.getCompleted()).toBe(0);
    expect(nt.getBuilding()).toEqual([]);
    expect(nt.getTotal()).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

describe("Edge cases", () => {
  it("should handle Finished for an item never Started", () => {
    const writer = new MockWriter();
    const tracker = new Tracker(5, writer);
    tracker.start();

    // Finishing an item that was never started — still increments completed
    tracker.send({ type: EventType.Finished, name: "ghost" });
    expect(tracker.getCompleted()).toBe(1);
    expect(tracker.getBuilding()).toEqual([]);
  });

  it("should handle more completions than total", () => {
    const writer = new MockWriter();
    const tracker = new Tracker(1, writer);
    tracker.start();

    tracker.send({ type: EventType.Finished, name: "a", status: "ok" });
    tracker.send({ type: EventType.Finished, name: "b", status: "ok" });

    expect(tracker.getCompleted()).toBe(2);
    // Bar should be capped at 20 filled chars, not overflow
    const line = writer.last();
    expect(line).toContain("\u2588".repeat(20));
  });

  it("should draw on every send call", () => {
    const writer = new MockWriter();
    const tracker = new Tracker(5, writer);
    tracker.start();

    tracker.send({ type: EventType.Started, name: "a" });
    tracker.send({ type: EventType.Started, name: "b" });
    tracker.send({ type: EventType.Finished, name: "a", status: "ok" });

    // Each send() triggers a draw — 3 sends = 3 lines
    expect(writer.lines.length).toBe(3);
  });

  it("should pad output to at least 80 characters", () => {
    const writer = new MockWriter();
    const tracker = new Tracker(1, writer);
    tracker.start();

    tracker.send({ type: EventType.Started, name: "x" });
    // The raw write includes padding
    expect(writer.lines[0].length).toBeGreaterThanOrEqual(80);
  });
});
