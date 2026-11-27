import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { BoardMachine } from "../../../packages/typescript/os-architecture-simulator/src/board-machine.ts";
import { BOARD_CONFIGS, CONSOLE_PROGRAM, CONSOLE_PROGRAM_ID } from "../../../packages/typescript/os-architecture-simulator/src/board.ts";

const fixture = JSON.parse(await readFile(new URL("./boards.json", import.meta.url), "utf8"));
assert.equal(fixture.schemaVersion, 1);
assert.equal(fixture.program, CONSOLE_PROGRAM_ID);
assert.deepEqual(fixture.programBytes, Array.from(CONSOLE_PROGRAM));
assert.deepEqual(fixture.cases.map(item => item.board), BOARD_CONFIGS);

const snapshots = [];
for (const scenario of fixture.cases) {
  const machine = new BoardMachine(scenario.board, new Uint8Array(fixture.programBytes), fixture.program);
  for (const budget of fixture.budgets) machine.run(budget);
  const snapshot = machine.snapshot();
  const instructions = snapshot.trace
    .filter(event => event.kind === "cpu.instruction")
    .map(event => event.data.mnemonic);
  const consoleWrites = snapshot.trace
    .filter(event => event.kind === "console.byte-written")
    .map(event => ({
      step: event.step,
      address: event.data.address,
      byte: event.data.byte,
      character: event.data.character,
      region: event.data.region,
    }));
  assert.equal(snapshot.program, fixture.program);
  assert.deepEqual(snapshot.cpu, scenario.expected.cpu);
  assert.deepEqual(snapshot.consoleBytes, scenario.expected.consoleBytes);
  assert.equal(snapshot.consoleText, scenario.expected.consoleText);
  assert.deepEqual(instructions, scenario.expected.instructions);
  assert.deepEqual(consoleWrites, scenario.expected.consoleWrites);
  assert.deepEqual(snapshot.budgets, fixture.budgets);

  const replayed = BoardMachine.replay(scenario.board, fixture.budgets, CONSOLE_PROGRAM, fixture.program);
  assert.equal(replayed.serialize(), machine.serialize());
  const restored = BoardMachine.restore(machine.serialize());
  assert.equal(restored.serialize(), machine.serialize());
  snapshots.push(snapshot);
}
assert.deepEqual(snapshots[0].consoleBytes, snapshots[1].consoleBytes);
assert.notEqual(snapshots[0].board.consoleAddress, snapshots[1].board.consoleAddress);
assert.notEqual(snapshots[0].board.ramEnd, snapshots[1].board.ramEnd);
