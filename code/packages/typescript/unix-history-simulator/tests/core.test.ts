import { describe, expect, it } from "vitest";
import { CnfBuilder, type Lit } from "@coding-adventures/sat-solver";
import fixtures from "../../../../specs/fixtures/historical-unix-v1/file-read.json";
import {
  advanceDescriptor, closeDescriptor, openDescriptor, type DescriptorTable,
} from "../src/descriptors.js";
import { demoImage, readFile, resolvePath } from "../src/filesystem.js";
import { Kernel } from "../src/kernel.js";
import { Session } from "../src/session.js";

describe("file system", () => {
  it("walks repeated slashes and directory components from root", () => {
    expect(resolvePath(demoImage, "//etc/./../etc//notes")).toMatchObject({ inode: 5 });
    expect(resolvePath(demoImage, "/../")).toMatchObject({ inode: 1 });
    expect(resolvePath(demoImage, "hello")).toMatchObject({ inode: 2 });
  });

  it("refuses invalid paths and traversal through files", () => {
    for (const path of ["", "he\0llo"]) expect(resolvePath(demoImage, path)).toBe("EINVAL");
    for (const path of ["/hello/", "/hello/..", "/hello/child"]) {
      expect(resolvePath(demoImage, path)).toBe("ENOTDIR");
    }
    expect(resolvePath(demoImage, "/missing")).toBe("ENOENT");
  });

  it("returns a byte range, a short read and then EOF", () => {
    expect(readFile(demoImage, 2, 7, 3)).toEqual(new TextEncoder().encode("Unix".slice(0, 3)));
    expect(readFile(demoImage, 2, 12, 8)).toEqual(Uint8Array.of(10));
    expect(readFile(demoImage, 2, 13, 8)).toEqual(new Uint8Array());
    expect(readFile(demoImage, 2, -1, 1)).toBe("EINVAL");
    expect(readFile(demoImage, 2, 0, -1)).toBe("EINVAL");
    expect(readFile(demoImage, 2, 0.5, 1)).toBe("EINVAL");
    expect(readFile(demoImage, 2, 0, Number.POSITIVE_INFINITY)).toBe("EINVAL");
    const unicodeImage = new Map([[7, { inode: 7, kind: "file" as const, text: "é" }]]);
    expect(readFile(unicodeImage, 7, 0, 2)).toEqual(Uint8Array.of(0xc3, 0xa9));
    expect(readFile(unicodeImage, 7, 1, 1)).toEqual(Uint8Array.of(0xa9));
    expect(readFile(demoImage, 1, 0, 8)).toBe("EISDIR");
    expect(readFile(demoImage, 999, 0, 8)).toBe("ENOENT");
  });
});

describe("kernel and shell", () => {
  it.each(fixtures)("replays $command", ({ command, output, events }) => {
    const session = new Session();
    expect(session.submit(command)).toBe(true);
    session.run();
    expect(session.events).toEqual(events);
    expect(session.output).toBe(output);
    expect(session.kernel.descriptors.every(slot => slot === null)).toBe(true);
  });

  it("advances two opens of the same inode independently and reuses a closed slot", () => {
    const kernel = new Kernel(demoImage, 2);
    expect(kernel.open("/hello")).toMatchObject({ fd: 0 });
    expect(kernel.open("/hello")).toMatchObject({ fd: 1 });
    expect(kernel.open("/hello")).toMatchObject({ error: "EMFILE" });
    expect(kernel.read(0, 3).event).toMatchObject({ nextOffset: 3 });
    expect(kernel.read(1, 1).event).toMatchObject({ offset: 0, nextOffset: 1 });
    expect(kernel.close(0)).toEqual({ call: "close", fd: 0 });
    expect(kernel.close(0)).toMatchObject({ error: "EBADF" });
    expect(kernel.read(0, 1).event).toMatchObject({ error: "EBADF" });
    expect(kernel.open("/empty")).toMatchObject({ fd: 0, offset: 0 });
    expect(kernel.open("/")).toMatchObject({ error: "EISDIR" });
    expect(kernel.list("/hello")).toMatchObject({ error: "ENOTDIR" });
    expect(kernel.list("/missing")).toMatchObject({ error: "ENOENT" });
    expect(kernel.read(1, -1).event).toMatchObject({ error: "EINVAL" });
    expect(kernel.descriptors[1]?.offset).toBe(1);
    expect(() => new Kernel(demoImage, 0)).toThrow(RangeError);
  });

  it("steps once per call, holds a pending command, and can reset", () => {
    const session = new Session();
    expect(session.step()).toBe(false);
    session.submit("cat hello");
    expect(session.submit("ls /")).toBe(false);
    expect(session.step()).toBe(true);
    expect(session.events).toHaveLength(1);
    expect(session.kernel.descriptors[0]).toMatchObject({ inode: 2, offset: 0 });
    session.reset();
    expect(session.busy).toBe(false);
    expect(session.events).toEqual([]);
    expect(session.submit("ls")).toBe(true);
    session.run();
    expect(session.output).toBe("hello\nempty\netc\n");
  });
});

type Operation = { kind: "open" | "read" | "close"; fd: number };
const alphabet: Operation[] = [
  { kind: "open", fd: 0 },
  ...[-1, 0, 1, 2].flatMap(fd => [{ kind: "read" as const, fd }, { kind: "close" as const, fd }]),
];

function compareHistory(history: readonly Operation[]) {
  // The oracle tracks occupied slots in a map and explicitly tries both slots.
  const model = new Map<number, number>();
  let table: DescriptorTable = [null, null];
  for (const { kind, fd } of history) {
    if (kind === "open") {
      const candidate = !model.has(0) ? 0 : !model.has(1) ? 1 : null;
      const result = openDescriptor(table, 2);
      expect(result?.fd ?? null).toBe(candidate);
      if (result !== undefined && candidate !== null) {
        model.set(candidate, 0);
        table = result.table;
      }
    } else {
      const result = kind === "read" ? advanceDescriptor(table, fd, 1) : closeDescriptor(table, fd);
      expect(result !== undefined).toBe(model.has(fd));
      if (result !== undefined) {
        if (kind === "read") model.set(fd, model.get(fd)! + 1);
        else model.delete(fd);
        table = result;
      }
    }
    expect(table).toEqual([0, 1].map(slot => model.has(slot) ? { inode: 2, offset: model.get(slot) } : null));
  }
}

function seeded(seed: number) {
  let state = seed;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return state >>> 0;
  };
}

describe("bounded evidence", () => {
  it("exhausts all 729 length-three histories in a two-slot table", () => {
    for (const a of alphabet) for (const b of alphabet) for (const c of alphabet) {
      compareHistory([a, b, c]);
    }
  });

  it("checks 128 seeded longer histories against the independent descriptor model", () => {
    for (let seed = 1; seed <= 128; seed++) {
      const next = seeded(seed);
      const history = Array.from({ length: 32 }, () => alphabet[next() % alphabet.length]);
      try {
        compareHistory(history);
      } catch (error) {
        throw new Error(`Replay seed ${seed}: ${JSON.stringify(history)}`, { cause: error });
      }
    }
  });

  it("fuzzes 1024 mutated commands with deterministic bytes", () => {
    const next = seeded(20261125);
    const encoder = new TextEncoder();
    const decoder = new TextDecoder();
    const corpus = ["cat /hello", "cat /empty", "ls /etc", "cat //etc/../hello", "ls /"];
    for (let i = 0; i < 1024; i++) {
      const bytes = encoder.encode(corpus[i % corpus.length]);
      for (let n = next() % 4; n > 0; n--) bytes[next() % bytes.length] = next() & 255;
      const input = decoder.decode(bytes);
      const run = () => {
        const session = new Session();
        session.submit(input);
        for (let step = 0; step < 32 && session.busy; step++) session.step();
        expect(session.busy, input).toBe(false);
        expect(session.kernel.descriptors.every(slot => slot === null), input).toBe(true);
        return [session.output, session.events];
      };
      expect(run(), input).toEqual(run());
    }
  });

  it("finds no SAT counterexample to lowest-free allocation across three slots", () => {
    function counterexample(faulty: boolean) {
      const cnf = new CnfBuilder();
      const busy = Array.from({ length: 3 }, () => cnf.newLit());
      const chosen = Array.from({ length: 4 }, () => cnf.newLit());
      const violations: Lit[] = [];
      // Pin the implementation's observed choice for each possible occupancy.
      for (let mask = 0; mask < 8; mask++) {
        const table = busy.map((_, i) => mask & (1 << i) ? { inode: 1, offset: 0 } : null);
        const fd = faulty && mask === 0 ? 1 : openDescriptor(table, 2)?.fd ?? 3;
        const differs = busy.map((bit, i) => mask & (1 << i) ? cnf.not(bit) : bit);
        for (let i = 0; i < chosen.length; i++) {
          cnf.addClause([...differs, i === fd ? chosen[i] : cnf.not(chosen[i])]);
        }
      }
      // The logical contract independently forbids a busy choice, skipping
      // any lower free slot, and reporting full while any slot is free.
      for (let i = 0; i < busy.length; i++) {
        violations.push(cnf.and(chosen[i], busy[i]));
        for (let j = 0; j < i; j++) violations.push(cnf.and(chosen[i], cnf.not(busy[j])));
        violations.push(cnf.and(chosen[3], cnf.not(busy[i])));
      }
      cnf.addClause(violations);
      return cnf.getSolver().solve();
    }
    expect(counterexample(false)).toBe("unsat");
    expect(counterexample(true)).toBe("sat");
  });
});
