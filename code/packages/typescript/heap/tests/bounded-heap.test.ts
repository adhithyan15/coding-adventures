import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  BoundedMaxHeap,
  BoundedMinHeap,
  type BoundedPriorityQueue,
  type PushResult,
  type ValueResult,
} from "../src/index.js";

type Direction = "min" | "max";
type Command =
  | { kind: "push"; value: number }
  | { kind: "pop" }
  | { kind: "peek" };

type QueueFactory = (capacity: number) => BoundedPriorityQueue<number>;

class WrongRemovalMinQueue implements BoundedPriorityQueue<number> {
  readonly values: number[] = [];
  constructor(readonly capacity: number) {}
  get size(): number { return this.values.length; }
  isEmpty(): boolean { return this.values.length === 0; }
  isFull(): boolean { return this.values.length === this.capacity; }
  tryPush(value: number): PushResult<number> {
    this.values.push(value); // Fault: ignores the capacity.
    return { status: "accepted" };
  }
  tryPeek(): ValueResult<number> {
    return this.values.length === 0
      ? { status: "empty" }
      : { status: "ok", value: Math.min(...this.values) };
  }
  tryPop(): ValueResult<number> {
    if (this.values.length === 0) return { status: "empty" };
    // Fault: returns the maximum from a min-queue.
    const value = Math.max(...this.values);
    this.values.splice(this.values.indexOf(value), 1);
    return { status: "ok", value };
  }
}

function failureKind(failure: string | null): string | null {
  return failure === null ? null : failure.split(":", 1)[0]!;
}

function factory(direction: Direction): QueueFactory {
  return direction === "min"
    ? (capacity) => new BoundedMinHeap(capacity, (a, b) => a - b)
    : (capacity) => new BoundedMaxHeap(capacity, (a, b) => a - b);
}

function expectedFirst(values: number[], direction: Direction): number | undefined {
  if (values.length === 0) return undefined;
  return direction === "min" ? Math.min(...values) : Math.max(...values);
}

function historyFailure(
  commands: Command[],
  capacity: number,
  direction: Direction,
  makeQueue: QueueFactory = factory(direction)
): string | null {
  try {
    const queue = makeQueue(capacity);
    const model: number[] = [];
    for (let index = 0; index < commands.length; index++) {
      const command = commands[index]!;
      if (command.kind === "push") {
        const actual = queue.tryPush(command.value);
        if (model.length === capacity) {
          if (actual.status !== "full" || actual.rejected !== command.value) {
            return `overflow:${index}`;
          }
        } else {
          if (actual.status !== "accepted") return `push:${index}`;
          model.push(command.value);
        }
      } else if (command.kind === "pop") {
        const actual = queue.tryPop();
        const expected = expectedFirst(model, direction);
        if (expected === undefined) {
          if (actual.status !== "empty") return `empty-pop:${index}`;
        } else {
          if (actual.status !== "ok" || actual.value !== expected) {
            return `pop:${index}`;
          }
          model.splice(model.indexOf(expected), 1);
        }
      } else {
        const actual = queue.tryPeek();
        const expected = expectedFirst(model, direction);
        if (expected === undefined) {
          if (actual.status !== "empty") return `empty-peek:${index}`;
        } else if (actual.status !== "ok" || actual.value !== expected) {
          return `peek:${index}`;
        }
      }

      const root = queue.tryPeek();
      const expected = expectedFirst(model, direction);
      if (queue.size !== model.length) return `size:${index}`;
      if (queue.isEmpty() !== (model.length === 0)) return `is-empty:${index}`;
      if (queue.isFull() !== (model.length === capacity)) return `is-full:${index}`;
      if (expected === undefined ? root.status !== "empty" : root.status !== "ok" || root.value !== expected) {
        return `root:${index}`;
      }
    }

    while (model.length > 0) {
      const actual = queue.tryPop();
      const expected = expectedFirst(model, direction);
      if (actual.status !== "ok" || actual.value !== expected) return "drain";
      model.splice(model.indexOf(expected!), 1);
    }
    if (queue.tryPop().status !== "empty" || queue.size !== 0) return "final-empty";
    return null;
  } catch (error) {
    return `throw:${error instanceof Error ? error.name : typeof error}`;
  }
}

function shrinkCommands(
  input: Command[],
  stillFails: (commands: Command[]) => boolean
): Command[] {
  let current = [...input];
  let chunk = Math.max(1, Math.floor(current.length / 2));
  while (current.length > 0 && chunk >= 1) {
    let reduced = false;
    for (let start = 0; start < current.length; start += chunk) {
      const candidate = current.filter((_, index) => index < start || index >= start + chunk);
      if (candidate.length < current.length && stillFails(candidate)) {
        current = candidate;
        reduced = true;
        break;
      }
    }
    if (!reduced) {
      if (chunk === 1) break;
      chunk = Math.max(1, Math.floor(chunk / 2));
    }
  }
  return current;
}

const exhaustiveCommands: Command[] = [
  { kind: "push", value: 0 },
  { kind: "push", value: 1 },
  { kind: "push", value: 2 },
  { kind: "pop" },
  { kind: "peek" },
];

describe("bounded priority queues", () => {
  it("validates capacity and comparator before reading an iterable", () => {
    let iteratorLookups = 0;
    const items = Object.defineProperty({}, Symbol.iterator, {
      get() {
        iteratorLookups++;
        throw new Error("iterator accessed");
      },
    }) as Iterable<number>;

    for (const invalid of [
      -1,
      1.5,
      NaN,
      Infinity,
      -Infinity,
      "1",
      new Number(1),
      2 ** 32,
      null,
      false,
      {},
      1n,
    ]) {
      assert.throws(
        () => BoundedMinHeap.fromIterable(invalid as number, items, (a, b) => a - b),
        RangeError
      );
    }
    assert.throws(
      () => BoundedMinHeap.fromIterable(1, items, null as unknown as (a: number, b: number) => number),
      TypeError
    );
    assert.equal(iteratorLookups, 0);

    const maximum = new BoundedMinHeap(0xffff_ffff, (a: number, b: number) => a - b);
    assert.equal(maximum.capacity, 0xffff_ffff);
    assert.equal(maximum.isEmpty(), true);
    assert.equal(maximum.isFull(), false);
  });

  it("returns discriminated empty/full results and preserves undefined identity", () => {
    const zero = new BoundedMinHeap<undefined>(0, () => 0);
    assert.deepEqual(zero.tryPeek(), { status: "empty" });
    assert.deepEqual(zero.tryPop(), { status: "empty" });
    const rejected = { key: "same object" };
    const objectQueue = new BoundedMinHeap<typeof rejected>(0, () => 0);
    const full = objectQueue.tryPush(rejected);
    assert.equal(full.status, "full");
    if (full.status === "full") assert.equal(full.rejected, rejected);

    const queue = new BoundedMinHeap<undefined>(1, () => 0);
    assert.deepEqual(queue.tryPush(undefined), { status: "accepted" });
    const peek = queue.tryPeek();
    assert.deepEqual(peek, { status: "ok", value: undefined });
    if (peek.status === "ok") assert.equal(Object.hasOwn(peek, "value"), true);
    assert.deepEqual(queue.tryPush(undefined), { status: "full", rejected: undefined });
    const pop = queue.tryPop();
    assert.deepEqual(pop, { status: "ok", value: undefined });
    if (pop.status === "ok") assert.equal(Object.hasOwn(pop, "value"), true);
  });

  it("keeps the configured capacity immutable for JavaScript callers", () => {
    const queue = new BoundedMinHeap(1, (a: number, b: number) => a - b);
    assert.equal(Object.hasOwn(queue, "capacity"), false);
    assert.deepEqual(queue.tryPush(3), { status: "accepted" });

    assert.equal(Reflect.set(queue, "capacity", 0), false);
    assert.throws(() => {
      (queue as unknown as { capacity: number }).capacity = 0;
    }, TypeError);

    assert.equal(queue.capacity, 1);
    assert.equal(queue.isFull(), true);
    assert.deepEqual(queue.tryPush(1), { status: "full", rejected: 1 });
    assert.equal(queue.size, 1);
    assert.deepEqual(queue.tryPeek(), { status: "ok", value: 3 });
  });

  it("does not invoke the comparator when a push is already full", () => {
    let comparisons = 0;
    const queue = new BoundedMinHeap(1, (a: number, b: number) => {
      comparisons++;
      return a - b;
    });
    queue.tryPush(1);
    assert.equal(comparisons, 0);
    assert.deepEqual(queue.tryPush(2), { status: "full", rejected: 2 });
    assert.equal(comparisons, 0);
    assert.deepEqual(queue.tryPeek(), { status: "ok", value: 1 });
  });

  it("validates comparator result types while allowing any non-NaN number sign", () => {
    for (const bad of [NaN, "-1", null, undefined, {}, true]) {
      const queue = new BoundedMinHeap<number>(2, (() => bad) as (a: number, b: number) => number);
      queue.tryPush(1);
      assert.throws(() => queue.tryPush(0), TypeError);
      assert.deepEqual(queue.tryPeek(), { status: "ok", value: 1 });
    }

    for (const result of [-Infinity, -0.25, 0, 0.25, Infinity]) {
      const queue = new BoundedMinHeap<number>(2, () => result);
      assert.deepEqual(queue.tryPush(1), { status: "accepted" });
      assert.deepEqual(queue.tryPush(0), { status: "accepted" });
      assert.equal(queue.size, 2);
    }
  });

  it("stages all comparator calls before a mutation", () => {
    for (let throwAt = 1; throwAt <= 3; throwAt++) {
      let calls = 0;
      let active = false;
      const queue = new BoundedMinHeap<number>(8, (a, b) => {
        if (active && ++calls === throwAt) throw new Error(`compare-${throwAt}`);
        return a - b;
      });
      for (const value of [10, 20, 30, 40, 50, 60, 70]) queue.tryPush(value);
      calls = 0;
      active = true;
      assert.throws(() => queue.tryPush(-1), new RegExp(`compare-${throwAt}`));
      active = false;
      assert.deepEqual(drain(queue), [10, 20, 30, 40, 50, 60, 70]);
    }

    for (let throwAt = 1; throwAt <= 4; throwAt++) {
      let calls = 0;
      let active = false;
      const queue = new BoundedMinHeap<number>(8, (a, b) => {
        if (active && ++calls === throwAt) throw new Error(`pop-compare-${throwAt}`);
        return a - b;
      });
      for (const value of [1, 2, 3, 4, 5, 6, 7]) queue.tryPush(value);
      calls = 0;
      active = true;
      assert.throws(() => queue.tryPop(), new RegExp(`pop-compare-${throwAt}`));
      active = false;
      assert.deepEqual(drain(queue), [1, 2, 3, 4, 5, 6, 7]);
    }

    for (const operation of ["push", "pop"] as const) {
      let calls = 0;
      let invalidAt = 0;
      const queue = new BoundedMinHeap<number>(8, (a, b) => {
        if (invalidAt > 0 && ++calls === invalidAt) return NaN;
        return a - b;
      });
      for (const value of [1, 2, 3, 4, 5, 6, 7]) queue.tryPush(value);
      calls = 0;
      invalidAt = operation === "push" ? 2 : 3;
      assert.throws(
        () => operation === "push" ? queue.tryPush(-1) : queue.tryPop(),
        TypeError
      );
      invalidAt = 0;
      assert.deepEqual(drain(queue), [1, 2, 3, 4, 5, 6, 7]);
    }

    const constructionError = new Error("construction comparison failed");
    let constructionCalls = 0;
    assert.throws(
      () => BoundedMinHeap.fromIterable<number>(4, [4, 3, 2, 1], (a, b) => {
        constructionCalls++;
        if (constructionCalls === 2) throw constructionError;
        return a - b;
      }),
      (error) => error === constructionError
    );
    assert.equal(constructionCalls, 2);
  });

  it("uses one-pass iterator construction and closes on the first overflow", () => {
    let iteratorCount = 0;
    let nextCount = 0;
    let returnCount = 0;
    const overflow = { id: "overflow" };
    const values = [4, 1, 3, overflow];
    const iterable: Iterable<number | typeof overflow> = {
      [Symbol.iterator]() {
        iteratorCount++;
        let index = 0;
        return {
          next() {
            nextCount++;
            return index < values.length
              ? { value: values[index++]!, done: false }
              : { value: undefined, done: true };
          },
          return() {
            returnCount++;
            return { value: undefined, done: true };
          },
        };
      },
    };
    const result = BoundedMinHeap.fromIterable(3, iterable, (a, b) => {
      const av = typeof a === "number" ? a : 5;
      const bv = typeof b === "number" ? b : 5;
      return av - bv;
    });
    assert.equal(result.status, "capacity-exceeded");
    if (result.status === "capacity-exceeded") {
      assert.equal(result.rejected, overflow);
      assert.deepEqual(drain(result.heap), [1, 3, 4]);
    }
    assert.equal(iteratorCount, 1);
    assert.equal(nextCount, 4);
    assert.equal(returnCount, 1);

    let cleanupCount = 0;
    const cleanupError = new Error("cleanup failed");
    const cleanupIterable = {
      [Symbol.iterator]() {
        let n = 0;
        return {
          next() {
            return { value: n++, done: false };
          },
          return() {
            cleanupCount++;
            throw cleanupError;
          },
        };
      },
    };
    assert.throws(
      () => BoundedMinHeap.fromIterable(0, cleanupIterable, (a, b) => a - b),
      (error) => error === cleanupError
    );
    assert.equal(cleanupCount, 1);
  });

  it("propagates iterator next errors and succeeds at exact capacity", () => {
    const nextError = new Error("next failed");
    const throwingIterable = {
      [Symbol.iterator]() {
        return {
          next() {
            throw nextError;
          },
        };
      },
    };
    assert.throws(
      () => BoundedMinHeap.fromIterable<number>(2, throwingIterable, (a, b) => a - b),
      (error) => error === nextError
    );
    const exact = BoundedMaxHeap.fromIterable(3, [1, 5, 3], (a, b) => a - b);
    assert.equal(exact.status, "built");
    if (exact.status === "built") assert.deepEqual(drain(exact.heap), [5, 3, 1]);
  });

  it("preserves object identity for equal-priority records without stable-tie promises", () => {
    const entries = [
      { priority: 1, id: "a" },
      { priority: 1, id: "b" },
      { priority: 0, id: "c" },
      { priority: 1, id: "d" },
    ];
    const queue = new BoundedMinHeap<(typeof entries)[number]>(
      4,
      (a, b) => a.priority - b.priority
    );
    for (const entry of entries) queue.tryPush(entry);
    const output = drain(queue);
    assert.deepEqual(output.map((entry) => entry.priority), [0, 1, 1, 1]);
    for (const entry of entries) assert.equal(output.includes(entry), true);
  });

  it("exhausts all short histories against an independent sorted model", () => {
    let histories = 0;
    let operations = 0;
    const maxLength = 6;
    for (const direction of ["min", "max"] as const) {
      for (let capacity = 0; capacity <= 3; capacity++) {
        const commands: Command[] = [];
        const enumerate = (remaining: number): void => {
          const failure = historyFailure(commands, capacity, direction);
          if (failure !== null) {
            assert.fail(
              `exhaustive history failed: direction=${direction} capacity=${capacity} commands=${JSON.stringify(commands)} failure=${failure}`
            );
          }
          histories++;
          operations += commands.length;
          if (remaining === 0) return;
          for (const command of exhaustiveCommands) {
            commands.push(command);
            enumerate(remaining - 1);
            commands.pop();
          }
        };
        enumerate(maxLength);
      }
    }
    assert.equal(histories, 156_248);
    assert.equal(operations, 898_440);
  });

  it("runs replayable seeded histories and shrinks a negative-control failure", () => {
    const seeds: Record<Direction, number> = { min: 0x04d0, max: 0x04d1 };
    let totalHistories = 0;
    let totalOperations = 0;
    for (const direction of ["min", "max"] as const) {
      let state = seeds[direction] >>> 0;
      const random = (): number => {
        state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
        return state;
      };
      for (let historyIndex = 0; historyIndex < 250; historyIndex++) {
        const length = 1 + (random() % 100);
        const commands: Command[] = [];
        for (let index = 0; index < length; index++) {
          const choice = random() % 5;
          if (choice < 3) {
            const value = [Number.MIN_SAFE_INTEGER, -1, 0, 1, Number.MAX_SAFE_INTEGER][random() % 5]!;
            commands.push({ kind: "push", value });
          } else {
            commands.push({ kind: choice === 3 ? "pop" : "peek" });
          }
        }
        const failure = historyFailure(commands, 7, direction);
        if (failure !== null) {
          const reduced = shrinkCommands(commands, (candidate) =>
            historyFailure(candidate, 7, direction) === failure
          );
          assert.fail(
            `seeded failure seed=${seeds[direction]} direction=${direction} history=${historyIndex} failure=${failure} original=${JSON.stringify(commands)} reduced=${JSON.stringify(reduced)}`
          );
        }
        totalHistories++;
        totalOperations += commands.length;
      }
    }
    assert.equal(totalHistories, 500);
    assert.equal(totalOperations, 25_955);

    // This seed produces harmless prefix pushes; the final two pushes and pop
    // expose the adapter's wrong-removal failure. The serialized commands are
    // also printed on failure so the minimized history can be replayed alone.
    const seed = 0x5eed;
    let state = seed >>> 0;
    const random = (): number => {
      state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
      return state;
    };
    const original: Command[] = Array.from({ length: 12 }, () => ({
      kind: "push",
      value: 10 + (random() % 20),
    }));
    original.push(
      { kind: "push", value: 2 },
      { kind: "push", value: 1 },
      { kind: "pop" }
    );
    const actualFailure = (commands: Command[]): string | null =>
      historyFailure(commands, 32, "min", (capacity) => new WrongRemovalMinQueue(capacity));
    const expectedFailure = failureKind(actualFailure(original));
    assert.equal(expectedFailure, "pop");

    const reduced = shrinkCommands(original, (candidate) =>
      failureKind(actualFailure(candidate)) === expectedFailure
    );
    assert.equal(
      failureKind(actualFailure(reduced)),
      expectedFailure,
      `seed=${seed} original=${JSON.stringify(original)} reduced=${JSON.stringify(reduced)}`
    );
    for (let index = 0; index < reduced.length; index++) {
      const oneDeletion = reduced.filter((_, candidateIndex) => candidateIndex !== index);
      assert.notEqual(
        failureKind(actualFailure(oneDeletion)),
        expectedFailure,
        `not 1-minimal seed=${seed} index=${index} reduced=${JSON.stringify(reduced)}`
      );
    }
    assert.equal(reduced.length, 3);
    assert.equal(reduced[0]?.kind, "push");
    assert.equal(reduced[1]?.kind, "push");
    assert.equal(reduced[2]?.kind, "pop");
    process.stdout.write(
      `negative-control shrink seed=${seed} failure=${expectedFailure} original=${JSON.stringify(original)} reduced=${JSON.stringify(reduced)}\n`
    );
    assert.ok(
      actualFailure(reduced) !== null,
      `replay seed=${seed} original=${JSON.stringify(original)} reduced=${JSON.stringify(reduced)}`
    );
  });

  it("detects wrong-removal and over-capacity faulty adapters", () => {
    assert.notEqual(
      historyFailure(
        [{ kind: "push", value: 1 }, { kind: "push", value: 2 }, { kind: "pop" }],
        3,
        "min",
        (capacity) => new WrongRemovalMinQueue(capacity)
      ),
      null
    );
    assert.notEqual(
      historyFailure(
        [{ kind: "push", value: 1 }, { kind: "push", value: 2 }],
        1,
        "min",
        (capacity) => new WrongRemovalMinQueue(capacity)
      ),
      null
    );
  });
});

function drain<T>(queue: BoundedPriorityQueue<T>): T[] {
  const values: T[] = [];
  while (true) {
    const result = queue.tryPop();
    if (result.status === "empty") return values;
    values.push(result.value);
  }
}
