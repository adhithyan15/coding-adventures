/**
 * Comprehensive tests for the Core package.
 *
 * Covers: config, register file, decoder, memory controller,
 * interrupt controller, core assembly, core execution, stats,
 * multi-core, and preset configurations.
 */

import { describe, expect, it } from "vitest";
import {
  Core,
  CoreStats,
  InterruptController,
  MemoryController,
  MockDecoder,
  MultiCoreCPU,
  RegisterFile,
  cortexA78LikeConfig,
  defaultCoreConfig,
  defaultMultiCoreConfig,
  defaultRegisterFileConfig,
  encodeADD,
  encodeADDI,
  encodeBRANCH,
  encodeHALT,
  encodeLOAD,
  encodeNOP,
  encodeProgram,
  encodeSTORE,
  encodeSUB,
  simpleConfig,
} from "../src/index.js";

// =========================================================================
// Config tests
// =========================================================================

describe("Config", () => {
  it("defaultCoreConfig returns valid config", () => {
    const c = defaultCoreConfig();
    expect(c.name).toBe("Default");
    expect(c.pipeline.stages.length).toBe(5);
    expect(c.hazardDetection).toBe(true);
  });

  it("simpleConfig returns valid config", () => {
    const c = simpleConfig();
    expect(c.name).toBe("Simple");
    expect(c.registerFile).not.toBeNull();
    expect(c.registerFile!.count).toBe(16);
  });

  it("cortexA78LikeConfig returns 13-stage pipeline", () => {
    const c = cortexA78LikeConfig();
    expect(c.name).toBe("CortexA78Like");
    expect(c.pipeline.stages.length).toBe(13);
    expect(c.fpUnit).not.toBeNull();
  });

  it("defaultRegisterFileConfig returns sensible defaults", () => {
    const c = defaultRegisterFileConfig();
    expect(c.count).toBe(16);
    expect(c.width).toBe(32);
    expect(c.zeroRegister).toBe(true);
  });

  it("defaultMultiCoreConfig returns 2 cores", () => {
    const c = defaultMultiCoreConfig();
    expect(c.numCores).toBe(2);
  });
});

// =========================================================================
// RegisterFile tests
// =========================================================================

describe("RegisterFile", () => {
  it("creates with default config when null", () => {
    const rf = new RegisterFile(null);
    expect(rf.count()).toBe(16);
    expect(rf.width()).toBe(32);
  });

  it("creates with custom config", () => {
    const rf = new RegisterFile({ count: 32, width: 64, zeroRegister: false });
    expect(rf.count()).toBe(32);
    expect(rf.width()).toBe(64);
  });

  it("reads and writes registers", () => {
    const rf = new RegisterFile({ count: 16, width: 32, zeroRegister: true });
    rf.write(1, 42);
    expect(rf.read(1)).toBe(42);
  });

  it("zero register always returns 0", () => {
    const rf = new RegisterFile({ count: 16, width: 32, zeroRegister: true });
    rf.write(0, 999);
    expect(rf.read(0)).toBe(0);
  });

  it("returns 0 for out-of-range reads", () => {
    const rf = new RegisterFile({ count: 16, width: 32, zeroRegister: false });
    expect(rf.read(-1)).toBe(0);
    expect(rf.read(100)).toBe(0);
  });

  it("ignores out-of-range writes", () => {
    const rf = new RegisterFile({ count: 16, width: 32, zeroRegister: false });
    rf.write(-1, 42); // should not throw
    rf.write(100, 42); // should not throw
  });

  it("masks values to register width", () => {
    const rf = new RegisterFile({ count: 16, width: 8, zeroRegister: false });
    rf.write(1, 0x1ff); // 9 bits
    expect(rf.read(1)).toBe(0xff); // only 8 bits
  });

  it("values returns a copy", () => {
    const rf = new RegisterFile({ count: 4, width: 32, zeroRegister: false });
    rf.write(1, 10);
    rf.write(2, 20);
    const vals = rf.values();
    expect(vals).toEqual([0, 10, 20, 0]);
    // Modifying the copy should not affect the register file.
    vals[1] = 999;
    expect(rf.read(1)).toBe(10);
  });

  it("reset zeros all registers", () => {
    const rf = new RegisterFile({ count: 4, width: 32, zeroRegister: false });
    rf.write(1, 42);
    rf.reset();
    expect(rf.read(1)).toBe(0);
  });

  it("toString produces readable output", () => {
    const rf = new RegisterFile({ count: 4, width: 32, zeroRegister: false });
    rf.write(1, 42);
    const s = rf.toString();
    expect(s).toContain("R1=42");
  });
});

// =========================================================================
// MockDecoder tests
// =========================================================================

describe("MockDecoder", () => {
  it("instructionSize returns 4", () => {
    const d = new MockDecoder();
    expect(d.instructionSize()).toBe(4);
  });

  it("encodes and decodes NOP", () => {
    const d = new MockDecoder();
    const raw = encodeNOP();
    const tok = makeDummyToken();
    d.decode(raw, tok);
    expect(tok.opcode).toBe("NOP");
    expect(tok.isHalt).toBe(false);
  });

  it("encodes and decodes ADD", () => {
    const d = new MockDecoder();
    const raw = encodeADD(1, 2, 3);
    const tok = makeDummyToken();
    d.decode(raw, tok);
    expect(tok.opcode).toBe("ADD");
    expect(tok.rd).toBe(1);
    expect(tok.rs1).toBe(2);
    expect(tok.rs2).toBe(3);
    expect(tok.regWrite).toBe(true);
  });

  it("encodes and decodes HALT", () => {
    const d = new MockDecoder();
    const raw = encodeHALT();
    const tok = makeDummyToken();
    d.decode(raw, tok);
    expect(tok.opcode).toBe("HALT");
    expect(tok.isHalt).toBe(true);
  });

  it("encodes and decodes ADDI", () => {
    const d = new MockDecoder();
    const raw = encodeADDI(1, 0, 42);
    const tok = makeDummyToken();
    d.decode(raw, tok);
    expect(tok.opcode).toBe("ADDI");
    expect(tok.rd).toBe(1);
    expect(tok.immediate).toBe(42);
    expect(tok.regWrite).toBe(true);
  });

  it("encodes and decodes LOAD", () => {
    const d = new MockDecoder();
    const raw = encodeLOAD(1, 2, 8);
    const tok = makeDummyToken();
    d.decode(raw, tok);
    expect(tok.opcode).toBe("LOAD");
    expect(tok.memRead).toBe(true);
    expect(tok.regWrite).toBe(true);
  });

  it("encodes and decodes STORE", () => {
    const d = new MockDecoder();
    const raw = encodeSTORE(2, 3, 8);
    const tok = makeDummyToken();
    d.decode(raw, tok);
    expect(tok.opcode).toBe("STORE");
    expect(tok.memWrite).toBe(true);
  });

  it("encodes and decodes BRANCH", () => {
    const d = new MockDecoder();
    const raw = encodeBRANCH(1, 2, 4);
    const tok = makeDummyToken();
    d.decode(raw, tok);
    expect(tok.opcode).toBe("BRANCH");
    expect(tok.isBranch).toBe(true);
  });

  it("encodes and decodes SUB", () => {
    const d = new MockDecoder();
    const raw = encodeSUB(1, 2, 3);
    const tok = makeDummyToken();
    d.decode(raw, tok);
    expect(tok.opcode).toBe("SUB");
    expect(tok.regWrite).toBe(true);
  });

  it("executes ADD correctly", () => {
    const d = new MockDecoder();
    const rf = new RegisterFile({ count: 16, width: 32, zeroRegister: true });
    rf.write(2, 10);
    rf.write(3, 20);

    const tok = makeDummyToken();
    d.decode(encodeADD(1, 2, 3), tok);
    d.execute(tok, rf);
    expect(tok.aluResult).toBe(30);
    expect(tok.writeData).toBe(30);
  });

  it("executes ADDI correctly", () => {
    const d = new MockDecoder();
    const rf = new RegisterFile({ count: 16, width: 32, zeroRegister: true });
    rf.write(2, 10);

    const tok = makeDummyToken();
    d.decode(encodeADDI(1, 2, 5), tok);
    d.execute(tok, rf);
    expect(tok.aluResult).toBe(15);
    expect(tok.writeData).toBe(15);
  });

  it("executes SUB correctly", () => {
    const d = new MockDecoder();
    const rf = new RegisterFile({ count: 16, width: 32, zeroRegister: true });
    rf.write(2, 30);
    rf.write(3, 10);

    const tok = makeDummyToken();
    d.decode(encodeSUB(1, 2, 3), tok);
    d.execute(tok, rf);
    expect(tok.aluResult).toBe(20);
  });

  it("executes BRANCH (taken)", () => {
    const d = new MockDecoder();
    const rf = new RegisterFile({ count: 16, width: 32, zeroRegister: true });
    // Both R1 and R2 are 0 (zero register), so branch is taken.
    const tok = makeDummyToken();
    tok.pc = 0;
    d.decode(encodeBRANCH(0, 0, 4), tok);
    d.execute(tok, rf);
    expect(tok.branchTaken).toBe(true);
    expect(tok.branchTarget).toBe(16); // 0 + 4*4
  });

  it("encodeProgram produces correct bytes", () => {
    const bytes = encodeProgram(encodeADDI(1, 0, 42), encodeHALT());
    expect(bytes.length).toBe(8);
    // Verify it's little-endian.
    const word0 = bytes[0] | (bytes[1] << 8) | (bytes[2] << 16) | (bytes[3] << 24);
    expect(word0).toBe(encodeADDI(1, 0, 42));
  });
});

// =========================================================================
// MemoryController tests
// =========================================================================

describe("MemoryController", () => {
  it("readWord and writeWord work correctly", () => {
    const mem = new Uint8Array(1024);
    const mc = new MemoryController(mem, 1);
    mc.writeWord(0, 0x12345678);
    expect(mc.readWord(0)).toBe(0x12345678);
  });

  it("returns 0 for out-of-bounds read", () => {
    const mem = new Uint8Array(4);
    const mc = new MemoryController(mem, 1);
    expect(mc.readWord(4)).toBe(0);
  });

  it("loadProgram copies bytes", () => {
    const mem = new Uint8Array(1024);
    const mc = new MemoryController(mem, 1);
    const prog = new Uint8Array([1, 2, 3, 4]);
    mc.loadProgram(prog, 0);
    expect(mem[0]).toBe(1);
    expect(mem[3]).toBe(4);
  });

  it("tick processes pending requests", () => {
    const mem = new Uint8Array(1024);
    const mc = new MemoryController(mem, 2);
    mem[0] = 0xab;
    mc.requestRead(0, 1, 0);
    expect(mc.pendingCount()).toBe(1);

    mc.tick(); // 1 cycle left
    mc.tick(); // completes
    expect(mc.pendingCount()).toBe(0);
  });

  it("memorySize returns correct size", () => {
    const mem = new Uint8Array(4096);
    const mc = new MemoryController(mem, 1);
    expect(mc.memorySize()).toBe(4096);
  });
});

// =========================================================================
// InterruptController tests
// =========================================================================

describe("InterruptController", () => {
  it("raises and acknowledges interrupts", () => {
    const ic = new InterruptController(4);
    ic.raiseInterrupt(0, 1);
    expect(ic.pendingCount()).toBe(1);

    const pending = ic.pendingForCore(1);
    expect(pending.length).toBe(1);
    expect(pending[0].interruptID).toBe(0);

    ic.acknowledge(1, 0);
    expect(ic.pendingCount()).toBe(0);
    expect(ic.acknowledgedCount()).toBe(1);
  });

  it("routes -1 target to core 0", () => {
    const ic = new InterruptController(4);
    ic.raiseInterrupt(5, -1);
    const pending = ic.pendingForCore(0);
    expect(pending.length).toBe(1);
  });

  it("clamps out-of-range target to core 0", () => {
    const ic = new InterruptController(2);
    ic.raiseInterrupt(5, 10);
    const pending = ic.pendingForCore(0);
    expect(pending.length).toBe(1);
  });

  it("reset clears all state", () => {
    const ic = new InterruptController(2);
    ic.raiseInterrupt(0, 0);
    ic.acknowledge(0, 0);
    ic.reset();
    expect(ic.pendingCount()).toBe(0);
    expect(ic.acknowledgedCount()).toBe(0);
  });
});

// =========================================================================
// Core assembly tests
// =========================================================================

describe("Core assembly", () => {
  it("creates a core with SimpleConfig", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    expect(c.isHalted()).toBe(false);
    expect(c.cycle()).toBe(0);
  });

  it("creates a core with CortexA78LikeConfig", () => {
    const c = Core.create(cortexA78LikeConfig(), new MockDecoder());
    expect(c.isHalted()).toBe(false);
  });

  it("creates a core with default config", () => {
    const c = Core.create(defaultCoreConfig(), new MockDecoder());
    expect(c.isHalted()).toBe(false);
  });
});

// =========================================================================
// Core execution tests
// =========================================================================

describe("Core execution", () => {
  it("runs HALT-only program", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    const program = encodeProgram(encodeHALT());
    c.loadProgram(program, 0);
    const stats = c.run(100);
    expect(c.isHalted()).toBe(true);
    expect(stats.totalCycles).toBeGreaterThan(0);
  });

  it("runs ADDI + HALT and sets register", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    const program = encodeProgram(encodeADDI(1, 0, 42), encodeHALT());
    c.loadProgram(program, 0);
    c.run(100);
    expect(c.isHalted()).toBe(true);
    expect(c.readRegister(1)).toBe(42);
  });

  it("runs ADD program", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    // R1 = 10, R2 = 20, R3 = R1 + R2
    // NOPs inserted to avoid RAW hazards -- values must be written back
    // before they can be read by the ADD instruction.
    const program = encodeProgram(
      encodeADDI(1, 0, 10),
      encodeNOP(),
      encodeNOP(),
      encodeNOP(),
      encodeNOP(),
      encodeADDI(2, 0, 20),
      encodeNOP(),
      encodeNOP(),
      encodeNOP(),
      encodeNOP(),
      encodeADD(3, 1, 2),
      encodeHALT(),
    );
    c.loadProgram(program, 0);
    c.run(200);
    expect(c.isHalted()).toBe(true);
    expect(c.readRegister(3)).toBe(30);
  });

  it("runs SUB program", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    // NOPs to avoid RAW hazards.
    const program = encodeProgram(
      encodeADDI(1, 0, 30),
      encodeNOP(),
      encodeNOP(),
      encodeNOP(),
      encodeNOP(),
      encodeADDI(2, 0, 10),
      encodeNOP(),
      encodeNOP(),
      encodeNOP(),
      encodeNOP(),
      encodeSUB(3, 1, 2),
      encodeHALT(),
    );
    c.loadProgram(program, 0);
    c.run(200);
    expect(c.isHalted()).toBe(true);
    expect(c.readRegister(3)).toBe(20);
  });

  it("CortexA78Like runs HALT program", () => {
    const c = Core.create(cortexA78LikeConfig(), new MockDecoder());
    const program = encodeProgram(encodeHALT());
    c.loadProgram(program, 0);
    const stats = c.run(200);
    expect(stats.totalCycles).toBeGreaterThan(0);
  });

  it("step returns snapshots", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    const program = encodeProgram(encodeADDI(1, 0, 42), encodeHALT());
    c.loadProgram(program, 0);

    const snap = c.step();
    expect(snap.cycle).toBe(1);
    expect(c.cycle()).toBe(1);
  });

  it("does not advance after halt", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    const program = encodeProgram(encodeHALT());
    c.loadProgram(program, 0);
    c.run(100);
    expect(c.isHalted()).toBe(true);
    const cycleAfterHalt = c.cycle();
    c.step();
    expect(c.cycle()).toBe(cycleAfterHalt);
  });
});

// =========================================================================
// Core accessor tests
// =========================================================================

describe("Core accessors", () => {
  it("registerFile returns the register file", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    expect(c.registerFile()).toBeDefined();
    expect(c.registerFile().count()).toBe(16);
  });

  it("memoryController returns the memory controller", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    expect(c.memoryController()).toBeDefined();
  });

  it("pipeline returns the pipeline", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    expect(c.pipeline()).toBeDefined();
  });

  it("predictor returns the predictor", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    expect(c.predictor()).toBeDefined();
  });

  it("cacheHierarchy returns the hierarchy", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    expect(c.cacheHierarchy()).toBeDefined();
  });

  it("getConfig returns the config", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    expect(c.getConfig().name).toBe("Simple");
  });

  it("writeRegister and readRegister work", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    c.writeRegister(5, 123);
    expect(c.readRegister(5)).toBe(123);
  });
});

// =========================================================================
// CoreStats tests
// =========================================================================

describe("CoreStats", () => {
  it("ipc and cpi return correct values", () => {
    const s = new CoreStats();
    s.totalCycles = 100;
    s.instructionsCompleted = 80;
    expect(Math.abs(s.ipc() - 0.8)).toBeLessThan(0.001);
    expect(Math.abs(s.cpi() - 1.25)).toBeLessThan(0.001);
  });

  it("returns 0 for empty stats", () => {
    const s = new CoreStats();
    expect(s.ipc()).toBe(0);
    expect(s.cpi()).toBe(0);
  });

  it("toString produces non-empty string", () => {
    const s = new CoreStats();
    s.totalCycles = 10;
    s.instructionsCompleted = 5;
    expect(s.toString()).toContain("10");
  });
});

// =========================================================================
// MultiCoreCPU tests
// =========================================================================

describe("MultiCoreCPU", () => {
  it("creates a multi-core CPU", () => {
    const config = defaultMultiCoreConfig();
    const mc = MultiCoreCPU.create(config, [new MockDecoder(), new MockDecoder()]);
    expect(mc.cores().length).toBe(2);
    expect(mc.allHalted()).toBe(false);
  });

  it("runs both cores until halt", () => {
    const config = defaultMultiCoreConfig();
    const mc = MultiCoreCPU.create(config, [new MockDecoder(), new MockDecoder()]);

    // Load same program on both cores at different addresses.
    const program = encodeProgram(encodeADDI(1, 0, 99), encodeHALT());
    mc.loadProgram(0, program, 0);
    mc.loadProgram(1, program, 0x1000);

    const stats = mc.run(200);
    expect(stats.length).toBe(2);
    expect(mc.allHalted()).toBe(true);
  });

  it("step returns snapshots for all cores", () => {
    const config = defaultMultiCoreConfig();
    const mc = MultiCoreCPU.create(config, [new MockDecoder(), new MockDecoder()]);
    const program = encodeProgram(encodeHALT());
    mc.loadProgram(0, program, 0);
    mc.loadProgram(1, program, 0x1000);

    const snaps = mc.step();
    expect(snaps.length).toBe(2);
    expect(mc.cycle()).toBe(1);
  });

  it("interruptController returns controller", () => {
    const config = defaultMultiCoreConfig();
    const mc = MultiCoreCPU.create(config, [new MockDecoder()]);
    expect(mc.interruptController()).toBeDefined();
  });

  it("sharedMemoryController returns controller", () => {
    const config = defaultMultiCoreConfig();
    const mc = MultiCoreCPU.create(config, [new MockDecoder()]);
    expect(mc.sharedMemoryController()).toBeDefined();
  });
});

// =========================================================================
// Store and Load integration test
// =========================================================================

describe("Store and Load", () => {
  it("stores and loads a value through memory", () => {
    const c = Core.create(simpleConfig(), new MockDecoder());
    // R1 = 77, STORE R1 at address 100, LOAD from 100 into R2, HALT
    const program = encodeProgram(
      encodeADDI(1, 0, 77),
      encodeADDI(4, 0, 100), // address base in R4 (won't use, we use R0+imm)
      encodeSTORE(0, 1, 100), // store R1 at [R0 + 100] = [0 + 100] = addr 100
      encodeNOP(),
      encodeNOP(),
      encodeLOAD(2, 0, 100), // load from [R0 + 100] into R2
      encodeHALT(),
    );
    c.loadProgram(program, 0);
    c.run(200);
    expect(c.isHalted()).toBe(true);
    expect(c.readRegister(2)).toBe(77);
  });
});

// =========================================================================
// Missing optional components test
// =========================================================================

describe("Missing optional components", () => {
  it("core works without L2 cache and FP unit", () => {
    const config = simpleConfig();
    config.l2Cache = null;
    config.fpUnit = null;

    const c = Core.create(config, new MockDecoder());
    const program = encodeProgram(encodeHALT());
    c.loadProgram(program, 0);
    c.run(100);
    expect(c.isHalted()).toBe(true);
  });
});

// =========================================================================
// Helper: create a dummy PipelineToken-like object for decode tests
// =========================================================================

function makeDummyToken() {
  return {
    pc: 0,
    rawInstruction: 0,
    opcode: "",
    rs1: -1,
    rs2: -1,
    rd: -1,
    immediate: 0,
    regWrite: false,
    memRead: false,
    memWrite: false,
    isBranch: false,
    isHalt: false,
    aluResult: 0,
    memData: 0,
    writeData: 0,
    branchTaken: false,
    branchTarget: 0,
    isBubble: false,
    stageEntered: {} as Record<string, number>,
    forwardedFrom: "",
  };
}
