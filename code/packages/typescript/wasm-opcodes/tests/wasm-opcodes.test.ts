import assert from "node:assert/strict";
import { describe, it } from "node:test";
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
    assert.equal(VERSION, "0.1.0");
  });

  // ── Table completeness ───────────────────────────────────────────────────

  it("contains at least 172 opcodes (all WASM 1.0 instructions)", () => {
    // WASM 1.0 defines exactly 172 single-byte opcodes in the range 0x00–0xBF.
    // The bytes 0x06–0x0A, 0x12–0x19, 0x1C–0x1F, and 0x25–0x27 are reserved
    // (not allocated to any instruction in the 1.0 standard).
    assert.ok(OPCODES.size >= 172);
  });

  it("OPCODES and OPCODES_BY_NAME have the same size", () => {
    assert.equal(OPCODES.size, OPCODES_BY_NAME.size);
  });

  // ── Opcode byte lookup ───────────────────────────────────────────────────

  it("getOpcode(0x6A) returns i32.add info", () => {
    const info = getOpcode(0x6A);
    assert.notEqual(info, undefined);
    assert.equal(info!.name, "i32.add");
    assert.equal(info!.opcode, 0x6A);
    assert.equal(info!.category, "numeric_i32");
  });

  it("getOpcode returns undefined for unknown byte 0xFF", () => {
    assert.equal(getOpcode(0xFF), undefined);
  });

  it("getOpcode returns undefined for byte 0x06 (gap in control block)", () => {
    assert.equal(getOpcode(0x06), undefined);
  });

  // ── Name lookup ──────────────────────────────────────────────────────────

  it("getOpcodeByName('i32.add') returns correct info", () => {
    const info = getOpcodeByName("i32.add");
    assert.notEqual(info, undefined);
    assert.equal(info!.opcode, 0x6A);
    assert.equal(info!.stackPop, 2);
    assert.equal(info!.stackPush, 1);
  });

  it("getOpcodeByName returns undefined for unknown name", () => {
    assert.equal(getOpcodeByName("i32.foo"), undefined);
    assert.equal(getOpcodeByName(""), undefined);
  });

  // ── Stack effects ─────────────────────────────────────────────────────────

  it("i32.add has stackPop=2, stackPush=1", () => {
    const info = getOpcode(0x6A)!;
    assert.equal(info.stackPop, 2);
    assert.equal(info.stackPush, 1);
  });

  it("i32.const has stackPop=0, stackPush=1", () => {
    const info = getOpcodeByName("i32.const")!;
    assert.equal(info.stackPop, 0);
    assert.equal(info.stackPush, 1);
  });

  it("drop has stackPop=1, stackPush=0", () => {
    const info = getOpcodeByName("drop")!;
    assert.equal(info.stackPop, 1);
    assert.equal(info.stackPush, 0);
  });

  it("select has stackPop=3, stackPush=1", () => {
    const info = getOpcodeByName("select")!;
    assert.equal(info.stackPop, 3);
    assert.equal(info.stackPush, 1);
  });

  it("nop has stackPop=0, stackPush=0", () => {
    const info = getOpcodeByName("nop")!;
    assert.equal(info.stackPop, 0);
    assert.equal(info.stackPush, 0);
  });

  // ── Immediates ────────────────────────────────────────────────────────────

  it("i32.const has immediates=['i32']", () => {
    const info = getOpcodeByName("i32.const")!;
    assert.deepEqual(info.immediates, ["i32"]);
  });

  it("i64.const has immediates=['i64']", () => {
    const info = getOpcodeByName("i64.const")!;
    assert.deepEqual(info.immediates, ["i64"]);
  });

  it("f32.const has immediates=['f32']", () => {
    const info = getOpcodeByName("f32.const")!;
    assert.deepEqual(info.immediates, ["f32"]);
  });

  it("f64.const has immediates=['f64']", () => {
    const info = getOpcodeByName("f64.const")!;
    assert.deepEqual(info.immediates, ["f64"]);
  });

  it("i32.load has immediates=['memarg']", () => {
    const info = getOpcodeByName("i32.load")!;
    assert.deepEqual(info.immediates, ["memarg"]);
  });

  it("i32.store has immediates=['memarg']", () => {
    const info = getOpcodeByName("i32.store")!;
    assert.deepEqual(info.immediates, ["memarg"]);
  });

  it("block has immediates=['blocktype']", () => {
    const info = getOpcodeByName("block")!;
    assert.deepEqual(info.immediates, ["blocktype"]);
  });

  it("loop has immediates=['blocktype']", () => {
    const info = getOpcodeByName("loop")!;
    assert.deepEqual(info.immediates, ["blocktype"]);
  });

  it("if has immediates=['blocktype']", () => {
    const info = getOpcodeByName("if")!;
    assert.deepEqual(info.immediates, ["blocktype"]);
  });

  it("call_indirect has immediates=['typeidx','tableidx']", () => {
    const info = getOpcodeByName("call_indirect")!;
    assert.deepEqual(info.immediates, ["typeidx", "tableidx"]);
  });

  it("call has immediates=['funcidx']", () => {
    const info = getOpcodeByName("call")!;
    assert.deepEqual(info.immediates, ["funcidx"]);
  });

  it("memory.size has immediates=['memidx']", () => {
    const info = getOpcodeByName("memory.size")!;
    assert.deepEqual(info.immediates, ["memidx"]);
  });

  it("nop has no immediates", () => {
    const info = getOpcodeByName("nop")!;
    assert.deepEqual(info.immediates, []);
  });

  it("i32.add has no immediates", () => {
    const info = getOpcodeByName("i32.add")!;
    assert.deepEqual(info.immediates, []);
  });

  // ── Data integrity ────────────────────────────────────────────────────────

  it("all opcodes have a non-empty name", () => {
    for (const [, info] of OPCODES) {
      assert.ok(info.name.length > 0);
    }
  });

  it("all opcode byte values are unique", () => {
    const bytes = Array.from(OPCODES.keys());
    const unique = new Set(bytes);
    assert.equal(unique.size, bytes.length);
  });

  it("all opcode names are unique", () => {
    const names = Array.from(OPCODES_BY_NAME.keys());
    const unique = new Set(names);
    assert.equal(unique.size, names.length);
  });

  // ── Specific instruction checks ───────────────────────────────────────────

  it("unreachable is opcode 0x00 in control category", () => {
    const info = getOpcode(0x00)!;
    assert.equal(info.name, "unreachable");
    assert.equal(info.category, "control");
  });

  it("br_table has vec_labelidx immediate", () => {
    const info = getOpcodeByName("br_table")!;
    assert.deepEqual(info.immediates, ["vec_labelidx"]);
  });

  it("local.tee pops and pushes 1 (peek-and-store)", () => {
    const info = getOpcodeByName("local.tee")!;
    assert.equal(info.stackPop, 1);
    assert.equal(info.stackPush, 1);
  });

  it("memory.grow pops 1 and pushes 1", () => {
    const info = getOpcodeByName("memory.grow")!;
    assert.equal(info.stackPop, 1);
    assert.equal(info.stackPush, 1);
  });

  it("i32.store pops 2 (address + value) and pushes 0", () => {
    const info = getOpcodeByName("i32.store")!;
    assert.equal(info.stackPop, 2);
    assert.equal(info.stackPush, 0);
  });

  it("f64.reinterpret_i64 is a conversion instruction", () => {
    const info = getOpcode(0xBF)!;
    assert.equal(info.name, "f64.reinterpret_i64");
    assert.equal(info.category, "conversion");
    assert.deepEqual(info.immediates, []);
    assert.equal(info.stackPop, 1);
    assert.equal(info.stackPush, 1);
  });

  it("i64 numeric instructions have correct category", () => {
    const info = getOpcodeByName("i64.add")!;
    assert.equal(info.category, "numeric_i64");
  });

  it("f32 numeric instructions have correct category", () => {
    const info = getOpcodeByName("f32.sqrt")!;
    assert.equal(info.category, "numeric_f32");
  });

  it("f64 numeric instructions have correct category", () => {
    const info = getOpcodeByName("f64.sqrt")!;
    assert.equal(info.category, "numeric_f64");
  });

  // ── OPCODES map consistency ───────────────────────────────────────────────

  it("every entry in OPCODES is accessible via OPCODES_BY_NAME", () => {
    for (const [, info] of OPCODES) {
      const byName = OPCODES_BY_NAME.get(info.name);
      assert.notEqual(byName, undefined);
      assert.equal(byName!.opcode, info.opcode);
    }
  });

  it("getOpcode and OPCODES.get return the same object", () => {
    const a = getOpcode(0x6A);
    const b = OPCODES.get(0x6A);
    assert.equal(a, b);
  });
});
