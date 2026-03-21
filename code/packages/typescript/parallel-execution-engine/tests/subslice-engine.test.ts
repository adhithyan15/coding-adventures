/**
 * Tests for SubsliceEngine -- Intel Xe hybrid SIMD execution engine.
 */

import { describe, it, expect } from "vitest";
import { limm, halt } from "@coding-adventures/gpu-core";
import {
  SubsliceEngine,
  makeSubsliceConfig,
  ExecutionUnit,
  ExecutionModel,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// SubsliceConfig
// ---------------------------------------------------------------------------

describe("SubsliceConfig", () => {
  it("has sensible defaults", () => {
    const config = makeSubsliceConfig();
    expect(config.numEus).toBe(8);
    expect(config.threadsPerEu).toBe(7);
    expect(config.simdWidth).toBe(8);
    expect(config.grfSize).toBe(128);
    expect(config.slmSize).toBe(65536);
  });

  it("can be customized", () => {
    const config = makeSubsliceConfig({
      numEus: 4,
      threadsPerEu: 2,
      simdWidth: 4,
    });
    expect(config.numEus).toBe(4);
    expect(config.threadsPerEu).toBe(2);
    expect(config.simdWidth).toBe(4);
  });
});

// ---------------------------------------------------------------------------
// ExecutionUnit
// ---------------------------------------------------------------------------

describe("ExecutionUnit", () => {
  it("creation", () => {
    const config = makeSubsliceConfig({
      numEus: 1,
      threadsPerEu: 2,
      simdWidth: 4,
    });
    const eu = new ExecutionUnit(0, config);
    expect(eu.euId).toBe(0);
    expect(eu.threads.length).toBe(2);
    expect(eu.threads[0].length).toBe(4);
  });

  it("loadProgram", () => {
    const config = makeSubsliceConfig({
      numEus: 1,
      threadsPerEu: 2,
      simdWidth: 2,
    });
    const eu = new ExecutionUnit(0, config);
    eu.loadProgram([limm(0, 1.0), halt()]);
    expect(eu.allHalted).toBe(false);
  });

  it("step", () => {
    const config = makeSubsliceConfig({
      numEus: 1,
      threadsPerEu: 2,
      simdWidth: 2,
    });
    const eu = new ExecutionUnit(0, config);
    eu.loadProgram([limm(0, 1.0), halt()]);
    const traces = eu.step();
    expect(Object.keys(traces).length).toBeGreaterThan(0);
  });

  it("allHalted after halt", () => {
    const config = makeSubsliceConfig({
      numEus: 1,
      threadsPerEu: 1,
      simdWidth: 2,
    });
    const eu = new ExecutionUnit(0, config);
    eu.loadProgram([halt()]);
    eu.step();
    expect(eu.allHalted).toBe(true);
  });

  it("setThreadLaneRegister", () => {
    const config = makeSubsliceConfig({
      numEus: 1,
      threadsPerEu: 2,
      simdWidth: 2,
    });
    const eu = new ExecutionUnit(0, config);
    eu.loadProgram([limm(0, 1.0), halt()]);
    eu.setThreadLaneRegister(0, 1, 5, 42.0);
    expect(eu.threads[0][1].registers.readFloat(5)).toBe(42.0);
  });

  it("reset", () => {
    const config = makeSubsliceConfig({
      numEus: 1,
      threadsPerEu: 1,
      simdWidth: 2,
    });
    const eu = new ExecutionUnit(0, config);
    eu.loadProgram([halt()]);
    eu.step();
    expect(eu.allHalted).toBe(true);

    eu.reset();
    expect(eu.allHalted).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// SubsliceEngine -- basic properties
// ---------------------------------------------------------------------------

describe("SubsliceEngine properties", () => {
  it("has correct name", () => {
    const engine = new SubsliceEngine(
      makeSubsliceConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 }),
    );
    expect(engine.name).toBe("SubsliceEngine");
  });

  it("has correct width", () => {
    const engine = new SubsliceEngine(
      makeSubsliceConfig({ numEus: 2, threadsPerEu: 3, simdWidth: 4 }),
    );
    expect(engine.width).toBe(2 * 3 * 4); // 24
  });

  it("reports SIMD execution model", () => {
    const engine = new SubsliceEngine(
      makeSubsliceConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 }),
    );
    expect(engine.executionModel).toBe(ExecutionModel.SIMD);
  });

  it("starts not halted", () => {
    const engine = new SubsliceEngine(
      makeSubsliceConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 }),
    );
    expect(engine.halted).toBe(false);
  });

  it("exposes config", () => {
    const config = makeSubsliceConfig({
      numEus: 2,
      threadsPerEu: 2,
      simdWidth: 4,
    });
    const engine = new SubsliceEngine(config);
    expect(engine.config).toBe(config);
  });

  it("exposes eus", () => {
    const engine = new SubsliceEngine(
      makeSubsliceConfig({ numEus: 3, threadsPerEu: 2, simdWidth: 4 }),
    );
    expect(engine.eus.length).toBe(3);
  });

  it("has readable toString", () => {
    const engine = new SubsliceEngine(
      makeSubsliceConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 4 }),
    );
    expect(engine.toString()).toContain("SubsliceEngine");
  });
});

// ---------------------------------------------------------------------------
// SubsliceEngine -- execution
// ---------------------------------------------------------------------------

describe("SubsliceEngine execution", () => {
  it("simple program runs to completion", () => {
    const engine = new SubsliceEngine(
      makeSubsliceConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 2 }),
    );
    engine.loadProgram([limm(0, 42.0), halt()]);
    const traces = engine.run();
    expect(traces.length).toBeGreaterThan(0);
    expect(engine.halted).toBe(true);
  });

  it("setEuThreadLaneRegister", () => {
    const engine = new SubsliceEngine(
      makeSubsliceConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 2 }),
    );
    engine.loadProgram([limm(0, 1.0), halt()]);
    engine.setEuThreadLaneRegister(0, 0, 1, 5, 99.0);

    expect(engine.eus[0].threads[0][1].registers.readFloat(5)).toBe(99.0);
  });

  it("step produces trace", () => {
    const engine = new SubsliceEngine(
      makeSubsliceConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 2 }),
    );
    engine.loadProgram([limm(0, 1.0), halt()]);

    const trace = engine.step({ cycle: 1 });
    expect(trace.cycle).toBe(1);
    expect(trace.engineName).toBe("SubsliceEngine");
    expect(trace.totalCount).toBe(2 * 2 * 2); // 8
  });

  it("halted step returns trace", () => {
    const engine = new SubsliceEngine(
      makeSubsliceConfig({ numEus: 1, threadsPerEu: 1, simdWidth: 2 }),
    );
    engine.loadProgram([halt()]);
    engine.run();
    expect(engine.halted).toBe(true);

    const trace = engine.step({ cycle: 99 });
    expect(trace.activeCount).toBe(0);
    expect(trace.description.toLowerCase()).toContain("halted");
  });
});

// ---------------------------------------------------------------------------
// SubsliceEngine -- reset
// ---------------------------------------------------------------------------

describe("SubsliceEngine reset", () => {
  it("resets to initial state", () => {
    const engine = new SubsliceEngine(
      makeSubsliceConfig({ numEus: 2, threadsPerEu: 2, simdWidth: 2 }),
    );
    engine.loadProgram([limm(0, 42.0), halt()]);
    engine.run();
    expect(engine.halted).toBe(true);

    engine.reset();
    expect(engine.halted).toBe(false);

    engine.run();
    expect(engine.halted).toBe(true);
  });
});
