import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { Machine } from "../../../packages/typescript/os-architecture-simulator/src/core.ts";
import { BatchMonitorProfile, ProtectedMessageSchedulerProfile } from "../../../packages/typescript/os-architecture-simulator/src/profiles.ts";

const fixture = JSON.parse(await readFile(new URL("./profiles.json", import.meta.url), "utf8"));
const profileValues = [new BatchMonitorProfile(), new ProtectedMessageSchedulerProfile()];
const profiles = new Map(profileValues.map(profile => [profile.id, profile]));
assert.equal(fixture.schemaVersion, 1);
for (const scenario of fixture.cases) {
  const profile = profiles.get(scenario.profile);
  assert.ok(profile, `unknown fixture profile ${scenario.profile}`);
  const machine = new Machine(profile);
  for (const stimulus of scenario.stimuli) machine.inject(stimulus);
  for (const budget of scenario.budgets) machine.run(budget);
  assert.equal(machine.tick, scenario.expectedTick);
  assert.deepEqual(machine.state, scenario.expectedState);
  assert.deepEqual(machine.events, scenario.expectedEffects);

  const snapshot = machine.serialize();
  const restored = Machine.restore(profile, snapshot);
  const replayed = Machine.replay(profile, machine.snapshot().history);
  assert.equal(restored.serialize(), snapshot);
  assert.equal(replayed.serialize(), snapshot);
  restored.run(3);
  replayed.run(3);
  assert.equal(restored.serialize(), replayed.serialize(), "restored and replayed runs continue identically");
}
console.log(`verified ${fixture.cases.length} shared semantic profile cases`);
