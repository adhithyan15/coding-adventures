import { describe, expect, it } from "vitest";
import { parseBrainfuck } from "@coding-adventures/brainfuck";
import { compile, releaseConfig } from "@coding-adventures/brainfuck-ir-compiler";
import {
  IDGenerator,
  imm,
  IrOp,
  type IrInstruction,
  IrProgram,
  lbl,
  reg,
} from "@coding-adventures/compiler-ir";

import {
  inferFunctionSignaturesFromComments,
  IrToWasmCompiler,
  WasmLoweringError,
} from "../src/index.js";

function addInstruction(
  program: IrProgram,
  ids: IDGenerator,
  opcode: IrOp,
  operands: IrInstruction["operands"] = [],
): void {
  program.addInstruction({
    opcode,
    operands,
    id: ids.next(),
  });
}

describe("IrToWasmCompiler", () => {
  it("lowers brainfuck IR into a wasm module with memory and wasi imports", () => {
    const ast = parseBrainfuck(",.");
    const { program } = compile(ast, "echo.bf", releaseConfig());

    const module = new IrToWasmCompiler().compile(program);

    expect(module.memories).toHaveLength(1);
    expect(module.exports.some((entry) => entry.name === "memory")).toBe(true);
    expect(module.exports.some((entry) => entry.name === "_start")).toBe(true);
    expect(module.imports.map((entry) => entry.name)).toEqual(["fd_write", "fd_read"]);
  });

  it("infers exported signatures from function comments", () => {
    const program = new IrProgram("_start");
    program.addInstruction({
      opcode: IrOp.COMMENT,
      operands: [lbl("function: add(a: u4, b: u4)")],
      id: -1,
    });
    program.addInstruction({
      opcode: IrOp.LABEL,
      operands: [lbl("_fn_add")],
      id: -1,
    });
    program.addInstruction({
      opcode: IrOp.LABEL,
      operands: [lbl("_start")],
      id: -1,
    });

    const signatures = inferFunctionSignaturesFromComments(program);

    expect(signatures.get("_fn_add")).toEqual({
      label: "_fn_add",
      paramCount: 2,
      exportName: "add",
    });
    expect(signatures.get("_start")).toEqual({
      label: "_start",
      paramCount: 0,
      exportName: "_start",
    });
  });

  it("lowers arithmetic and comparison instructions into wasm bodies", () => {
    const ids = new IDGenerator();
    const program = new IrProgram("_fn_ops");
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_fn_ops")], id: -1 });
    addInstruction(program, ids, IrOp.LOAD_IMM, [reg(2), imm(7)]);
    addInstruction(program, ids, IrOp.LOAD_IMM, [reg(3), imm(3)]);
    addInstruction(program, ids, IrOp.ADD, [reg(4), reg(2), reg(3)]);
    addInstruction(program, ids, IrOp.SUB, [reg(5), reg(2), reg(3)]);
    addInstruction(program, ids, IrOp.AND, [reg(6), reg(2), reg(3)]);
    addInstruction(program, ids, IrOp.CMP_EQ, [reg(7), reg(2), reg(3)]);
    addInstruction(program, ids, IrOp.CMP_NE, [reg(8), reg(2), reg(3)]);
    addInstruction(program, ids, IrOp.CMP_LT, [reg(9), reg(3), reg(2)]);
    addInstruction(program, ids, IrOp.CMP_GT, [reg(10), reg(2), reg(3)]);
    addInstruction(program, ids, IrOp.NOP);
    addInstruction(program, ids, IrOp.ADD_IMM, [reg(1), reg(4), imm(0)]);
    addInstruction(program, ids, IrOp.RET);

    const module = new IrToWasmCompiler().compile(program, [
      { label: "_fn_ops", paramCount: 0, exportName: "ops" },
    ]);

    expect(module.imports).toHaveLength(0);
    expect(module.memories).toHaveLength(0);
    expect(module.exports.some((entry) => entry.name === "ops")).toBe(true);
    expect(module.code[0]?.locals.length).toBeGreaterThanOrEqual(11);
    expect(module.code[0]?.code.length).toBeGreaterThan(0);
  });

  it("lowers structured if and loop patterns", () => {
    const ids = new IDGenerator();
    const program = new IrProgram("_fn_choose");
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_fn_choose")], id: -1 });
    addInstruction(program, ids, IrOp.BRANCH_Z, [reg(2), lbl("if_0_else")]);
    addInstruction(program, ids, IrOp.LOAD_IMM, [reg(1), imm(10)]);
    addInstruction(program, ids, IrOp.JUMP, [lbl("if_0_end")]);
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("if_0_else")], id: -1 });
    addInstruction(program, ids, IrOp.LOAD_IMM, [reg(1), imm(20)]);
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("if_0_end")], id: -1 });
    addInstruction(program, ids, IrOp.LOAD_IMM, [reg(3), imm(0)]);
    addInstruction(program, ids, IrOp.LOAD_IMM, [reg(4), imm(0)]);
    addInstruction(program, ids, IrOp.ADD_IMM, [reg(5), reg(2), imm(0)]);
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("loop_0_start")], id: -1 });
    addInstruction(program, ids, IrOp.CMP_LT, [reg(6), reg(4), reg(5)]);
    addInstruction(program, ids, IrOp.BRANCH_Z, [reg(6), lbl("loop_0_end")]);
    addInstruction(program, ids, IrOp.ADD_IMM, [reg(3), reg(3), imm(1)]);
    addInstruction(program, ids, IrOp.ADD_IMM, [reg(4), reg(4), imm(1)]);
    addInstruction(program, ids, IrOp.JUMP, [lbl("loop_0_start")]);
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("loop_0_end")], id: -1 });
    addInstruction(program, ids, IrOp.RET);

    const module = new IrToWasmCompiler().compile(program, [
      { label: "_fn_choose", paramCount: 1, exportName: "choose" },
    ]);

    expect(module.functions).toEqual([0]);
    expect(module.exports.some((entry) => entry.name === "choose")).toBe(true);
    expect(module.code[0]?.code.length).toBeGreaterThan(0);
  });

  it("lays out data and lowers byte and word memory operations", () => {
    const ids = new IDGenerator();
    const program = new IrProgram("_fn_store_read");
    program.addData({ label: "buf", size: 8, init: 0 });
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_fn_store_read")], id: -1 });
    addInstruction(program, ids, IrOp.LOAD_ADDR, [reg(2), lbl("buf")]);
    addInstruction(program, ids, IrOp.LOAD_IMM, [reg(3), imm(0)]);
    addInstruction(program, ids, IrOp.LOAD_IMM, [reg(4), imm(90)]);
    addInstruction(program, ids, IrOp.STORE_BYTE, [reg(4), reg(2), reg(3)]);
    addInstruction(program, ids, IrOp.LOAD_BYTE, [reg(5), reg(2), reg(3)]);
    addInstruction(program, ids, IrOp.LOAD_IMM, [reg(6), imm(4)]);
    addInstruction(program, ids, IrOp.LOAD_IMM, [reg(7), imm(12345)]);
    addInstruction(program, ids, IrOp.STORE_WORD, [reg(7), reg(2), reg(6)]);
    addInstruction(program, ids, IrOp.LOAD_WORD, [reg(1), reg(2), reg(6)]);
    addInstruction(program, ids, IrOp.RET);

    const module = new IrToWasmCompiler().compile(program, [
      { label: "_fn_store_read", paramCount: 0, exportName: "store_read" },
    ]);

    expect(module.memories).toHaveLength(1);
    expect(module.data).toEqual([
      {
        memoryIndex: 0,
        offsetExpr: expect.any(Uint8Array),
        data: new Uint8Array(8),
      },
    ]);
    expect(module.exports.some((entry) => entry.name === "memory")).toBe(true);
    expect(module.exports.some((entry) => entry.name === "store_read")).toBe(true);
  });

  it("lowers calls between functions and preserves exported names", () => {
    const ids = new IDGenerator();
    const program = new IrProgram("_start");
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    addInstruction(program, ids, IrOp.LOAD_IMM, [reg(2), imm(5)]);
    addInstruction(program, ids, IrOp.CALL, [lbl("_fn_double")]);
    addInstruction(program, ids, IrOp.HALT);
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_fn_double")], id: -1 });
    addInstruction(program, ids, IrOp.ADD, [reg(1), reg(2), reg(2)]);
    addInstruction(program, ids, IrOp.RET);

    const module = new IrToWasmCompiler().compile(program, [
      { label: "_start", paramCount: 0, exportName: "_start" },
      { label: "_fn_double", paramCount: 1, exportName: "double" },
    ]);

    expect(module.functions).toHaveLength(2);
    expect(module.exports.map((entry) => entry.name)).toEqual(["_start", "double"]);
    expect(module.code).toHaveLength(2);
  });

  it("imports the right wasi functions for supported syscalls", () => {
    const ids = new IDGenerator();
    const program = new IrProgram("_start");
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    addInstruction(program, ids, IrOp.LOAD_IMM, [reg(4), imm(65)]);
    addInstruction(program, ids, IrOp.SYSCALL, [imm(1)]);
    addInstruction(program, ids, IrOp.SYSCALL, [imm(2)]);
    addInstruction(program, ids, IrOp.SYSCALL, [imm(10)]);

    const module = new IrToWasmCompiler().compile(program, [
      { label: "_start", paramCount: 0, exportName: "_start" },
    ]);

    expect(module.imports.map((entry) => entry.name)).toEqual([
      "fd_write",
      "fd_read",
      "proc_exit",
    ]);
    expect(module.memories).toHaveLength(1);
    expect(module.exports.some((entry) => entry.name === "_start")).toBe(true);
  });

  it("rejects unsupported syscalls", () => {
    const ids = new IDGenerator();
    const program = new IrProgram("_start");
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    addInstruction(program, ids, IrOp.SYSCALL, [imm(99)]);

    expect(() =>
      new IrToWasmCompiler().compile(program, [
        { label: "_start", paramCount: 0, exportName: "_start" },
      ]),
    ).toThrowError(new WasmLoweringError("unsupported SYSCALL number(s): 99"));
  });

  it("rejects functions without signatures", () => {
    const program = new IrProgram("_fn_add");
    program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_fn_add")], id: -1 });

    expect(() => new IrToWasmCompiler().compile(program)).toThrowError(
      new WasmLoweringError("missing function signature for _fn_add"),
    );
  });
});
