import { describe, it, expect } from "vitest";
import { SystemBoard, defaultSystemConfig, BootPhase } from "../src/index.js";

describe("SystemBoard", () => {
  it("powers on successfully", () => {
    const board = new SystemBoard(defaultSystemConfig());
    board.powerOn();
    expect(board.powered).toBe(true);
    expect(board.getCurrentPhase()).toBe(BootPhase.BIOS);
  });

  it("boots to hello-world and displays output", () => {
    const board = new SystemBoard(defaultSystemConfig());
    board.powerOn();
    board.run(100000);

    const snap = board.displaySnapshot();
    expect(snap).not.toBeNull();
    expect(snap!.contains("Hello World")).toBe(true);
  });

  it("reaches idle phase after hello-world completes", () => {
    const board = new SystemBoard(defaultSystemConfig());
    board.powerOn();
    board.run(100000);
    expect(board.isIdle()).toBe(true);
  });

  it("boot trace records all phases", () => {
    const board = new SystemBoard(defaultSystemConfig());
    board.powerOn();
    board.run(100000);

    const trace = board.trace;
    expect(trace.events.length).toBeGreaterThan(0);
    const phases = trace.phases();
    expect(phases).toContain(BootPhase.PowerOn);
    expect(phases).toContain(BootPhase.BIOS);
  });

  it("cycle count increases during execution", () => {
    const board = new SystemBoard(defaultSystemConfig());
    board.powerOn();
    board.run(100);
    expect(board.getCycleCount()).toBe(100);
  });

  it("powerOn is idempotent", () => {
    const board = new SystemBoard(defaultSystemConfig());
    board.powerOn();
    board.powerOn(); // second call should be no-op
    expect(board.powered).toBe(true);
  });
});
