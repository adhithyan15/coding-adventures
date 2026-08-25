// The GROUPED split (HL21 §5.3): several parallel arrays partitioned into one
// file per language, rather than one file per element.
//
// ---------------------------------------------------------------------------
// Why this suite exists for a plan that is not enabled
// ---------------------------------------------------------------------------
//
// `core/book-generation.json` is the only ledger this projection is for, and it
// is NOT in `SHARD_PLANS` — it cannot round-trip byte-exactly, for a reason
// that is a decision rather than a bug (see the last describe block). The
// machinery is finished and proved here against fixtures and against the real
// ledger's DATA, so that enabling it later is one line plus one re-indent
// commit, not a rebuild.
//
// The alternative was to leave the projection unwritten until the blocker
// cleared, which would have meant re-deriving all of this — including the
// measurement that the spec's ORIGINAL blocker had already resolved itself —
// from scratch, months later, by someone who had not measured it.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { BOOK_GENERATION_PLAN, SHARD_PLANS, shardContents } from "../src/shard-cli.js";
import { defaultCurriculumRoot } from "../src/loader.js";
import { BOOK_GENERATION_GROUPED_KEYS, mergeGroupedShards } from "../src/shard.js";

const root = defaultCurriculumRoot();
const codeUnit = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);
const serialize = (v: unknown) => `${JSON.stringify(v, null, 2)}\n`;

/** Read the real ledger, shard it in memory, and fold it straight back. */
function roundTrip(document: Record<string, unknown>): {
  names: string[];
  rebuilt: string;
  meta: Record<string, unknown>;
} {
  const contents = shardContents(document, BOOK_GENERATION_PLAN);
  const names = [...contents.keys()].sort(codeUnit);
  const shards = names.map((name) => ({
    name,
    path: name,
    value: JSON.parse(contents.get(name)!) as unknown,
  }));
  return {
    names,
    rebuilt: serialize(mergeGroupedShards(shards, BOOK_GENERATION_GROUPED_KEYS)),
    meta: JSON.parse(contents.get("_meta.json")!) as Record<string, unknown>,
  };
}

const committed = readFileSync(join(root, "core", "book-generation.json"), "utf8");
const document = JSON.parse(committed) as Record<string, unknown>;

describe("the grouped split of the real core/book-generation.json", () => {
  it("writes one file per language plus _meta.json", () => {
    const { names } = roundTrip(document);
    expect(names[0]).toBe("_meta.json");
    expect(names).toHaveLength(24);
    expect(names).toContain("spanish.json");
    // Sorted filename order IS alphabetical language order, which is the
    // property that makes the rebuild reproduce authored order without any
    // ordinal prefix.
    expect(names.slice(1)).toEqual([...names.slice(1)].sort(codeUnit));
  });

  it("keeps version, sourceBaseUrl and scriptSets in _meta.json", () => {
    // `scriptSets` is keyed by SCRIPT SET, not by language, so it is genuinely
    // shared and has no per-language home. Confirmed rather than assumed: no
    // element of it carries a `language`.
    const { meta } = roundTrip(document);
    expect(Object.keys(meta)).toEqual(["version", "sourceBaseUrl", "scriptSets"]);
    // and no `_keys`, because the six grouped arrays are already a suffix.
    expect(Object.hasOwn(meta, "_keys")).toBe(false);
  });

  it("loses no element from any of the six arrays", () => {
    const { rebuilt } = roundTrip(document);
    const after = JSON.parse(rebuilt) as Record<string, unknown[]>;
    for (const key of BOOK_GENERATION_GROUPED_KEYS) {
      expect(after[key], key).toHaveLength((document[key] as unknown[]).length);
    }
    // A FLOOR, not an equality. `targets` grows with every tranche — it was 949
    // when HL21 was written, 1,007 when this projection was built, and 1,014 by
    // the time the suite next ran. Pinning the exact number would make every
    // content PR edit this line, which teaches people to edit it.
    expect(after.targets.length).toBeGreaterThanOrEqual(1007);
  });

  it("reproduces the canonical serialization byte for byte", () => {
    // The real assertion: the projection itself is lossless and order-preserving.
    const { rebuilt } = roundTrip(document);
    expect(rebuilt).toBe(serialize(document));
  });

  it("preserves the data exactly", () => {
    const { rebuilt } = roundTrip(document);
    expect(JSON.parse(rebuilt)).toEqual(document);
  });
});

describe("the spec's recorded blocker has resolved itself", () => {
  it("targets is contiguous by language — 23 runs for 23 languages", () => {
    // HL21 §5.3 measured 27 runs for 23 languages at 949 entries and concluded a
    // one-time re-sort was needed before this ledger could be sharded losslessly.
    // At 1,007 entries it is 23 for 23: the split runs for hindi, kannada,
    // spanish and telugu closed as later tranches inserted into them.
    //
    // Pinned as a test because it is the thing that would silently re-break: a
    // tranche appending to the END of `targets` rather than into its language's
    // run reopens the blocker, and this says so immediately instead of at the
    // moment somebody tries to enable the plan.
    for (const key of BOOK_GENERATION_GROUPED_KEYS) {
      const seen: string[] = [];
      let previous: string | undefined;
      for (const element of document[key] as { language: string }[]) {
        if (element.language !== previous) seen.push(element.language);
        previous = element.language;
      }
      expect(new Set(seen).size, `${key} is not contiguous by language`).toBe(seen.length);
      // and the runs are in alphabetical order, which is sorted-filename order
      expect(seen, `${key} runs are not alphabetical`).toEqual([...seen].sort(codeUnit));
    }
  });
});

describe("the grouped merge validates its _meta.json like its siblings do", () => {
  // Three guards `mergeSectionedShards` and `mergeMetaAndList` had and this one
  // did not. Without the shape check a `_meta.json` holding `["a","b"]` spreads
  // to `{0:"a",1:"b"}` and those fabricated numeric keys flow into the rebuilt
  // document instead of raising anything.
  const keys = ["targets"];
  const shard = { name: "spanish.json", path: "x", value: { targets: [] } };

  it.each([
    ["an array", ["a", "b"]],
    ["a string", "abc"],
    ["null", null],
  ])("refuses a _meta.json that is %s", (_label, value) => {
    expect(() =>
      mergeGroupedShards([{ name: "_meta.json", path: "m", value }, shard], keys),
    ).toThrow(/must be a JSON object/);
  });

  it("refuses a _meta.json that carries a grouped array", () => {
    // Otherwise merge order silently decides whether the meta copy or the
    // shards win, and one of those is always a stale duplicate.
    expect(() =>
      mergeGroupedShards(
        [{ name: "_meta.json", path: "m", value: { version: 1, targets: [] } }, shard],
        keys,
      ),
    ).toThrow(/must not carry 'targets'/);
  });

  it("refuses a shard in a subdirectory, which would duplicate a language", () => {
    // A grouped ledger is flat. `book-generation.d/sub/spanish.json` would
    // otherwise be consumed as a second Spanish slice, silently duplicating
    // every one of that language's entries into all six arrays.
    expect(() =>
      mergeGroupedShards(
        [
          { name: "_meta.json", path: "m", value: { version: 1 } },
          { name: "sub/spanish.json", path: "x", value: { targets: [] } },
        ],
        keys,
      ),
    ).toThrow(/no subdirectories/);
  });
});

describe("why this plan is not in SHARD_PLANS", () => {
  it("is absent, deliberately", () => {
    expect(SHARD_PLANS.some((p) => p.path === "core/book-generation.json")).toBe(false);
  });

  it("because the COMMITTED file does not round-trip, by 74 lines of whitespace", () => {
    // Twelve `marwadi` entries in `targets` are indented two spaces deeper than
    // the canonical form — a hand-merge artifact at lines 2911-2984.
    //
    // This test is the blocker, stated as an executable fact rather than a
    // comment. It FAILS THE DAY SOMEBODY RE-INDENTS THE FILE, which is exactly
    // when the plan should be enabled — so the fix and the signal to act on it
    // arrive together.
    const canonical = serialize(document);
    expect(committed).not.toBe(canonical);

    const committedLines = committed.split("\n");
    const canonicalLines = canonical.split("\n");
    expect(committedLines).toHaveLength(canonicalLines.length);

    const differing = committedLines
      .map((line, i) => (line === canonicalLines[i] ? -1 : i))
      .filter((i) => i >= 0);
    expect(differing).toHaveLength(74);

    // Every difference is leading whitespace only — the data is untouched.
    for (const i of differing) {
      expect(committedLines[i]!.trim()).toBe(canonicalLines[i]!.trim());
    }
    expect(JSON.parse(committed)).toEqual(JSON.parse(canonical));
  });
});
