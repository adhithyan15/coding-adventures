import { expect, test } from "vitest";
import { BOARD_CONFIGS } from "../src/board.ts";
import { RISCV_PROGRAM_WORDS, runRiscVBoard, wordsToBytes } from "../src/riscv-adapter.ts";

test("the bounded RV32I probe has fixed instruction words and little-endian image", () => {
  expect(RISCV_PROGRAM_WORDS).toHaveLength(11);
  const bytes = wordsToBytes(RISCV_PROGRAM_WORDS);
  expect(bytes).toHaveLength(44);
  expect(Array.from(bytes.slice(0, 4))).toEqual([0x83, 0x40, 0, 0]);
});

test("RISC-V adapter honors instruction budgets without inventing completion", () => {
  const run = runRiscVBoard(BOARD_CONFIGS[0]!, 1);
  expect(run.halted).toBe(false);
  expect(run.cycles).toBe(1);
  expect(run.instructions).toEqual(["lbu"]);
  expect(run.consoleBytes).toEqual([]);
});

test("RISC-V adapter rejects invalid instruction budgets", () => {
  for (const budget of [-1, 0.5, Number.MAX_SAFE_INTEGER + 1, Number.NaN]) {
    expect(() => runRiscVBoard(BOARD_CONFIGS[0]!, budget)).toThrow(/nonnegative safe integer/);
  }
});
