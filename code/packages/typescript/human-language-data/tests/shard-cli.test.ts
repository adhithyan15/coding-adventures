import { existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  SHARD_PLANS,
  metaOf,
  runShardCli,
  safeLedgerPath,
  shardContents,
  shardFilename,
  shardLedger,
  unshardContents,
  unshardLedger,
  type ShardPlan,
} from "../src/shard-cli.js";
import { defaultCurriculumRoot, loadCurriculumSpine } from "../src/loader.js";
import { listShardNames } from "../src/shard.js";

const SPINE = SHARD_PLANS.find((plan) => plan.path === "core/spine.json")!;

describe("the real core/spine.json round trip", () => {
  // The proof the whole PR rests on. If this is not byte-exact, the shards and
  // the generated monolith cannot both be trusted and `--check` becomes noise
  // people learn to ignore.
  it("rebuilds the committed monolith byte for byte", () => {
    const root = defaultCurriculumRoot();
    const onDisk = readFileSync(join(root, "core", "spine.json"), "utf8");
    expect(unshardContents(root, SPINE)).toBe(onDisk);
  });

  it("passes its own --check", () => {
    expect(runShardCli(["--check"])).toBe(0);
  });

  it("keeps every node, in the authored ladder order", () => {
    // Sorted filename order must reproduce authored order, which is why the
    // shard names carry an ordinal prefix. Alphabetical node order would pass a
    // naive round trip and silently re-sort the ladder.
    const root = defaultCurriculumRoot();
    const spine = loadCurriculumSpine(root);
    const names = listShardNames(join(root, "core", "spine.json"));
    const fromFilenames = names
      .filter((name) => name !== "_meta.json")
      .map((name) => name.replace(/^\d+-/, "").replace(/\.json$/, ""));
    expect(fromFilenames).toEqual(spine.nodes.map((node) => node.id));
    expect(fromFilenames).not.toEqual([...fromFilenames].sort());
  });

  it("gives every node a filename that is safe on every filesystem", () => {
    // Verified, not assumed — the brief said node ids "are already [A-Z-]+ so
    // they are safe filenames", and that is the kind of claim that is true until
    // the pull request that makes it false.
    for (const node of loadCurriculumSpine().nodes) {
      expect(node.id).toMatch(/^[A-Z][A-Z0-9-]*$/);
    }
  });
});

describe("shard/unshard on a scratch ledger", () => {
  let root: string;

  const plan: ShardPlan = {
    path: "core/toy.json",
    sections: [{ key: "nodes", idOf: (element) => (element as { id?: unknown }).id as string }],
    // These fixtures exercise the shard/unshard round trip itself, so they keep
    // the monolith the way `core/spine.json` does. The `"removed"` disposition
    // has its own suite in chapters-shards.test.ts.
    monolith: "generated",
  };

  const document = {
    version: 1,
    stages: ["one", "two"],
    nodes: [
      { id: "ZEBRA", stage: "one" },
      { id: "APPLE", stage: "one" },
      { id: "MANGO", stage: "two" },
    ],
  };

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), "hl-shard-cli-"));
    mkdirSync(join(root, "core"));
    writeFileSync(
      join(root, "core", "toy.json"),
      `${JSON.stringify(document, null, 2)}\n`,
      "utf8",
    );
  });

  afterEach(() => {
    rmSync(root, { recursive: true, force: true });
  });

  it("round-trips losslessly, preserving non-alphabetical order", () => {
    const before = readFileSync(join(root, "core", "toy.json"), "utf8");
    shardLedger(root, plan);
    expect(unshardContents(root, plan)).toBe(before);
    // And the order really was the interesting kind.
    expect(document.nodes.map((n) => n.id)).not.toEqual(
      [...document.nodes.map((n) => n.id)].sort(),
    );
  });

  it("names shards so that sorted order is authored order", () => {
    shardLedger(root, plan);
    expect(listShardNames(join(root, "core", "toy.json"))).toEqual([
      "0010-ZEBRA.json",
      "0020-APPLE.json",
      "0030-MANGO.json",
      "_meta.json",
    ]);
  });

  it("is idempotent — sharding twice writes the same bytes", () => {
    shardLedger(root, plan);
    const first = snapshot(join(root, "core", "toy.d"));
    shardLedger(root, plan);
    expect(snapshot(join(root, "core", "toy.d"))).toEqual(first);
  });

  it("removes shards the monolith no longer produces", () => {
    shardLedger(root, plan);
    const stale = join(root, "core", "toy.d", "0999-GHOST.json");
    writeFileSync(stale, "{}\n", "utf8");
    shardLedger(root, plan);
    // Left behind, it would surface later as a node nobody can find in source.
    expect(existsSync(stale)).toBe(false);
  });

  it("splits every non-list key into _meta", () => {
    expect(metaOf(document, ["nodes"])).toEqual({ version: 1, stages: ["one", "two"] });
    expect(shardContents(document, plan).get("_meta.json")).toBe(
      `${JSON.stringify({ version: 1, stages: ["one", "two"] }, null, 2)}\n`,
    );
  });

  it("survives a hand-inserted shard at an intermediate ordinal", () => {
    // Stride-of-ten numbering exists so a node can be added between two others
    // without renaming its neighbours — renaming neighbours would be its own
    // merge conflict, which is the thing this work exists to remove.
    shardLedger(root, plan);
    writeFileSync(
      join(root, "core", "toy.d", "0015-INSERTED.json"),
      `${JSON.stringify({ id: "INSERTED", stage: "one" }, null, 2)}\n`,
      "utf8",
    );
    const rebuilt = JSON.parse(unshardContents(root, plan)) as { nodes: { id: string }[] };
    expect(rebuilt.nodes.map((n) => n.id)).toEqual(["ZEBRA", "INSERTED", "APPLE", "MANGO"]);
  });
});

describe("refusals", () => {
  let root: string;

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), "hl-shard-cli-"));
    mkdirSync(join(root, "core"));
  });

  afterEach(() => {
    rmSync(root, { recursive: true, force: true });
  });

  function write(document: unknown): void {
    writeFileSync(
      join(root, "core", "toy.json"),
      `${JSON.stringify(document, null, 2)}\n`,
      "utf8",
    );
  }

  const plan: ShardPlan = {
    path: "core/toy.json",
    sections: [{ key: "nodes", idOf: (element) => (element as { id?: unknown }).id as string }],
    // These fixtures exercise the shard/unshard round trip itself, so they keep
    // the monolith the way `core/spine.json` does. The `"removed"` disposition
    // has its own suite in chapters-shards.test.ts.
    monolith: "generated",
  };

  it("refuses an id that is not a safe filename", () => {
    // `idOf` reads a field out of authored JSON. An id of `../../../etc/passwd`
    // deciding where this tool writes is exactly the bug a "the ids are fine"
    // comment produces two years later.
    write({ version: 1, nodes: [{ id: "../../../etc/passwd" }] });
    expect(() => shardLedger(root, plan)).toThrow(/not a safe shard filename/);
  });

  it("refuses a Windows reserved device name", () => {
    // `CON.json` cannot be created on Windows, so this would produce a shard set
    // that silently fails to check out on half the machines that use it.
    write({ version: 1, nodes: [{ id: "CON" }] });
    expect(() => shardLedger(root, plan)).toThrow(/reserved device name/);
  });

  it("refuses duplicate ids, which would silently lose an element", () => {
    write({ version: 1, nodes: [{ id: "ALPHA" }, { id: "ALPHA" }] });
    expect(() => shardLedger(root, plan)).toThrow(/duplicate nodes id 'ALPHA'/);
  });

  it("ACCEPTS a ledger whose array is not last, recording the key order", () => {
    // This used to be a refusal, and HL21 §2.5 said so: with no way to record
    // where the array sat, the rebuild appended it last, so a ledger with a key
    // AFTER the array could not round-trip and was rejected rather than
    // silently reordered.
    //
    // `<track>/curriculum.json` is the ledger that refusal was waiting for —
    // `{version, language, path, spine, extensions, conceptAliases}`, with three
    // sharded keys in the middle — so the position is now written down in
    // `_meta.json` and the refusal is gone. The property that mattered is
    // unchanged and is what this test asserts: the rebuild is byte-exact.
    const document = { version: 1, nodes: [{ id: "ALPHA" }], trailing: true };
    write(document);
    const before = readFileSync(join(root, "core", "toy.json"), "utf8");
    shardLedger(root, plan);
    expect(unshardContents(root, plan)).toBe(before);

    // And the order really was the interesting kind: `nodes` is not last.
    expect(Object.keys(document).at(-1)).not.toBe("nodes");
    const meta = JSON.parse(
      readFileSync(join(root, "core", "toy.d", "_meta.json"), "utf8"),
    ) as { _keys?: string[] };
    expect(meta._keys).toEqual(["version", "nodes", "trailing"]);
  });

  it("does NOT record a key order when the array is already last", () => {
    // The 21 shard sets committed before `_keys` existed must not acquire a
    // line none of them needs. `needsKeyOrder` is what keeps `_meta.json`
    // byte-identical for the suffix case.
    write({ version: 1, nodes: [{ id: "ALPHA" }] });
    shardLedger(root, plan);
    const meta = JSON.parse(
      readFileSync(join(root, "core", "toy.d", "_meta.json"), "utf8"),
    ) as Record<string, unknown>;
    expect(Object.hasOwn(meta, "_keys")).toBe(false);
  });

  it("refuses a ledger with no such array", () => {
    write({ version: 1 });
    expect(() => shardLedger(root, plan)).toThrow(/no top-level 'nodes'/);
  });

  it("refuses a ledger that already has a top-level '_keys'", () => {
    // `_keys` is read as the recorded key order and stripped on rebuild, so a
    // document with a real one would lose it and come back reordered.
    write({ version: 1, _keys: ["whatever"], nodes: [{ id: "ALPHA" }] });
    expect(() => shardLedger(root, plan)).toThrow(/reserves to record/);
  });

  it("contains the ledger path inside the curriculum root", () => {
    // `--shard ../../.github/workflows/release.yml` is a perfectly good relative
    // path and a perfectly terrible thing to overwrite.
    expect(() => safeLedgerPath(root, "../escape.json")).toThrow(/unsafe ledger path/);
    expect(() => safeLedgerPath(root, "core/../../escape.json")).toThrow(/unsafe ledger path/);
    expect(() => safeLedgerPath(root, "core/notes.md")).toThrow(/unsafe ledger path/);
    // On Windows `path.relative` returns the TARGET unchanged when the two paths
    // sit on different roots, so `D:\evil\x.json` is neither ".." nor
    // "../"-prefixed and would pass the lexical containment test.
    expect(() => safeLedgerPath(root, "D:\\evil\\x.json")).toThrow(/must be relative/);
    expect(() => safeLedgerPath(root, "/etc/passwd.json")).toThrow(/must be relative/);
    // Compared against the REALPATH of root: `safeLedgerPath` now resolves the
    // parent directory, and `mkdtempSync(tmpdir())` is itself behind a symlink
    // on macOS (`/var` -> `/private/var`).
    expect(safeLedgerPath(root, "core/toy.json")).toBe(
      join(realpathSync(root), "core", "toy.json"),
    );
  });
});

describe("the writer cannot be walked outside the checkout", () => {
  let root: string;

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), "hl-shard-cli-"));
    mkdirSync(join(root, "core"));
    writeFileSync(
      join(root, "core", "toy.json"),
      `${JSON.stringify({ version: 1, nodes: [{ id: "ALPHA" }] }, null, 2)}\n`,
      "utf8",
    );
  });

  afterEach(() => {
    rmSync(root, { recursive: true, force: true });
  });

  const plan: ShardPlan = {
    path: "core/toy.json",
    sections: [{ key: "nodes", idOf: (element) => (element as { id?: unknown }).id as string }],
    // These fixtures exercise the shard/unshard round trip itself, so they keep
    // the monolith the way `core/spine.json` does. The `"removed"` disposition
    // has its own suite in chapters-shards.test.ts.
    monolith: "generated",
  };

  function trySymlink(target: string, path: string): boolean {
    for (const type of ["dir", "junction"] as const) {
      try {
        symlinkSync(target, path, type);
        return true;
      } catch {
        // Windows needs Developer Mode for "dir" but allows "junction".
      }
    }
    return false;
  }

  it("refuses a symlinked shard directory instead of deleting through it", (ctx) => {
    // The HIGH finding this test exists for. `--shard` used to gate on
    // `existsSync`, which FOLLOWS symlinks, so a committed
    // `core/toy.d -> ../victim` had rmSync delete every *.json in the victim
    // directory and writeFileSync put shards there. Pointed at `.git` or
    // `~/.ssh`, `npm run shard` on such a branch is arbitrary file deletion.
    const victim = join(root, "victim");
    mkdirSync(victim);
    const treasure = join(victim, "secrets.json");
    writeFileSync(treasure, '{ "keep": "me" }\n', "utf8");
    if (!trySymlink(victim, join(root, "core", "toy.d"))) ctx.skip();

    expect(() => shardLedger(root, plan)).toThrow(/symbolic link/);
    // The point of the test: the victim file is still there.
    expect(existsSync(treasure)).toBe(true);
    expect(readFileSync(treasure, "utf8")).toBe('{ "keep": "me" }\n');
  });

  it("refuses a DANGLING symlinked monolith instead of creating its target", (ctx) => {
    // The round-2 finding. `if (existsSync(monolith)) assertRealFile(monolith)`
    // skipped the guard entirely for a dangling link, because `existsSync` uses
    // `stat` and a dangling link has nothing to stat. `writeFileSync` then
    // opened O_CREAT through the link and created the target — attacker-chosen
    // JSON, at a path outside the checkout.
    const target = join(root, "not-created-yet.json");
    shardLedger(root, plan);
    rmSync(join(root, "core", "toy.json"));
    try {
      symlinkSync(target, join(root, "core", "toy.json"), "file");
    } catch {
      ctx.skip();
    }

    expect(() => unshardLedger(root, plan)).toThrow(/symbolic link/);
    expect(existsSync(target)).toBe(false);
  });

  it("refuses a symlinked PARENT directory, not just a symlinked X.d", (ctx) => {
    // Round 3. Every guard added over the previous two rounds calls `lstat`,
    // which does not follow the FINAL component — but does follow every
    // component before it. So `core/toy.d` as a link was refused and `core`
    // itself as a link walked straight through, with all four guards satisfied:
    // `rmSync` deleted out-of-tree files and `writeFileSync` overwrote them.
    // Lexical containment cannot see this; only `realpath` can.
    const outside = mkdtempSync(join(tmpdir(), "hl-victim-"));
    const treasure = join(outside, "treasure.json");
    writeFileSync(treasure, '{ "keep": "me" }\n', "utf8");
    writeFileSync(
      join(outside, "toy.json"),
      `${JSON.stringify({ version: 1, nodes: [{ id: "ALPHA" }] }, null, 2)}\n`,
      "utf8",
    );

    const linkedRoot = mkdtempSync(join(tmpdir(), "hl-shard-cli-"));
    try {
      if (!trySymlink(outside, join(linkedRoot, "core"))) ctx.skip();

      expect(() => shardLedger(linkedRoot, plan)).toThrow(/resolves outside the curriculum root/);
      expect(() => unshardLedger(linkedRoot, plan)).toThrow(
        /resolves outside the curriculum root/,
      );
      expect(existsSync(treasure)).toBe(true);
    } finally {
      rmSync(linkedRoot, { recursive: true, force: true });
      rmSync(outside, { recursive: true, force: true });
    }
  });

  it("refuses a non-directory squatting on the shard directory name", () => {
    // `mkdirSync(dir, { recursive: true })` silently no-ops on an existing
    // entry, so without this the run would carry on against something that is
    // not a shard directory at all.
    writeFileSync(join(root, "core", "toy.d"), "not a directory\n", "utf8");
    expect(() => shardLedger(root, plan)).toThrow(/exists and is not a directory/);
  });
});

describe("metaOf cannot be walked through the prototype setter", () => {
  it("keeps __proto__ as data and does not drop it", () => {
    // Plain `meta[key] = value` goes through [[Set]] and invokes the
    // `__proto__` setter: the key vanished from the emitted _meta.json and the
    // local object's prototype was swapped. Contained, but a silent data loss
    // on top of being the exact sink the key check exists to close.
    const hostile = JSON.parse('{ "version": 1, "__proto__": { "polluted": "yes" }, "nodes": [] }');
    const meta = metaOf(hostile, ["nodes"]);
    expect(Object.getPrototypeOf(meta)).toBe(null);
    expect(Object.hasOwn(meta, "__proto__")).toBe(true);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.prototype).not.toHaveProperty("polluted");
  });
});

describe("the command line", () => {
  it("rejects an unknown mode", () => {
    expect(runShardCli([])).toBe(2);
    expect(runShardCli(["--frobnicate"])).toBe(2);
    expect(runShardCli(["--shard"])).toBe(2);
    expect(runShardCli(["--shard", "a.json", "b.json"])).toBe(2);
  });

  it("rejects a path that is not a registered ledger", () => {
    expect(runShardCli(["--check", "core/not-a-ledger.json"])).toBe(2);
  });

  it("accepts the registered ledger by name", () => {
    expect(runShardCli(["--check", "core/spine.json"])).toBe(0);
  });
});

describe("shardFilename", () => {
  it("zero-pads so that string order is numeric order", () => {
    expect(shardFilename(0, "A")).toBe("0010-A.json");
    expect(shardFilename(8, "B")).toBe("0090-B.json");
    expect(shardFilename(9, "C")).toBe("0100-C.json");
    // The property, not just the examples: unpadded, "0100" would sort before "0090".
    const names = [shardFilename(8, "B"), shardFilename(9, "C")];
    expect([...names].sort()).toEqual(names);
  });

  it("refuses to overflow the pad width rather than silently reordering", () => {
    // At 1000 elements the ordinal becomes `10000`, which sorts BEFORE `1010` —
    // so sorted-filename order stops reproducing authored order. `--check`
    // cannot catch it, because both directions use the same broken order and
    // the round trip still closes. The result is a re-sorted ladder nobody sees.
    expect(shardFilename(998, "LAST-OK")).toBe("9990-LAST-OK.json");
    expect(() => shardFilename(999, "OVERFLOW")).toThrow(/does not fit 4 digits/);
    // And the ordering claim the guard protects, stated outright.
    expect(["9990-A.json", "10000-B.json"].sort()).toEqual(["10000-B.json", "9990-A.json"]);
  });
});

describe("a plan that names shards by its own ordinal, with no id", () => {
  // The `<track>/chapters.json` shape: identity IS the number, so the filename
  // is the padded number and nothing else.
  let root: string;

  const plan: ShardPlan = {
    path: "core/toy.json",
    sections: [
      { key: "chapters", ordinalOf: (element) => (element as { chapter: number }).chapter },
    ],
    monolith: "generated",
  };

  const document = {
    version: 1,
    language: "toy",
    chapters: [{ chapter: 2 }, { chapter: 9 }, { chapter: 10 }, { chapter: 11 }],
  };

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), "hl-shard-ordinal-"));
    mkdirSync(join(root, "core"));
    writeFileSync(join(root, "core", "toy.json"), `${JSON.stringify(document, null, 2)}\n`, "utf8");
  });

  afterEach(() => {
    rmSync(root, { recursive: true, force: true });
  });

  it("names each shard for its own number, zero-padded", () => {
    shardLedger(root, plan);
    expect(listShardNames(join(root, "core", "toy.json"))).toEqual([
      "0002.json",
      "0009.json",
      "0010.json",
      "0011.json",
      "_meta.json",
    ]);
  });

  it("round-trips byte-exactly", () => {
    const before = readFileSync(join(root, "core", "toy.json"), "utf8");
    shardLedger(root, plan);
    expect(unshardContents(root, plan)).toBe(before);
  });

  it("would have re-sorted the ledger under UNPADDED names", () => {
    // THE trap, in four chapters. This is the test that fails if someone decides
    // the padding is noise and renames the shards to `2.json`, `9.json`, … —
    // under which sorted order is 10, 11, 2, 9 and chapter 2 lands last.
    const unpadded = document.chapters.map((c) => `${c.chapter}.json`);
    expect([...unpadded].sort()).toEqual(["10.json", "11.json", "2.json", "9.json"]);
    expect([...unpadded].sort()).not.toEqual(unpadded);

    // Padded, the same four sort into authored order.
    const padded = document.chapters.map((c) => `${String(c.chapter).padStart(4, "0")}.json`);
    expect([...padded].sort()).toEqual(padded);
  });

  it("refuses two elements that claim the same number", () => {
    // With no id, a duplicate chapter number is a duplicate FILENAME and the
    // second element would overwrite the first — one chapter gone, no error, and
    // `--check` agreeing with itself about the truncated set ever after.
    writeFileSync(
      join(root, "core", "toy.json"),
      `${JSON.stringify({ ...document, chapters: [{ chapter: 3 }, { chapter: 3 }] }, null, 2)}\n`,
      "utf8",
    );
    expect(() => shardLedger(root, plan)).toThrow(/already taken/);
  });
});

/** Every file in a shard directory, as name -> bytes. */
function snapshot(dir: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const name of listShardNames(`${dir.slice(0, -2)}.json`)) {
    out[name] = readFileSync(join(dir, name), "utf8");
  }
  return out;
}
