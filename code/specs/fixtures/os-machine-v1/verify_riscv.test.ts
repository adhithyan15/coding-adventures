import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { BOARD_CONFIGS } from "../../../packages/typescript/os-architecture-simulator/src/board.ts";
import { RISCV_PROGRAM_ID, RISCV_PROGRAM_WORDS, runRiscVBoard } from "../../../packages/typescript/os-architecture-simulator/src/riscv-adapter.ts";

test("RV32I board fixture matches repository simulator execution", async () => {
  const fixture = JSON.parse(await readFile(new URL("./riscv.json", import.meta.url), "utf8"));
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.program, RISCV_PROGRAM_ID);
  assert.equal(fixture.instructionSet, "RV32I");
  assert.deepEqual(fixture.instructions, RISCV_PROGRAM_WORDS);
  assert.deepEqual(fixture.expect.map(item => item.board), BOARD_CONFIGS.map(board => board.id));

  for (const expected of fixture.expect) {
    const board = BOARD_CONFIGS.find(candidate => candidate.id === expected.board);
    assert.ok(board, `unknown board ${expected.board}`);
    const snapshot = runRiscVBoard(board, fixture.budget, fixture.instructions);
    assert.equal(snapshot.program, fixture.program);
    assert.equal(snapshot.halted, expected.halted);
    assert.equal(snapshot.cycles, expected.cycles);
    assert.equal(snapshot.pc, expected.pc);
    assert.deepEqual(snapshot.consoleBytes, expected.consoleBytes);
    assert.equal(snapshot.consoleText, expected.consoleText);
    assert.deepEqual(snapshot.instructions, expected.instructions);
    for (const [register, value] of Object.entries(expected.registers)) assert.equal(snapshot.registers[register], value);
    assert.deepEqual(snapshot.deviceEffects.map(effect => Number(effect.data.address)), expected.deviceEffectAddresses);
    assert.deepEqual(snapshot.deviceEffects.map(effect => ({
      kind: effect.kind,
      byte: effect.data.byte,
      character: effect.data.character,
      address: effect.data.address,
      region: effect.data.region,
    })), expected.deviceEffects);
    assert.deepEqual(snapshot.programWords, fixture.instructions);
    assert.equal(snapshot.programBytes.length, fixture.instructions.length * 4);
  }

  assert.deepEqual(fixture.expect[0].consoleBytes, fixture.expect[1].consoleBytes);
  assert.notEqual(fixture.expect[0].deviceEffectAddresses[0], fixture.expect[1].deviceEffectAddresses[0]);
});
