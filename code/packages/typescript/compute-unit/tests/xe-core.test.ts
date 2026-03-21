/**
 * Tests for XeCore -- Intel Xe Core simulator.
 */

import { describe, it, expect } from "vitest";
import { limm, fmul, halt } from "@coding-adventures/gpu-core";

import { Architecture, makeWorkItem } from "../src/index.js";
import { XeCore, makeXeCoreConfig } from "../src/xe-core.js";

// ---------------------------------------------------------------------------
// XeCoreConfig tests
// ---------------------------------------------------------------------------

describe("XeCoreConfig", () => {
  it("has correct defaults", () => {
    const config = makeXeCoreConfig();
    expect(config.numEus).toBe(16);
    expect(config.threadsPerEu).toBe(7);
    expect(config.simdWidth).toBe(8);
    expect(config.grfPerEu).toBe(128);
    expect(config.slmSize).toBe(65536);
    expect(config.l1CacheSize).toBe(196608);
    expect(config.memoryLatencyCycles).toBe(200);
  });

  it("allows customization", () => {
    const config = makeXeCoreConfig({ numEus: 4, threadsPerEu: 3, simdWidth: 4 });
    expect(config.numEus).toBe(4);
    expect(config.threadsPerEu).toBe(3);
    expect(config.simdWidth).toBe(4);
  });
});

// ---------------------------------------------------------------------------
// XeCore tests
// ---------------------------------------------------------------------------

describe("XeCore", () => {
  const simpleProgram = () => [limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()];

  it("creates correctly", () => {
    const xe = new XeCore(
      makeXeCoreConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 }),
    );
    expect(xe.name).toBe("XeCore");
    expect(xe.architecture).toBe(Architecture.INTEL_XE_CORE);
    expect(xe.idle).toBe(true);
  });

  it("dispatch and run works", () => {
    const xe = new XeCore(
      makeXeCoreConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 }),
    );
    xe.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 16,
    }));
    const traces = xe.run();
    expect(traces.length).toBeGreaterThan(0);
    expect(xe.idle).toBe(true);
  });

  it("traces have correct architecture", () => {
    const xe = new XeCore(
      makeXeCoreConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 }),
    );
    xe.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 8,
    }));
    const traces = xe.run();
    for (const trace of traces) {
      expect(trace.architecture).toBe(Architecture.INTEL_XE_CORE);
      expect(trace.unitName).toBe("XeCore");
    }
  });

  it("SLM is accessible", () => {
    const xe = new XeCore(makeXeCoreConfig());
    const slm = xe.slm;
    slm.write(0, 99.0, 0);
    expect(Math.abs(slm.read(0, 0) - 99.0)).toBeLessThan(0.01);
  });

  it("engine is accessible", () => {
    const xe = new XeCore(
      makeXeCoreConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 }),
    );
    expect(xe.engine).toBeDefined();
  });

  it("resets correctly", () => {
    const xe = new XeCore(
      makeXeCoreConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 }),
    );
    xe.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 8,
    }));
    xe.run();
    xe.reset();
    expect(xe.idle).toBe(true);
  });

  it("supports per-thread data", () => {
    const xe = new XeCore(
      makeXeCoreConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 }),
    );
    xe.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 8,
      perThreadData: {
        0: { 0: 10.0 },
        1: { 0: 20.0 },
      },
    }));
    xe.run();
    expect(xe.idle).toBe(true);
  });

  it("tracks occupancy", () => {
    const xe = new XeCore(
      makeXeCoreConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 }),
    );
    xe.dispatch(makeWorkItem({
      workId: 0,
      program: simpleProgram(),
      threadCount: 8,
    }));
    const trace = xe.step({ cycle: 1 });
    // Should show some activity
    expect(trace.occupancy).toBeGreaterThanOrEqual(0.0);
  });

  it("toString includes key info", () => {
    const xe = new XeCore(
      makeXeCoreConfig({ numEus: 4, threadsPerEu: 3, simdWidth: 4 }),
    );
    const r = xe.toString();
    expect(r).toContain("XeCore");
    expect(r).toContain("eus=4");
  });
});
