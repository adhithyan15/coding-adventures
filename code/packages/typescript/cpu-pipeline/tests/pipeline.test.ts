/**
 * Comprehensive tests for the CPU Pipeline package.
 *
 * These tests cover:
 * - Token creation, cloning, and string representation
 * - Pipeline configuration and validation
 * - Pipeline creation and basic operation
 * - Stall mechanics
 * - Flush mechanics
 * - Forwarding integration
 * - Statistics (IPC, CPI)
 * - Trace and snapshot accuracy
 * - Deep pipeline configurations
 * - Branch prediction integration
 * - Halt behavior
 */

import { describe, expect, it } from "vitest";
import {
  type DecodeFunc,
  type ExecuteFunc,
  type FetchFunc,
  HazardAction,
  type HazardResponse,
  type MemoryFunc,
  Pipeline,
  type PipelineConfig,
  PipelineStats,
  type PipelineToken,
  StageCategory,
  type WritebackFunc,
  classic5Stage,
  cloneToken,
  deep13Stage,
  newBubble,
  newToken,
  noHazard,
  numStages,
  tokenToString,
  validateConfig,
} from "../src/index.js";

// =========================================================================
// Test helpers -- simple instruction memory and callbacks
// =========================================================================
//
// For testing, we create a tiny "instruction memory" -- just an array of
// numbers. Each number represents one instruction's raw bits. The fetch
// callback reads from this array using PC/4 as the index.
//
// Encoding: raw = (opcode << 24) | (rd << 16) | (rs1 << 8) | rs2

const OP_NOP = 0x00;
const OP_ADD = 0x01;
const OP_LDR = 0x02;
const OP_STR = 0x03;
const OP_BEQ = 0x04;
const OP_HALT = 0xff;

function makeInstruction(opcode: number, rd: number, rs1: number, rs2: number): number {
  return (opcode << 24) | (rd << 16) | (rs1 << 8) | rs2;
}

function simpleFetch(instrs: number[]): FetchFunc {
  return (pc: number): number => {
    const idx = Math.floor(pc / 4);
    if (idx < 0 || idx >= instrs.length) {
      return makeInstruction(OP_NOP, 0, 0, 0);
    }
    return instrs[idx];
  };
}

function simpleDecode(): DecodeFunc {
  return (raw: number, tok: PipelineToken): PipelineToken => {
    const opcode = (raw >> 24) & 0xff;
    const rd = (raw >> 16) & 0xff;
    const rs1 = (raw >> 8) & 0xff;
    const rs2 = raw & 0xff;

    switch (opcode) {
      case OP_ADD:
        tok.opcode = "ADD";
        tok.rd = rd;
        tok.rs1 = rs1;
        tok.rs2 = rs2;
        tok.regWrite = true;
        break;
      case OP_LDR:
        tok.opcode = "LDR";
        tok.rd = rd;
        tok.rs1 = rs1;
        tok.memRead = true;
        tok.regWrite = true;
        break;
      case OP_STR:
        tok.opcode = "STR";
        tok.rs1 = rs1;
        tok.rs2 = rs2;
        tok.memWrite = true;
        break;
      case OP_BEQ:
        tok.opcode = "BEQ";
        tok.rs1 = rs1;
        tok.rs2 = rs2;
        tok.isBranch = true;
        break;
      case OP_HALT:
        tok.opcode = "HALT";
        tok.isHalt = true;
        break;
      default:
        tok.opcode = "NOP";
        break;
    }
    return tok;
  };
}

function simpleExecute(): ExecuteFunc {
  return (tok: PipelineToken): PipelineToken => {
    switch (tok.opcode) {
      case "ADD":
        tok.aluResult = tok.rs1 + tok.rs2;
        break;
      case "LDR":
        tok.aluResult = tok.rs1 + tok.immediate;
        break;
      case "STR":
        tok.aluResult = tok.rs1 + tok.immediate;
        break;
      case "BEQ":
        tok.branchTarget = tok.pc + tok.immediate;
        break;
    }
    return tok;
  };
}

function simpleMemory(): MemoryFunc {
  return (tok: PipelineToken): PipelineToken => {
    if (tok.memRead) {
      tok.memData = 42;
      tok.writeData = tok.memData;
    } else {
      tok.writeData = tok.aluResult;
    }
    return tok;
  };
}

interface CompletedInstructions {
  pcs: number[];
}

function simpleWriteback(completed: CompletedInstructions | null): WritebackFunc {
  return (tok: PipelineToken): void => {
    if (completed !== null) {
      completed.pcs.push(tok.pc);
    }
  };
}

function newTestPipeline(instrs: number[], completed: CompletedInstructions | null): Pipeline {
  const config = classic5Stage();
  return Pipeline.create(
    config,
    simpleFetch(instrs),
    simpleDecode(),
    simpleExecute(),
    simpleMemory(),
    simpleWriteback(completed),
  );
}

// =========================================================================
// Token tests
// =========================================================================

describe("Token", () => {
  it("creates a new token with default register values", () => {
    const tok = newToken();
    expect(tok.rs1).toBe(-1);
    expect(tok.rs2).toBe(-1);
    expect(tok.rd).toBe(-1);
    expect(tok.isBubble).toBe(false);
    expect(tok.stageEntered).toBeDefined();
  });

  it("creates a bubble", () => {
    const b = newBubble();
    expect(b.isBubble).toBe(true);
    expect(tokenToString(b)).toBe("---");
  });

  it("formats token string with opcode", () => {
    const tok = newToken();
    tok.opcode = "ADD";
    tok.pc = 100;
    expect(tokenToString(tok)).toBe("ADD@100");
  });

  it("formats token string without opcode", () => {
    const tok = newToken();
    tok.pc = 200;
    expect(tokenToString(tok)).toBe("instr@200");
  });

  it("clones a token deeply", () => {
    const tok = newToken();
    tok.pc = 100;
    tok.opcode = "ADD";
    tok.stageEntered["IF"] = 1;
    tok.stageEntered["ID"] = 2;

    const clone = cloneToken(tok);
    expect(clone).not.toBeNull();
    expect(clone!.pc).toBe(100);
    expect(clone!.opcode).toBe("ADD");

    // Mutating the clone should not affect the original.
    clone!.stageEntered["EX"] = 3;
    expect(tok.stageEntered["EX"]).toBeUndefined();
  });

  it("clones null returns null", () => {
    const clone = cloneToken(null);
    expect(clone).toBeNull();
  });
});

// =========================================================================
// PipelineConfig tests
// =========================================================================

describe("PipelineConfig", () => {
  it("classic 5-stage has 5 stages and is valid", () => {
    const config = classic5Stage();
    expect(numStages(config)).toBe(5);
    expect(validateConfig(config)).toBeNull();
    expect(config.stages[0].name).toBe("IF");
    expect(config.stages[4].name).toBe("WB");
  });

  it("deep 13-stage has 13 stages and is valid", () => {
    const config = deep13Stage();
    expect(numStages(config)).toBe(13);
    expect(validateConfig(config)).toBeNull();
  });

  it("rejects config with fewer than 2 stages", () => {
    const config: PipelineConfig = {
      stages: [{ name: "IF", description: "Fetch", category: StageCategory.Fetch }],
      executionWidth: 1,
    };
    const err = validateConfig(config);
    expect(err).not.toBeNull();
    expect(err).toContain("at least 2 stages");
  });

  it("rejects config with zero execution width", () => {
    const config = classic5Stage();
    config.executionWidth = 0;
    const err = validateConfig(config);
    expect(err).not.toBeNull();
    expect(err).toContain("execution width");
  });

  it("rejects config with duplicate stage names", () => {
    const config: PipelineConfig = {
      stages: [
        { name: "IF", description: "Fetch", category: StageCategory.Fetch },
        { name: "IF", description: "Fetch 2", category: StageCategory.Fetch },
        { name: "WB", description: "Writeback", category: StageCategory.Writeback },
      ],
      executionWidth: 1,
    };
    const err = validateConfig(config);
    expect(err).not.toBeNull();
    expect(err).toContain("duplicate");
  });

  it("rejects config without fetch stage", () => {
    const config: PipelineConfig = {
      stages: [
        { name: "EX", description: "Execute", category: StageCategory.Execute },
        { name: "WB", description: "Writeback", category: StageCategory.Writeback },
      ],
      executionWidth: 1,
    };
    const err = validateConfig(config);
    expect(err).not.toBeNull();
    expect(err).toContain("fetch");
  });

  it("rejects config without writeback stage", () => {
    const config: PipelineConfig = {
      stages: [
        { name: "IF", description: "Fetch", category: StageCategory.Fetch },
        { name: "EX", description: "Execute", category: StageCategory.Execute },
      ],
      executionWidth: 1,
    };
    const err = validateConfig(config);
    expect(err).not.toBeNull();
    expect(err).toContain("writeback");
  });
});

// =========================================================================
// Pipeline creation tests
// =========================================================================

describe("Pipeline creation", () => {
  it("creates a pipeline with valid config", () => {
    const instrs = [makeInstruction(OP_ADD, 1, 2, 3)];
    const p = newTestPipeline(instrs, null);
    expect(p).toBeDefined();
    expect(p.isHalted()).toBe(false);
    expect(p.cycle()).toBe(0);
    expect(p.pc()).toBe(0);
  });

  it("throws on invalid config", () => {
    const config: PipelineConfig = {
      stages: [{ name: "IF", description: "Fetch", category: StageCategory.Fetch }],
      executionWidth: 1,
    };
    expect(() =>
      Pipeline.create(
        config,
        () => 0,
        (_, t) => t,
        t => t,
        t => t,
        () => {},
      ),
    ).toThrow();
  });
});

// =========================================================================
// Pipeline execution tests
// =========================================================================

describe("Pipeline execution", () => {
  it("halts when HALT instruction reaches WB", () => {
    const instrs = [
      makeInstruction(OP_ADD, 1, 2, 3),
      makeInstruction(OP_ADD, 4, 5, 6),
      makeInstruction(OP_HALT, 0, 0, 0),
    ];

    const p = newTestPipeline(instrs, null);
    const stats = p.run(100);

    expect(p.isHalted()).toBe(true);
    expect(stats.totalCycles).toBeGreaterThan(0);
  });

  it("does not advance after halting", () => {
    const instrs = [makeInstruction(OP_HALT, 0, 0, 0)];
    const p = newTestPipeline(instrs, null);

    // Run until halted.
    p.run(20);
    expect(p.isHalted()).toBe(true);
    const cycleAfterHalt = p.cycle();

    // Calling step again should not change the cycle.
    p.step();
    expect(p.cycle()).toBe(cycleAfterHalt);
  });

  it("completes first instruction at cycle 5 in 5-stage pipeline", () => {
    const instrs = Array(10).fill(makeInstruction(OP_ADD, 1, 2, 3));
    const completed: CompletedInstructions = { pcs: [] };
    const p = newTestPipeline(instrs, completed);

    // After 4 cycles, no instruction should have completed.
    for (let i = 0; i < 4; i++) p.step();
    expect(completed.pcs.length).toBe(0);

    // After cycle 5, exactly 1 instruction should have completed.
    p.step();
    expect(completed.pcs.length).toBe(1);

    // After cycle 6, 2 completions.
    p.step();
    expect(completed.pcs.length).toBe(2);

    // After cycle 7, 3 completions.
    p.step();
    expect(completed.pcs.length).toBe(3);
  });

  it("achieves near-1.0 IPC for independent instructions", () => {
    const instrs = Array(100).fill(makeInstruction(OP_ADD, 1, 2, 3));
    const p = newTestPipeline(instrs, null);

    for (let i = 0; i < 50; i++) p.step();

    const stats = p.stats();
    // After 50 cycles of a 5-stage pipeline: completed = 50 - 5 + 1 = 46
    const expectedCompleted = 50 - 5 + 1;
    expect(stats.instructionsCompleted).toBe(expectedCompleted);

    const ipc = stats.ipc();
    expect(ipc).toBeGreaterThan(0.8);
    expect(ipc).toBeLessThanOrEqual(1.0);
  });

  it("respects maxCycles in run()", () => {
    const instrs = Array(1000).fill(makeInstruction(OP_ADD, 1, 2, 3));
    const p = newTestPipeline(instrs, null);

    p.run(10);
    expect(p.cycle()).toBe(10);
  });
});

// =========================================================================
// Stall tests
// =========================================================================

describe("Stall mechanics", () => {
  it("freezes earlier stages and inserts bubble", () => {
    const instrs = Array(20).fill(makeInstruction(OP_ADD, 1, 2, 3));
    const p = newTestPipeline(instrs, null);

    let cycleCount = 0;
    p.setHazardFunc(() => {
      cycleCount++;
      // Stall at cycle 3
      if (cycleCount === 3) {
        return {
          ...noHazard(),
          action: HazardAction.Stall,
          stallStages: 2,
        };
      }
      return noHazard();
    });

    // Run 3 cycles
    p.step();
    p.step();
    const snap = p.step(); // cycle 3 -- stall

    expect(snap.stalled).toBe(true);
    expect(p.stats().stallCycles).toBe(1);
  });

  it("stall reduces IPC below 1.0", () => {
    const instrs = Array(50).fill(makeInstruction(OP_ADD, 1, 2, 3));
    const p = newTestPipeline(instrs, null);

    let cycleCount = 0;
    p.setHazardFunc(() => {
      cycleCount++;
      if (cycleCount % 5 === 0) {
        return {
          ...noHazard(),
          action: HazardAction.Stall,
          stallStages: 2,
        };
      }
      return noHazard();
    });

    for (let i = 0; i < 30; i++) p.step();

    const stats = p.stats();
    expect(stats.ipc()).toBeLessThan(1.0);
    expect(stats.stallCycles).toBeGreaterThan(0);
  });
});

// =========================================================================
// Flush tests
// =========================================================================

describe("Flush mechanics", () => {
  it("redirects PC and inserts bubbles", () => {
    const instrs = [
      makeInstruction(OP_BEQ, 0, 1, 2), // branch instruction
      makeInstruction(OP_ADD, 1, 2, 3), // speculative
      makeInstruction(OP_ADD, 4, 5, 6), // speculative
    ];

    // Large enough memory so we can fetch from PC=20.
    const bigInstrs = Array(100).fill(makeInstruction(OP_NOP, 0, 0, 0));
    bigInstrs[0] = instrs[0];
    bigInstrs[1] = instrs[1];
    bigInstrs[2] = instrs[2];

    const p = newTestPipeline(bigInstrs, null);

    // Flush when the branch reaches EX stage (cycle 3 or 4).
    let flushed = false;
    p.setHazardFunc((stages) => {
      if (!flushed && stages.length >= 3) {
        const exTok = stages[2]; // EX is stage index 2
        if (exTok !== null && !exTok.isBubble && exTok.isBranch) {
          flushed = true;
          return {
            ...noHazard(),
            action: HazardAction.Flush,
            flushCount: 2,
            redirectPC: 20,
          };
        }
      }
      return noHazard();
    });

    p.step(); // cycle 1
    p.step(); // cycle 2
    p.step(); // cycle 3 -- BEQ enters EX

    const snap = p.step(); // cycle 4 -- flush should occur
    expect(snap.flushing).toBe(true);

    // After flush, PC should be redirected.
    expect(p.pc()).toBe(24); // 20 + 4 (advanced by fetch)

    expect(p.stats().flushCycles).toBe(1);
  });
});

// =========================================================================
// Forwarding tests
// =========================================================================

describe("Forwarding", () => {
  it("applies forwarded value to decode stage token", () => {
    const instrs = Array(10).fill(makeInstruction(OP_ADD, 1, 2, 3));
    const p = newTestPipeline(instrs, null);

    let forwardCycle = 0;
    p.setHazardFunc(() => {
      forwardCycle++;
      if (forwardCycle === 4) {
        return {
          ...noHazard(),
          action: HazardAction.ForwardFromEX,
          forwardValue: 99,
          forwardSource: "EX",
        };
      }
      return noHazard();
    });

    for (let i = 0; i < 4; i++) p.step();

    // The forwarded token should have ForwardedFrom set.
    const exTok = p.stageContents("EX");
    expect(exTok).not.toBeNull();
    expect(exTok!.forwardedFrom).toBe("EX");
  });
});

// =========================================================================
// Statistics tests
// =========================================================================

describe("PipelineStats", () => {
  it("calculates IPC correctly", () => {
    const stats = new PipelineStats();
    stats.totalCycles = 100;
    stats.instructionsCompleted = 80;
    expect(Math.abs(stats.ipc() - 0.8)).toBeLessThan(0.001);
  });

  it("calculates CPI correctly", () => {
    const stats = new PipelineStats();
    stats.totalCycles = 120;
    stats.instructionsCompleted = 100;
    expect(Math.abs(stats.cpi() - 1.2)).toBeLessThan(0.001);
  });

  it("returns 0 IPC for zero cycles", () => {
    const stats = new PipelineStats();
    expect(stats.ipc()).toBe(0.0);
  });

  it("returns 0 CPI for zero instructions", () => {
    const stats = new PipelineStats();
    stats.totalCycles = 10;
    expect(stats.cpi()).toBe(0.0);
  });

  it("produces non-empty string representation", () => {
    const stats = new PipelineStats();
    stats.totalCycles = 100;
    stats.instructionsCompleted = 80;
    stats.stallCycles = 5;
    stats.flushCycles = 3;
    stats.bubbleCycles = 10;
    const s = stats.toString();
    expect(s).not.toBe("");
    expect(s).toContain("100");
    expect(s).toContain("80");
  });
});

// =========================================================================
// Snapshot and trace tests
// =========================================================================

describe("Snapshots and traces", () => {
  it("snapshot reflects pipeline contents accurately", () => {
    const instrs = [
      makeInstruction(OP_ADD, 1, 2, 3),
      makeInstruction(OP_ADD, 4, 5, 6),
      makeInstruction(OP_NOP, 0, 0, 0),
    ];

    const p = newTestPipeline(instrs, null);

    // After 1 cycle, only IF has a token.
    const snap1 = p.step();
    expect(snap1.cycle).toBe(1);
    const ifTok = snap1.stages["IF"];
    expect(ifTok).toBeDefined();
    expect(ifTok.pc).toBe(0);

    // After 2 cycles, IF has second instruction, ID has first.
    const snap2 = p.step();
    expect(snap2.cycle).toBe(2);
    const idTok = snap2.stages["ID"];
    expect(idTok).toBeDefined();
    expect(idTok.pc).toBe(0);
  });

  it("trace has one entry per cycle", () => {
    const instrs = Array(10).fill(makeInstruction(OP_ADD, 1, 2, 3));
    const p = newTestPipeline(instrs, null);

    for (let i = 0; i < 7; i++) p.step();

    const trace = p.trace();
    expect(trace.length).toBe(7);

    // Verify cycle numbering is sequential.
    for (let i = 0; i < trace.length; i++) {
      expect(trace[i].cycle).toBe(i + 1);
    }
  });

  it("snapshot does not advance the clock", () => {
    const instrs = [makeInstruction(OP_ADD, 1, 2, 3)];
    const p = newTestPipeline(instrs, null);

    p.step();
    const snap1 = p.snapshot();
    const snap2 = p.snapshot();
    expect(snap1.cycle).toBe(snap2.cycle);
  });
});

// =========================================================================
// Deep pipeline tests
// =========================================================================

describe("Deep pipeline", () => {
  it("takes more cycles to produce first completion", () => {
    const config = deep13Stage();
    const instrs = Array(30).fill(makeInstruction(OP_ADD, 1, 2, 3));

    const p = Pipeline.create(
      config,
      simpleFetch(instrs),
      simpleDecode(),
      simpleExecute(),
      simpleMemory(),
      simpleWriteback(null),
    );

    // Run for 12 cycles -- no instruction should have completed yet.
    for (let i = 0; i < 12; i++) p.step();
    expect(p.stats().instructionsCompleted).toBe(0);

    // After cycle 13, exactly 1 instruction should have completed.
    p.step();
    expect(p.stats().instructionsCompleted).toBe(1);
  });
});

// =========================================================================
// Custom stage configuration tests
// =========================================================================

describe("Custom stage configuration", () => {
  it("3-stage pipeline completes at cycle 3", () => {
    const config: PipelineConfig = {
      stages: [
        { name: "IF", description: "Fetch", category: StageCategory.Fetch },
        { name: "EX", description: "Execute", category: StageCategory.Execute },
        { name: "WB", description: "Writeback", category: StageCategory.Writeback },
      ],
      executionWidth: 1,
    };

    const instrs = Array(10).fill(makeInstruction(OP_ADD, 1, 2, 3));
    const completed: CompletedInstructions = { pcs: [] };

    const p = Pipeline.create(
      config,
      simpleFetch(instrs),
      simpleDecode(),
      simpleExecute(),
      simpleMemory(),
      simpleWriteback(completed),
    );

    // In a 3-stage pipeline, first completion at cycle 3.
    for (let i = 0; i < 2; i++) p.step();
    expect(completed.pcs.length).toBe(0);

    p.step(); // cycle 3
    expect(completed.pcs.length).toBe(1);
  });
});

// =========================================================================
// Branch prediction integration tests
// =========================================================================

describe("Branch prediction", () => {
  it("uses predict callback to determine next PC", () => {
    const instrs = Array(100).fill(makeInstruction(OP_ADD, 1, 2, 3));
    const p = newTestPipeline(instrs, null);

    // Always predict PC+8 (skip one instruction).
    p.setPredictFunc((pc: number) => pc + 8);

    p.step(); // cycle 1: fetches PC=0, predicts next=8
    expect(p.pc()).toBe(8);

    p.step(); // cycle 2: fetches PC=8, predicts next=16
    expect(p.pc()).toBe(16);
  });
});

// =========================================================================
// SetPC tests
// =========================================================================

describe("SetPC", () => {
  it("sets and returns the PC", () => {
    const instrs = Array(10).fill(makeInstruction(OP_ADD, 1, 2, 3));
    const p = newTestPipeline(instrs, null);
    p.setPC(100);
    expect(p.pc()).toBe(100);
  });
});

// =========================================================================
// HazardAction and HazardResponse tests
// =========================================================================

describe("HazardAction", () => {
  it("noHazard() returns action None", () => {
    const h = noHazard();
    expect(h.action).toBe(HazardAction.None);
    expect(h.forwardValue).toBe(0);
    expect(h.stallStages).toBe(0);
    expect(h.flushCount).toBe(0);
  });
});

// =========================================================================
// Config accessor tests
// =========================================================================

describe("Pipeline accessors", () => {
  it("config() returns the pipeline configuration", () => {
    const instrs = [makeInstruction(OP_ADD, 1, 2, 3)];
    const p = newTestPipeline(instrs, null);
    const cfg = p.config();
    expect(cfg.stages.length).toBe(5);
    expect(cfg.executionWidth).toBe(1);
  });

  it("stageContents returns null for invalid stage name", () => {
    const instrs = [makeInstruction(OP_ADD, 1, 2, 3)];
    const p = newTestPipeline(instrs, null);
    expect(p.stageContents("NONEXISTENT")).toBeNull();
  });

  it("stageContents returns the token in the stage", () => {
    const instrs = Array(10).fill(makeInstruction(OP_ADD, 1, 2, 3));
    const p = newTestPipeline(instrs, null);
    p.step();

    const ifTok = p.stageContents("IF");
    expect(ifTok).not.toBeNull();
    expect(ifTok!.pc).toBe(0);
  });
});

// =========================================================================
// numStages tests
// =========================================================================

describe("numStages", () => {
  it("returns correct count", () => {
    expect(numStages(classic5Stage())).toBe(5);
    expect(numStages(deep13Stage())).toBe(13);
  });
});

// =========================================================================
// Run with halt
// =========================================================================

describe("Run with halt", () => {
  it("stops at halt before maxCycles", () => {
    // 2 ADDs then HALT
    const instrs = [
      makeInstruction(OP_ADD, 1, 2, 3),
      makeInstruction(OP_ADD, 4, 5, 6),
      makeInstruction(OP_HALT, 0, 0, 0),
    ];

    const p = newTestPipeline(instrs, null);
    const stats = p.run(1000);

    expect(p.isHalted()).toBe(true);
    // Pipeline should not have run all 1000 cycles.
    expect(stats.totalCycles).toBeLessThan(1000);
  });
});
