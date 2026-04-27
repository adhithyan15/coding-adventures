import { IrOp } from "@coding-adventures/compiler-ir";
import { checkNib } from "@coding-adventures/nib-type-checker";
import { parseNib } from "@coding-adventures/nib-parser";
import { describe, expect, it } from "vitest";

import { compileNib, debugConfig, releaseConfig } from "../src/index.js";

function compileSource(source: string, useDebug = false) {
  const ast = parseNib(source);
  const checked = checkNib(ast);
  expect(checked.ok).toBe(true);
  return compileNib(checked.typedAst, useDebug ? debugConfig() : releaseConfig()).program;
}

describe("nib-ir-compiler", () => {
  it("emits the program prologue", () => {
    const program = compileSource("fn main() { }");
    expect(program.entryLabel).toBe("_start");
    expect(program.instructions.some((instruction) => instruction.opcode === IrOp.HALT)).toBe(true);
  });

  it("emits static data declarations", () => {
    const program = compileSource("static x: u4 = 7;");
    expect(program.data).toEqual([{ label: "x", size: 1, init: 7 }]);
  });

  it("compiles wrapping add with masking", () => {
    const program = compileSource("fn main() { let x: u4 = 1 +% 2; }");
    expect(program.instructions.some((instruction) => instruction.opcode === IrOp.ADD)).toBe(true);
    expect(
      program.instructions.some(
        (instruction) =>
          instruction.opcode === IrOp.AND_IMM &&
          instruction.operands[2] &&
          "value" in instruction.operands[2] &&
          instruction.operands[2].value === 15,
      ),
    ).toBe(true);
  });

  it("compiles function calls", () => {
    const program = compileSource("fn add(a: u4, b: u4) -> u4 { return a +% b; } fn main() { add(1, 2); }");
    expect(
      program.instructions.some(
        (instruction) =>
          instruction.opcode === IrOp.CALL &&
          instruction.operands[0] &&
          "name" in instruction.operands[0] &&
          instruction.operands[0].name === "_fn_add",
      ),
    ).toBe(true);
  });

  it("compiles for loops with labels and jumps", () => {
    const program = compileSource("fn main() { for i: u8 in 0..5 { } }");
    expect(program.instructions.some((instruction) => instruction.opcode === IrOp.BRANCH_Z)).toBe(true);
    expect(program.instructions.some((instruction) => instruction.opcode === IrOp.JUMP)).toBe(true);
  });

  it("respects debug comments", () => {
    const debugProgram = compileSource("fn main() { let x: u4 = 1; }", true);
    const releaseProgram = compileSource("fn main() { let x: u4 = 1; }", false);
    expect(debugProgram.instructions.some((instruction) => instruction.opcode === IrOp.COMMENT)).toBe(true);
    expect(releaseProgram.instructions.some((instruction) => instruction.opcode === IrOp.COMMENT)).toBe(false);
  });
});
