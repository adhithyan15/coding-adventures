/**
 * Tests for synchronization primitives -- Fence, Semaphore, Event.
 */

import { describe, it, expect } from "vitest";
import { Fence, Semaphore, Event } from "../src/index.js";

describe("Fence", () => {
  it("defaults to unsignaled", () => {
    const fence = new Fence();
    expect(fence.signaled).toBe(false);
  });

  it("can be created signaled", () => {
    const fence = new Fence(true);
    expect(fence.signaled).toBe(true);
  });

  it("can be signaled", () => {
    const fence = new Fence();
    fence.signal();
    expect(fence.signaled).toBe(true);
  });

  it("wait returns true when signaled", () => {
    const fence = new Fence(true);
    expect(fence.wait()).toBe(true);
  });

  it("wait returns false when unsignaled", () => {
    const fence = new Fence();
    expect(fence.wait()).toBe(false);
  });

  it("can be reset", () => {
    const fence = new Fence(true);
    fence.reset();
    expect(fence.signaled).toBe(false);
  });

  it("supports reuse cycle", () => {
    const fence = new Fence();
    fence.signal();
    expect(fence.signaled).toBe(true);
    fence.reset();
    expect(fence.signaled).toBe(false);
    fence.signal();
    expect(fence.signaled).toBe(true);
  });

  it("has unique IDs", () => {
    const f1 = new Fence();
    const f2 = new Fence();
    expect(f1.fenceId).not.toBe(f2.fenceId);
  });

  it("wait cycles starts at zero", () => {
    const fence = new Fence();
    expect(fence.waitCycles).toBe(0);
  });

  it("reset clears wait cycles", () => {
    const fence = new Fence();
    fence.reset();
    expect(fence.waitCycles).toBe(0);
  });
});

describe("Semaphore", () => {
  it("defaults to unsignaled", () => {
    const sem = new Semaphore();
    expect(sem.signaled).toBe(false);
  });

  it("can be signaled", () => {
    const sem = new Semaphore();
    sem.signal();
    expect(sem.signaled).toBe(true);
  });

  it("can be reset", () => {
    const sem = new Semaphore();
    sem.signal();
    sem.reset();
    expect(sem.signaled).toBe(false);
  });

  it("has unique IDs", () => {
    const s1 = new Semaphore();
    const s2 = new Semaphore();
    expect(s1.semaphoreId).not.toBe(s2.semaphoreId);
  });
});

describe("Event", () => {
  it("defaults to unsignaled", () => {
    const event = new Event();
    expect(event.signaled).toBe(false);
  });

  it("can be set", () => {
    const event = new Event();
    event.set();
    expect(event.signaled).toBe(true);
  });

  it("can be reset", () => {
    const event = new Event();
    event.set();
    event.reset();
    expect(event.signaled).toBe(false);
  });

  it("status reports correctly", () => {
    const event = new Event();
    expect(event.status()).toBe(false);
    event.set();
    expect(event.status()).toBe(true);
  });

  it("has unique IDs", () => {
    const e1 = new Event();
    const e2 = new Event();
    expect(e1.eventId).not.toBe(e2.eventId);
  });
});
