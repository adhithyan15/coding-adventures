import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
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

describe("shards cannot leave the checkout", () => {
  // Creating a symlink needs Developer Mode or elevation on Windows, and gets
  // EPERM without them. A Windows JUNCTION needs neither and `lstat` reports it
  // as a symbolic link all the same, so the directory case — the one that can
  // actually read outside the checkout — still gets exercised there. The file
  // case has no junction equivalent and skips; the guard it covers still
  // compiles and still runs on every CI Linux job.
  function trySymlink(target: string, path: string, type: "dir" | "file"): boolean {
    for (const attempt of type === "dir" ? ([type, "junction"] as const) : ([type] as const)) {
      try {
        symlinkSync(target, path, attempt);
        return true;
      } catch {
        // Try the next flavour, then give up and let the caller skip.
      }
    }
    return false;
  }

  it("refuses a symlinked X.d rather than following it", (ctx) => {
    // git tracks symlinks as first-class objects, so `core/spine.d -> ~/.docker`
    // is a thing a pull request can contain. `statSync` would follow it and merge
    // whatever JSON it found into the curriculum; `lstatSync` does not.
    const outside = join(root, "outside");
    mkdirSync(outside);
    writeJson(join(outside, "_meta.json"), { version: 666 });
    writeJson(join(outside, "0010-STOLEN.json"), { id: "STOLEN" });

    const monolith = join(root, "inside", "spine.json");
    mkdirSync(join(root, "inside"));
    writeJson(monolith, { version: 1, items: [] });
    if (!trySymlink(outside, join(root, "inside", "spine.d"), "dir")) ctx.skip();

    expect(() => isSharded(monolith)).toThrow(/is a symbolic link/);
    expect(() => readMaybeSharded(monolith, mergeItems)).toThrow(/is a symbolic link/);
  });

  it("refuses a symlinked shard file rather than silently dropping it", (ctx) => {
    // `Dirent.isFile()` is false for a symlink, so without this guard the shard
    // would vanish from the merge without a word — and a ledger that is quietly
    // missing an element still looks complete.
    const secret = join(root, "secret.json");
    writeJson(secret, { id: "SECRET" });
    const dir = join(root, "spine.d");
    mkdirSync(dir);
    writeJson(join(dir, "_meta.json"), { version: 1 });
    if (!trySymlink(secret, join(dir, "0010-LINK.json"), "file")) ctx.skip();

    expect(() => listShardNames(join(root, "spine.json"))).toThrow(/is a symbolic link/);
  });

  it("answers 'not sharded' instead of throwing when X.d simply is not there", () => {
    expect(isSharded(join(root, "nothing-here", "spine.json"))).toBe(false);
  });
});

describe("hostile shard contents", () => {
  it("does not pollute Object.prototype, and refuses the key outright", () => {
    const monolith = join(root, "spine.json");
    const dir = join(root, "spine.d");
    mkdirSync(dir);
    // Written as raw text: `JSON.stringify({__proto__: ...})` would not emit the
    // key at all, so building this fixture through an object literal would test
    // nothing.
    writeFileSync(
      join(dir, "_meta.json"),
      '{ "version": 1, "__proto__": { "polluted": "yes" } }\n',
      "utf8",
    );
    writeJson(join(dir, "0010-ALPHA.json"), { id: "ALPHA" });

    expect(() => readMaybeSharded(monolith, (s) => mergeMetaAndList(s, "nodes"))).toThrow(
      /must not carry '__proto__'/,
    );
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.prototype).not.toHaveProperty("polluted");
  });

  it("refuses __proto__ in an element shard, not just in _meta", () => {
    const monolith = join(root, "spine.json");
    const dir = join(root, "spine.d");
    mkdirSync(dir);
    writeJson(join(dir, "_meta.json"), { version: 1 });
    writeFileSync(
      join(dir, "0010-ALPHA.json"),
      '{ "id": "ALPHA", "constructor": { "prototype": {} } }\n',
      "utf8",
    );

    expect(() => readMaybeSharded(monolith, (s) => mergeMetaAndList(s, "nodes"))).toThrow(
      /0010-ALPHA\.json.*must not carry 'constructor'/s,
    );
  });

  it("refuses __proto__ in the monolith too", () => {
    const monolith = join(root, "spine.json");
    writeFileSync(monolith, '{ "version": 1, "__proto__": { "polluted": "yes" } }\n', "utf8");

    expect(() => readMaybeSharded(monolith, mergeItems)).toThrow(/must not carry '__proto__'/);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });

  it("keeps the offending file's bytes out of the error message", () => {
    // V8 quotes the bytes it choked on straight into the message. Shards are repo
    // files and symlinks out of the tree are refused, so this is defence in
    // depth — but `--check` runs in CI, and CI logs are read far more widely
    // than the repo is.
    const monolith = join(root, "spine.json");
    const dir = join(root, "spine.d");
    mkdirSync(dir);
    writeFileSync(join(dir, "0010-ALPHA.json"), 'AWS_SECRET_ACCESS_KEY=hunter2\n', "utf8");

    let message = "";
    try {
      readMaybeSharded(monolith, mergeItems);
    } catch (error) {
      message = (error as Error).message;
    }
    expect(message).toMatch(/0010-ALPHA\.json/);
    expect(message).toMatch(/malformed JSON/);
    expect(message).not.toMatch(/hunter2|AWS_SECRET/);
  });

  it("elides the bytes even when the file itself contains a quote", () => {
    // The regression that killed the first version of this filter. V8 splices
    // the offending bytes in RAW, so a `"` in the file mis-pairs the delimiters
    // and a quote-matching elision leaves the tail of the secret behind:
    // `ab"cd AKIA…` produced `Unexpected token 'a', "ab"cd AKIA"...`, of which a
    // naive filter elided only `"ab"`.
    const monolith = join(root, "spine.json");
    const dir = join(root, "spine.d");
    mkdirSync(dir);
    writeFileSync(join(dir, "0010-ALPHA.json"), 'ab"cd AKIAIOSFODNN7EXAMPLE\n', "utf8");

    let message = "";
    try {
      readMaybeSharded(monolith, mergeItems);
    } catch (error) {
      message = (error as Error).message;
    }
    expect(message).toMatch(/0010-ALPHA\.json/);
    expect(message).not.toMatch(/AKIA/);
  });

  it("keeps the part of a parse error that helps", () => {
    // Elision must not reduce every failure to "malformed JSON". The forms that
    // carry no snippet name the position, and that is what a reader opens the
    // file to.
    const monolith = join(root, "spine.json");
    const dir = join(root, "spine.d");
    mkdirSync(dir);
    writeFileSync(join(dir, "0010-ALPHA.json"), '{ "id": "unterminated\n', "utf8");

    let message = "";
    try {
      readMaybeSharded(monolith, mergeItems);
    } catch (error) {
      message = (error as Error).message;
    }
    expect(message).toMatch(/position|Unterminated|JSON/i);
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
