/**
 * forme-stage — event bus tests
 */

import { describe, it, expect } from "vitest";
import { inMemoryEventBus } from "../src/index.js";

describe("inMemoryEventBus", () => {
  it("delivers events to subscribers", () => {
    const bus = inMemoryEventBus();
    const seen: number[] = [];
    bus.on("ping", (p) => seen.push(p as number));
    bus.emit("ping", 1);
    bus.emit("ping", 2);
    expect(seen).toEqual([1, 2]);
  });

  it("multiple subscribers all receive each event in registration order", () => {
    const bus = inMemoryEventBus();
    const order: string[] = [];
    bus.on("e", () => order.push("a"));
    bus.on("e", () => order.push("b"));
    bus.on("e", () => order.push("c"));
    bus.emit("e", null);
    expect(order).toEqual(["a", "b", "c"]);
  });

  it("emit on an event with no subscribers is a no-op", () => {
    const bus = inMemoryEventBus();
    expect(() => bus.emit("nobody-listens", { x: 1 })).not.toThrow();
  });

  it("unsubscribe stops further deliveries", () => {
    const bus = inMemoryEventBus();
    let count = 0;
    const off = bus.on("e", () => count++);
    bus.emit("e", null);
    off();
    bus.emit("e", null);
    expect(count).toBe(1);
  });

  it("unsubscribe is idempotent — second call does not throw", () => {
    const bus = inMemoryEventBus();
    const off = bus.on("e", () => {});
    off();
    expect(() => off()).not.toThrow();
  });

  it("a handler that unsubscribes itself mid-emit does not skip siblings", () => {
    const bus = inMemoryEventBus();
    const order: string[] = [];
    let off1!: () => void;
    off1 = bus.on("e", () => { order.push("1"); off1(); });
    bus.on("e", () => order.push("2"));
    bus.emit("e", null);
    expect(order).toEqual(["1", "2"]);
  });

  it("a handler that throws does not break other subscribers", () => {
    const bus = inMemoryEventBus();
    let bRan = false;
    bus.on("e", () => { throw new Error("a failed"); });
    bus.on("e", () => { bRan = true; });
    expect(() => bus.emit("e", null)).not.toThrow();
    expect(bRan).toBe(true);
  });

  it("events are isolated by name", () => {
    const bus = inMemoryEventBus();
    let aCount = 0, bCount = 0;
    bus.on("a", () => aCount++);
    bus.on("b", () => bCount++);
    bus.emit("a", null); bus.emit("a", null); bus.emit("b", null);
    expect(aCount).toBe(2);
    expect(bCount).toBe(1);
  });
});
