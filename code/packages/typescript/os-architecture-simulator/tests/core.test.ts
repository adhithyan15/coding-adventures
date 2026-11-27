import assert from "node:assert/strict";
import test from "node:test";
import { Machine, type JsonRecord, type JsonValue, type Profile, type Stimulus } from "../src/core.ts";
import { BatchMonitorProfile, ProtectedMessageSchedulerProfile } from "../src/profiles.ts";

test("run budgets count transitions and reject invalid counts", () => {
  const machine = new Machine(new BatchMonitorProfile());
  assert.deepEqual(machine.run(0), []);
  assert.equal(machine.tick, 0);
  assert.throws(() => machine.run(-1), /nonnegative safe integer/);
  assert.throws(() => machine.run(1.5), /nonnegative safe integer/);
  machine.run(4);
  assert.equal(machine.tick, 4);
});

test("public step is one run(1) transition, including an automatic idle tick", () => {
  const profile: Profile = {
    id: "step-parity-profile",
    initialState: () => ({ observed: [] }),
    transition(state, tick, stimulus) {
      const previous = state as { observed: string[] };
      return {
        state: { observed: [...previous.observed, stimulus.kind] },
        effects: [{ kind: "stimulus.observed", data: { tick, kind: stimulus.kind } }],
      };
    },
  };
  const stepped = new Machine(profile);
  const runOne = new Machine(profile);
  const queued = { kind: "external.input", data: { count: 2 } };
  stepped.inject(queued);
  runOne.inject(queued);

  assert.deepEqual(stepped.step(), runOne.run(1));
  assert.equal(stepped.tick, 1);
  assert.deepEqual(stepped.snapshot(), runOne.snapshot());
  assert.deepEqual(stepped.step(), runOne.run(1));
  assert.equal(stepped.tick, 2);
  assert.deepEqual(stepped.events.at(-1)?.data, { tick: 2, kind: "clock.tick" });
  assert.deepEqual(stepped.snapshot(), runOne.snapshot());

  const idleStepped = new Machine(new BatchMonitorProfile());
  const idleRun = new Machine(new BatchMonitorProfile());
  assert.deepEqual(idleStepped.step(), idleRun.run(1));
  assert.deepEqual(idleStepped.events, []);
  assert.equal(idleStepped.tick, 1);
});

test("stimuli, state, and effects preserve nested finite JSON values deterministically", () => {
  const profile: Profile = {
    id: "nested-json-profile",
    initialState: () => ({ last: null }),
    transition(state, _tick, stimulus) {
      return {
        state: { ...(state as { last: JsonValue | null }), last: stimulus.data },
        effects: [{ kind: "input.received", data: { payload: stimulus.data } }],
      };
    },
  };
  const inputData = Object.assign(Object.create(null) as JsonRecord, {
    zeta: [true, null, { score: 1.25, name: "worker" }],
    count: 3,
    active: false,
    signedZero: -0,
  });
  const machine = new Machine(profile);
  machine.inject({ kind: "payload", data: inputData });
  const [event] = machine.step();

  assert.deepEqual(event?.data.payload, { active: false, count: 3, signedZero: 0, zeta: [true, null, { name: "worker", score: 1.25 }] });
  assert.equal(Object.is((event?.data.payload as JsonRecord).signedZero, -0), false);
  assert.deepEqual(machine.state, { last: { active: false, count: 3, signedZero: 0, zeta: [true, null, { name: "worker", score: 1.25 }] } });
  const encoded = machine.serialize();
  assert.equal(Machine.restore(profile, encoded).serialize(), encoded);
  assert.equal(Machine.replay(profile, machine.snapshot().history).serialize(), encoded);

  const specialKeys = Object.create(null) as JsonRecord;
  specialKeys["__proto__"] = { safe: true };
  const special = new Machine(profile);
  special.inject({ kind: "special-key", data: specialKeys });
  const payload = special.step()[0]?.data.payload as JsonRecord;
  assert.equal(Object.hasOwn(payload, "__proto__"), true);
  assert.deepEqual(payload["__proto__"], { safe: true });
  assert.equal(Object.getPrototypeOf(payload), Object.prototype);
});

test("synthetic policies accept typed numbers and structured IPC bodies", () => {
  const batch = new Machine(new BatchMonitorProfile());
  batch.inject({ kind: "job.submit", data: { id: "numeric-service", serviceTicks: 1 } });
  assert.deepEqual(batch.step().map(event => event.kind), ["job.accepted"]);
  assert.deepEqual(batch.step().map(event => event.kind), ["job.started", "job.serviced", "job.completed"]);

  const modern = new Machine(new ProtectedMessageSchedulerProfile());
  const body = { request: "lookup", arguments: ["alpha", 4, null] };
  modern.inject({ kind: "task.create", data: { id: "worker", quantum: 1 } });
  modern.inject({ kind: "message.send", data: { to: "worker", capability: "cap:worker", body } });
  modern.run(3);
  assert.deepEqual(modern.events.map(event => event.kind), [
    "task.created", "ipc.enqueued", "scheduler.dispatched", "ipc.delivered", "scheduler.preempted",
  ]);
  assert.deepEqual(modern.events.find(event => event.kind === "ipc.delivered")?.data.body, body);
});

test("unsupported or oversized values are rejected without queuing stimuli", () => {
  const machine = new Machine(new BatchMonitorProfile());
  const before = machine.serialize();
  const reject = (data: unknown, message: RegExp) => {
    assert.throws(() => machine.inject({ kind: "invalid", data: data as JsonRecord }), message);
    assert.equal(machine.serialize(), before);
  };
  reject({ value: Number.NaN }, /finite/);
  reject({ value: new Map() }, /plain JSON objects/);
  reject({ value: new Date(0) }, /plain JSON objects/);
  reject({ value: undefined }, /only JSON values/);
  reject({ value: 1n }, /only JSON values/);
  reject({ value: Symbol("unsupported") }, /only JSON values/);
  reject({ value: { [Symbol("key")]: "unsupported" } }, /symbol keys/);
  reject({ value: new Array(1) }, /dense data arrays/);
  const extraArray = [1] as unknown[] & { extra?: string };
  extraArray.extra = "extra";
  reject({ value: extraArray }, /extra properties/);
  reject({ value: "x".repeat(1_000_001) }, /maximum length/);
  const maximumString = "x".repeat(1_000_000);
  reject({ value: Array.from({ length: 11 }, () => maximumString) }, /maximum JSON text/);

  let deep: unknown = "leaf";
  for (let level = 0; level < 65; level++) deep = [deep];
  reject({ value: deep }, /maximum JSON depth/);
  reject({ value: Array.from({ length: 100_001 }, () => null) }, /maximum JSON node count/);

  let getterCalls = 0;
  const accessorData = Object.create(null) as Record<string, unknown>;
  Object.defineProperty(accessorData, "value", {
    enumerable: true,
    get() { getterCalls++; return "must not execute"; },
  });
  reject(accessorData, /without accessors/);
  assert.equal(getterCalls, 0);
});

test("invalid profile output cannot partially commit a transition", () => {
  const broken: Profile = {
    id: "broken-test-profile",
    initialState: () => ({ counter: 0 }),
    transition(state) {
      (state as { counter: number }).counter = 99;
      return { state, effects: [{ kind: "bad", data: { forbidden: new Map() } as unknown as JsonRecord }] };
    },
  };
  const machine = new Machine(broken);
  const before = machine.serialize();
  assert.throws(() => machine.run(1), /plain JSON objects/);
  assert.equal(machine.tick, 0);
  assert.equal(machine.serialize(), before);
});

test("snapshot restore rejects a profile mismatch", () => {
  const source = new Machine(new BatchMonitorProfile());
  assert.throws(() => Machine.restore(new ProtectedMessageSchedulerProfile(), source.serialize()), /profile mismatch/);
});

test("snapshot preserves pending stimuli and rejects forged state", () => {
  const profile = new BatchMonitorProfile();
  const source = new Machine(profile);
  source.inject({ kind: "job.submit", data: { id: "pending", serviceTicks: "1" } });
  const restored = Machine.restore(profile, source.serialize());
  source.run(1);
  restored.run(1);
  assert.equal(restored.serialize(), source.serialize());

  const forged = JSON.parse(source.serialize()) as { state: unknown };
  forged.state = { queued: [], active: null, completed: ["forged"] };
  assert.throws(() => Machine.restore(profile, JSON.stringify(forged)), /does not match replayed history/);
});

test("profile policies remain distinct while sharing the same engine interface", () => {
  const batch = new Machine(new BatchMonitorProfile());
  const modern = new Machine(new ProtectedMessageSchedulerProfile());
  assert.notEqual(batch.profile.id, modern.profile.id);
  batch.inject({ kind: "job.submit", data: { id: "job", serviceTicks: "1" } });
  modern.inject({ kind: "task.create", data: { id: "task", quantum: "1" } });
  batch.run(1);
  modern.run(1);
  assert.deepEqual(batch.events.map(event => event.kind), ["job.accepted"]);
  assert.deepEqual(modern.events.map(event => event.kind), ["task.created"]);
  assert.equal(JSON.stringify(batch.state).includes("tasks"), false);
  assert.equal(JSON.stringify(modern.state).includes("queued"), false);
});
