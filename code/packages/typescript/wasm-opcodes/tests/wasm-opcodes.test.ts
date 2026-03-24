import { describe, it, expect } from "vitest";
import {
  VERSION,
  OPCODES,
  OPCODES_BY_NAME,
  getOpcode,
  getOpcodeByName,
} from "../src/index.js";

describe("wasm-opcodes", () => {
  // ── Version ──────────────────────────────────────────────────────────────

  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });

  // ── Table completeness ───────────────────────────────────────────────────

  it("contains at least 172 opcodes (all WASM 1.0 instructions)", () => {
    // WASM 1.0 defines exactly 172 single-byte opcodes in the range 0x00–0xBF.
    // The bytes 0x06–0x0A, 0x12–0x19, 0x1C–0x1F, and 0x25–0x27 are reserved
    // (not allocated to any instruction in the 1.0 standard).
    expect(OPCODES.size).toBeGreaterThanOrEqual(172);
  });

  it("OPCODES and OPCODES_BY_NAME have the same size", () => {
    expect(OPCODES.size).toBe(OPCODES_BY_NAME.size);
  });

  // ── Opcode byte lookup ───────────────────────────────────────────────────

  it("getOpcode(0x6A) returns i32.add info", () => {
    const info = getOpcode(0x6A);
    expect(info).toBeDefined();
    expect(info!.name).toBe("i32.add");
    expect(info!.opcode).toBe(0x6A);
    expect(info!.category).toBe("numeric_i32");
  });

  it("getOpcode returns undefined for unknown byte 0xFF", () => {
    expect(getOpcode(0xFF)).toBeUndefined();
  });

  it("getOpcode returns undefined for byte 0x06 (gap in control block)", () => {
    expect(getOpcode(0x06)).toBeUndefined();
  });

  // ── Name lookup ──────────────────────────────────────────────────────────

  it("getOpcodeByName('i32.add') returns correct info", () => {
    const info = getOpcodeByName("i32.add");
    expect(info).toBeDefined();
    expect(info!.opcode).toBe(0x6A);
    expect(info!.stackPop).toBe(2);
    expect(info!.stackPush).toBe(1);
  });

  it("getOpcodeByName returns undefined for unknown name", () => {
    expect(getOpcodeByName("i32.foo")).toBeUndefined();
    expect(getOpcodeByName("")).toBeUndefined();
  });

  // ── Stack effects ─────────────────────────────────────────────────────────

  it("i32.add has stackPop=2, stackPush=1", () => {
    const info = getOpcode(0x6A)!;
    expect(info.stackPop).toBe(2);
    expect(info.stackPush).toBe(1);
  });

  it("i32.const has stackPop=0, stackPush=1", () => {
    const info = getOpcodeByName("i32.const")!;
    expect(info.stackPop).toBe(0);
    expect(info.stackPush).toBe(1);
  });

  it("drop has stackPop=1, stackPush=0", () => {
    const info = getOpcodeByName("drop")!;
    expect(info.stackPop).toBe(1);
    expect(info.stackPush).toBe(0);
  });

  it("select has stackPop=3, stackPush=1", () => {
    const info = getOpcodeByName("select")!;
    expect(info.stackPop).toBe(3);
    expect(info.stackPush).toBe(1);
  });

  it("nop has stackPop=0, stackPush=0", () => {
    const info = getOpcodeByName("nop")!;
    expect(info.stackPop).toBe(0);
    expect(info.stackPush).toBe(0);
  });

  // ── Immediates ────────────────────────────────────────────────────────────

  it("i32.const has immediates=['i32']", () => {
    const info = getOpcodeByName("i32.const")!;
    expect(info.immediates).toEqual(["i32"]);
  });

  it("i64.const has immediates=['i64']", () => {
    const info = getOpcodeByName("i64.const")!;
    expect(info.immediates).toEqual(["i64"]);
  });

  it("f32.const has immediates=['f32']", () => {
    const info = getOpcodeByName("f32.const")!;
    expect(info.immediates).toEqual(["f32"]);
  });

  it("f64.const has immediates=['f64']", () => {
    const info = getOpcodeByName("f64.const")!;
    expect(info.immediates).toEqual(["f64"]);
  });

  it("i32.load has immediates=['memarg']", () => {
    const info = getOpcodeByName("i32.load")!;
    expect(info.immediates).toEqual(["memarg"]);
  });

  it("i32.store has immediates=['memarg']", () => {
    const info = getOpcodeByName("i32.store")!;
    expect(info.immediates).toEqual(["memarg"]);
  });

  it("block has immediates=['blocktype']", () => {
    const info = getOpcodeByName("block")!;
    expect(info.immediates).toEqual(["blocktype"]);
  });

  it("loop has immediates=['blocktype']", () => {
    const info = getOpcodeByName("loop")!;
    expect(info.immediates).toEqual(["blocktype"]);
  });

  it("if has immediates=['blocktype']", () => {
    const info = getOpcodeByName("if")!;
    expect(info.immediates).toEqual(["blocktype"]);
  });

  it("call_indirect has immediates=['typeidx','tableidx']", () => {
    const info = getOpcodeByName("call_indirect")!;
    expect(info.immediates).toEqual(["typeidx", "tableidx"]);
  });

  it("call has immediates=['funcidx']", () => {
    const info = getOpcodeByName("call")!;
    expect(info.immediates).toEqual(["funcidx"]);
  });

  it("memory.size has immediates=['memidx']", () => {
    const info = getOpcodeByName("memory.size")!;
    expect(info.immediates).toEqual(["memidx"]);
  });

  it("nop has no immediates", () => {
    const info = getOpcodeByName("nop")!;
    expect(info.immediates).toEqual([]);
  });

  it("i32.add has no immediates", () => {
    const info = getOpcodeByName("i32.add")!;
    expect(info.immediates).toEqual([]);
  });

  // ── Data integrity ────────────────────────────────────────────────────────

  it("all opcodes have a non-empty name", () => {
    for (const [, info] of OPCODES) {
      expect(info.name.length).toBeGreaterThan(0);
    }
  });

  it("all opcode byte values are unique", () => {
    const bytes = Array.from(OPCODES.keys());
    const unique = new Set(bytes);
    expect(unique.size).toBe(bytes.length);
  });

  it("all opcode names are unique", () => {
    const names = Array.from(OPCODES_BY_NAME.keys());
    const unique = new Set(names);
    expect(unique.size).toBe(names.length);
  });

  // ── Specific instruction checks ───────────────────────────────────────────

  it("unreachable is opcode 0x00 in control category", () => {
    const info = getOpcode(0x00)!;
    expect(info.name).toBe("unreachable");
    expect(info.category).toBe("control");
  });

  it("br_table has vec_labelidx immediate", () => {
    const info = getOpcodeByName("br_table")!;
    expect(info.immediates).toEqual(["vec_labelidx"]);
  });

  it("local.tee pops and pushes 1 (peek-and-store)", () => {
    const info = getOpcodeByName("local.tee")!;
    expect(info.stackPop).toBe(1);
    expect(info.stackPush).toBe(1);
  });

  it("memory.grow pops 1 and pushes 1", () => {
    const info = getOpcodeByName("memory.grow")!;
    expect(info.stackPop).toBe(1);
    expect(info.stackPush).toBe(1);
  });

  it("i32.store pops 2 (address + value) and pushes 0", () => {
    const info = getOpcodeByName("i32.store")!;
    expect(info.stackPop).toBe(2);
    expect(info.stackPush).toBe(0);
  });

  it("f64.reinterpret_i64 is a conversion instruction", () => {
    const info = getOpcode(0xBF)!;
    expect(info.name).toBe("f64.reinterpret_i64");
    expect(info.category).toBe("conversion");
    expect(info.immediates).toEqual([]);
    expect(info.stackPop).toBe(1);
    expect(info.stackPush).toBe(1);
  });

  it("i64 numeric instructions have correct category", () => {
    const info = getOpcodeByName("i64.add")!;
    expect(info.category).toBe("numeric_i64");
  });

  it("f32 numeric instructions have correct category", () => {
    const info = getOpcodeByName("f32.sqrt")!;
    expect(info.category).toBe("numeric_f32");
  });

  it("f64 numeric instructions have correct category", () => {
    const info = getOpcodeByName("f64.sqrt")!;
    expect(info.category).toBe("numeric_f64");
  });

  // ── OPCODES map consistency ───────────────────────────────────────────────

  it("every entry in OPCODES is accessible via OPCODES_BY_NAME", () => {
    for (const [, info] of OPCODES) {
      const byName = OPCODES_BY_NAME.get(info.name);
      expect(byName).toBeDefined();
      expect(byName!.opcode).toBe(info.opcode);
    }
  });

  it("getOpcode and OPCODES.get return the same object", () => {
    const a = getOpcode(0x6A);
    const b = OPCODES.get(0x6A);
    expect(a).toBe(b);
  });
});
