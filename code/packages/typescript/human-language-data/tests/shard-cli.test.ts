import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
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
    listKey: "nodes",
    idOf: (element) => (element as { id?: unknown }).id as string,
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
    expect(metaOf(document, "nodes")).toEqual({ version: 1, stages: ["one", "two"] });
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
    listKey: "nodes",
    idOf: (element) => (element as { id?: unknown }).id as string,
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

  it("refuses a ledger whose array is not the last top-level key", () => {
    // Appending on rebuild would otherwise silently reorder the keys.
    write({ version: 1, nodes: [{ id: "ALPHA" }], trailing: true });
    expect(() => shardLedger(root, plan)).toThrow(/must be the last top-level key/);
  });

  it("refuses a ledger with no such array", () => {
    write({ version: 1 });
    expect(() => shardLedger(root, plan)).toThrow(/no top-level 'nodes' array/);
  });

  it("contains the ledger path inside the curriculum root", () => {
    // `--shard ../../.github/workflows/release.yml` is a perfectly good relative
    // path and a perfectly terrible thing to overwrite.
    expect(() => safeLedgerPath(root, "../escape.json")).toThrow(/unsafe ledger path/);
    expect(() => safeLedgerPath(root, "core/../../escape.json")).toThrow(/unsafe ledger path/);
    expect(() => safeLedgerPath(root, "core/notes.md")).toThrow(/unsafe ledger path/);
    expect(safeLedgerPath(root, "core/toy.json")).toBe(join(root, "core", "toy.json"));
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
});

/** Every file in a shard directory, as name -> bytes. */
function snapshot(dir: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const name of listShardNames(`${dir.slice(0, -2)}.json`)) {
    out[name] = readFileSync(join(dir, name), "utf8");
  }
  return out;
}
