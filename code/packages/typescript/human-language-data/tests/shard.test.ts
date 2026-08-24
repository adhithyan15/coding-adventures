import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  isSharded,
  listShardNames,
  mergeMetaAndList,
  readMaybeSharded,
  readShards,
  shardDirectoryFor,
  type Shard,
} from "../src/shard.js";
import { loadCurriculumSpine } from "../src/loader.js";

// A scratch curriculum root per test. Real fixtures on disk would make these
// tests a second thing that has to be kept in sync with the corpus; a temp dir
// makes each case say exactly what it is about and nothing else.
let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "hl-shard-"));
});

afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

function writeJson(path: string, value: unknown): void {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

/** Fold shards into `{ ...meta, items: [...] }` — the spine's shape, in miniature. */
function mergeItems(shards: Shard[]): { version?: number; items: unknown[] } {
  const meta = shards.find((s) => s.name === "_meta.json")?.value as
    | { version?: number }
    | undefined;
  return {
    ...(meta ?? {}),
    items: shards.filter((s) => s.name !== "_meta.json").map((s) => s.value),
  };
}

describe("shardDirectoryFor", () => {
  it("turns X.json into X.d", () => {
    expect(shardDirectoryFor(join("core", "spine.json"))).toBe(join("core", "spine.d"));
  });

  it("refuses a path that is not a .json ledger", () => {
    // Otherwise a typo yields `book.tex.d`, which exists, is empty, and takes an
    // afternoon to explain.
    expect(() => shardDirectoryFor("book/book.tex")).toThrow(/not a \.json ledger/);
  });
});

describe("reading a ledger that is sharded", () => {
  it("reads and merges every shard when the directory is present", () => {
    const monolith = join(root, "spine.json");
    writeJson(monolith, { version: 99, items: ["ignored — the shards win"] });
    const dir = join(root, "spine.d");
    mkdirSync(dir);
    writeJson(join(dir, "_meta.json"), { version: 1 });
    writeJson(join(dir, "0010-ALPHA.json"), { id: "ALPHA" });
    writeJson(join(dir, "0020-BETA.json"), { id: "BETA" });

    expect(isSharded(monolith)).toBe(true);
    expect(readMaybeSharded(monolith, mergeItems)).toEqual({
      version: 1,
      items: [{ id: "ALPHA" }, { id: "BETA" }],
    });
  });

  it("ignores non-JSON strays so an editor swapfile cannot break a build", () => {
    const monolith = join(root, "spine.json");
    const dir = join(root, "spine.d");
    mkdirSync(dir);
    writeJson(join(dir, "0010-ALPHA.json"), { id: "ALPHA" });
    writeFileSync(join(dir, "README.md"), "notes\n", "utf8");
    writeFileSync(join(dir, ".spine.json.swp"), "binary\n", "utf8");
    mkdirSync(join(dir, "nested.json"));

    // `.spine.json.swp` does not end in `.json`; `nested.json` is a directory.
    expect(listShardNames(monolith)).toEqual(["0010-ALPHA.json"]);
  });
});

describe("falling back to the monolith", () => {
  it("reads X.json unchanged when there is no X.d", () => {
    const monolith = join(root, "spine.json");
    writeJson(monolith, { version: 1, items: [{ id: "ALPHA" }] });

    expect(isSharded(monolith)).toBe(false);
    expect(readShards(monolith)).toBeNull();
    // The merge function is never called — the monolith is already the document.
    expect(
      readMaybeSharded(monolith, () => {
        throw new Error("merge must not run when the ledger is not sharded");
      }),
    ).toEqual({ version: 1, items: [{ id: "ALPHA" }] });
  });

  it("reports the monolith by name when the monolith itself is malformed", () => {
    const monolith = join(root, "spine.json");
    writeFileSync(monolith, "{ not json", "utf8");
    expect(() => readMaybeSharded(monolith, mergeItems)).toThrow(/spine\.json.*malformed JSON/s);
  });
});

describe("an empty shard directory", () => {
  it("throws rather than returning an empty ledger", () => {
    // "No spine on disk" and "a spine with no nodes" are opposite facts. A
    // loader that returns the second when it means the first hands every
    // downstream gate a clean bill of health for a corpus that is not there.
    const monolith = join(root, "spine.json");
    writeJson(monolith, { version: 1, items: [{ id: "ALPHA" }] });
    mkdirSync(join(root, "spine.d"));

    expect(() => readMaybeSharded(monolith, mergeItems)).toThrow(
      /holds no \*\.json shards/,
    );
  });

  it("throws even when a directory of strays makes it look populated", () => {
    const monolith = join(root, "spine.json");
    const dir = join(root, "spine.d");
    mkdirSync(dir);
    writeFileSync(join(dir, "notes.txt"), "wip\n", "utf8");

    expect(() => readMaybeSharded(monolith, mergeItems)).toThrow(/holds no \*\.json shards/);
  });
});

describe("sorted-order determinism", () => {
  it("merges in sorted filename order regardless of creation order", () => {
    const monolith = join(root, "spine.json");
    const dir = join(root, "spine.d");
    mkdirSync(dir);
    // Written back to front, and with the zero-padding that makes string sort
    // agree with numeric sort.
    for (const name of ["0030-GAMMA", "0010-ALPHA", "0020-BETA"]) {
      writeJson(join(dir, `${name}.json`), { id: name });
    }

    expect(listShardNames(monolith)).toEqual([
      "0010-ALPHA.json",
      "0020-BETA.json",
      "0030-GAMMA.json",
    ]);
  });

  it("sorts by code unit, not by host locale", () => {
    // `localeCompare` under en-US ignores leading punctuation and folds case, so
    // it puts `_meta.json` and `A.json` in a machine-dependent order and treats
    // `a` and `A` as equal. Code-unit order is the same everywhere, which is the
    // only property that matters for a merged artifact CI compares byte for byte.
    const monolith = join(root, "spine.json");
    const dir = join(root, "spine.d");
    mkdirSync(dir);
    for (const name of ["_meta", "B-upper", "a-lower", "0010-num"]) {
      writeJson(join(dir, `${name}.json`), { id: name });
    }

    // Code units: '0' 0x30 < 'B' 0x42 < '_' 0x5F < 'a' 0x61. Spelling the
    // expected order out literally is the guard — a refactor to `localeCompare`
    // reorders these under any full-ICU build and fails here, rather than
    // surfacing as a --check that passes on one machine and not another.
    const names = listShardNames(monolith);
    expect(names).toEqual(["0010-num.json", "B-upper.json", "_meta.json", "a-lower.json"]);
    expect(names).toEqual([...names].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0)));
  });

  it("produces byte-identical merges across repeated reads", () => {
    const monolith = join(root, "spine.json");
    const dir = join(root, "spine.d");
    mkdirSync(dir);
    for (let i = 1; i <= 12; i += 1) {
      writeJson(join(dir, `${String(i * 10).padStart(4, "0")}-NODE-${i}.json`), { id: i });
    }
    const once = JSON.stringify(readMaybeSharded(monolith, mergeItems));
    const twice = JSON.stringify(readMaybeSharded(monolith, mergeItems));
    expect(twice).toBe(once);
    // Zero-padding is load-bearing: unpadded, `100` would sort before `20`.
    expect((readMaybeSharded(monolith, mergeItems).items as { id: number }[]).map((n) => n.id))
      .toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
  });
});

describe("mergeMetaAndList", () => {
  function shardsOf(entries: Record<string, unknown>): Shard[] {
    const dir = join(root, "spine.d");
    mkdirSync(dir, { recursive: true });
    for (const [name, value] of Object.entries(entries)) writeJson(join(dir, name), value);
    return readShards(join(root, "spine.json"))!;
  }

  it("hoists _meta's fields beside the element array", () => {
    expect(
      mergeMetaAndList(
        shardsOf({
          "_meta.json": { version: 1, stages: ["pre-A1", "A1"] },
          "0010-ALPHA.json": { id: "ALPHA" },
          "0020-BETA.json": { id: "BETA" },
        }),
        "nodes",
      ),
    ).toEqual({ version: 1, stages: ["pre-A1", "A1"], nodes: [{ id: "ALPHA" }, { id: "BETA" }] });
  });

  it("requires _meta rather than defaulting it away", () => {
    // A rebase that drops _meta.json must not read as "a spine that has no stages".
    expect(() => mergeMetaAndList(shardsOf({ "0010-ALPHA.json": { id: "ALPHA" } }), "nodes")).toThrow(
      /no '_meta\.json'/,
    );
  });

  it("rejects a _meta that duplicates the element array", () => {
    expect(() =>
      mergeMetaAndList(shardsOf({ "_meta.json": { version: 1, nodes: [] } }), "nodes"),
    ).toThrow(/must not carry 'nodes'/);
  });

  it("rejects a _meta that is not an object", () => {
    expect(() => mergeMetaAndList(shardsOf({ "_meta.json": [1, 2, 3] }), "nodes")).toThrow(
      /must be a JSON object/,
    );
  });
});

describe("the real spine, through the loader", () => {
  it("loads whichever form is on disk, with the same fields either way", () => {
    // Additive by construction: with no `core/spine.d/` in the tree this is the
    // old monolith read, unchanged. Once the shards land it is the merged read.
    // The assertions below hold for both, which is the point.
    const spine = loadCurriculumSpine();
    expect(spine.version).toBe(1);
    expect(spine.stages).toContain("pre-A1");
    expect(spine.nodes.length).toBeGreaterThan(0);
    for (const node of spine.nodes) {
      expect(node.id).toMatch(/^[A-Z-]+$/);
      expect(spine.stages).toContain(node.stage);
    }
    expect(new Set(spine.nodes.map((n) => n.id)).size).toBe(spine.nodes.length);
  });

  it("keeps the nodes in stage order, which filename order must preserve", () => {
    // The spine is an ORDERED ladder: pre-A1 first, C2 last, and not
    // alphabetical within a stage. Sorted-filename order therefore cannot be
    // "sort by node id" — the shard filenames carry a numeric prefix precisely
    // so that this property survives the round trip.
    const spine = loadCurriculumSpine();
    const rank = spine.nodes.map((n) => spine.stages.indexOf(n.stage));
    expect(rank).toEqual([...rank].sort((a, b) => a - b));
    const ids = spine.nodes.map((n) => n.id);
    expect(ids).not.toEqual([...ids].sort());
  });
});

describe("a malformed shard", () => {
  it("names the offending file, not the merged position", () => {
    const monolith = join(root, "spine.json");
    const dir = join(root, "spine.d");
    mkdirSync(dir);
    writeJson(join(dir, "0010-ALPHA.json"), { id: "ALPHA" });
    writeFileSync(join(dir, "0020-BETA.json"), '{ "id": "BETA", }\n', "utf8");
    writeJson(join(dir, "0030-GAMMA.json"), { id: "GAMMA" });

    // "Unexpected token } at position 412" against a merged read of three files
    // tells the reader nothing they can open an editor on. The filename does.
    expect(() => readMaybeSharded(monolith, mergeItems)).toThrow(
      /shard '0020-BETA\.json'.*malformed JSON/s,
    );
  });
});
