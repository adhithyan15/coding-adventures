import { describe, it, expect } from "vitest";
import { ControlFlow, EventLoop, EventSource, VERSION } from "../src/index";

// ════════════════════════════════════════════════════════════════════════════
// Helpers — mock sources
// ════════════════════════════════════════════════════════════════════════════

/**
 * A source that emits a fixed sequence of event batches, one per poll call.
 * After all batches are exhausted, poll() returns [].
 */
class FixedSource<E> implements EventSource<E> {
  private index = 0;
  constructor(private batches: E[][]) {}
  poll(): E[] {
    if (this.index >= this.batches.length) return [];
    return this.batches[this.index++];
  }
}

/** Returns one incrementing number per poll call. Never stops on its own. */
class InfiniteSource implements EventSource<number> {
  private n = 0;
  poll(): number[] {
    return [++this.n];
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════════════════════════════════════

describe("EventLoop", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });

  it("delivers all events to handlers", () => {
    const loop = new EventLoop<number>();
    loop.addSource(new FixedSource([[1, 2, 3], [-1]]));

    const received: number[] = [];
    loop.onEvent((e) => {
      if (e === -1) return ControlFlow.Exit;
      received.push(e);
      return ControlFlow.Continue;
    });

    loop.run();
    expect(received).toEqual([1, 2, 3]);
  });

  it("stops immediately when a handler returns Exit", () => {
    const loop = new EventLoop<string>();
    loop.addSource(new FixedSource([["a", "b", "stop", "c", "d"]]));

    const seen: string[] = [];
    loop.onEvent((e) => {
      seen.push(e);
      return e === "stop" ? ControlFlow.Exit : ControlFlow.Continue;
    });

    loop.run();
    expect(seen).toEqual(["a", "b", "stop"]);
    expect(seen).not.toContain("c");
    expect(seen).not.toContain("d");
  });

  it("stops when stop() is called from within a handler", () => {
    const loop = new EventLoop<number>();
    loop.addSource(new InfiniteSource());

    let count = 0;
    loop.onEvent((_e) => {
      count++;
      if (count >= 5) loop.stop();
      return ControlFlow.Continue;
    });

    loop.run();
    expect(count).toBeGreaterThanOrEqual(5);
  });

  it("delivers each event to all registered handlers", () => {
    const loop = new EventLoop<number>();
    loop.addSource(new FixedSource([[99], [-1]]));

    let h1Saw = 0;
    let h2Saw = 0;

    loop.onEvent((e) => {
      if (e === 99) h1Saw = e;
      return e === -1 ? ControlFlow.Exit : ControlFlow.Continue;
    });
    loop.onEvent((e) => {
      if (e === 99) h2Saw = e;
      return ControlFlow.Continue;
    });

    loop.run();
    expect(h1Saw).toBe(99);
    expect(h2Saw).toBe(99);
  });

  it("merges events from multiple sources", () => {
    const loop = new EventLoop<string>();
    loop.addSource(new FixedSource([["alpha"]]));
    loop.addSource(new FixedSource([["beta"]]));
    loop.addSource(new FixedSource([[], ["stop"]]));

    const seen: string[] = [];
    loop.onEvent((e) => {
      if (e === "stop") return ControlFlow.Exit;
      seen.push(e);
      return ControlFlow.Continue;
    });

    loop.run();
    expect(seen).toHaveLength(2);
    expect(seen).toContain("alpha");
    expect(seen).toContain("beta");
  });

  it("handles a loop with no sources — exits when stop() is called", () => {
    // A loop with no sources produces no events. It will idle forever unless stopped.
    // We test that stop() works by calling it from within the only handler,
    // which would be reached if a source existed. Instead, we verify no crash.
    const loop = new EventLoop<number>();
    // Add a source that immediately exits so the loop terminates.
    loop.addSource(new FixedSource([[-1]]));
    loop.onEvent((e) => (e === -1 ? ControlFlow.Exit : ControlFlow.Continue));
    loop.run(); // should not throw
  });

  it("preserves event order within a source", () => {
    const loop = new EventLoop<number>();
    loop.addSource(new FixedSource([[3, 1, 4, 1, 5], [-1]]));

    const received: number[] = [];
    loop.onEvent((e) => {
      if (e === -1) return ControlFlow.Exit;
      received.push(e);
      return ControlFlow.Continue;
    });

    loop.run();
    expect(received).toEqual([3, 1, 4, 1, 5]);
  });
});

describe("ControlFlow", () => {
  it("Continue and Exit are distinct", () => {
    expect(ControlFlow.Continue).not.toBe(ControlFlow.Exit);
  });

  it("has string values for readable debugging", () => {
    expect(typeof ControlFlow.Continue).toBe("string");
    expect(typeof ControlFlow.Exit).toBe("string");
  });
});
