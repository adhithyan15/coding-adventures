// The `<track>/curriculum.d/` migration (HL21 §5.2), tested against the REAL
// committed corpus rather than a fixture.
//
// ---------------------------------------------------------------------------
// Why this ledger mattered most
// ---------------------------------------------------------------------------
//
// `spine` is the single worst conflict point in the corpus. Every content
// tranche in every track appends to `spine[<node>].segments`, and there are only
// 33 nodes for 23 tracks' worth of authors to collide on. One file per node is
// the whole prize: two tranches touching two different nodes never meet.
//
// ---------------------------------------------------------------------------
// The trap this ledger sprang, which the spec said it would not
// ---------------------------------------------------------------------------
//
// HL21 §5.2 argued that `spine` is keyed by node id and so "needs no ordinal: an
// object has no meaningful order". That is true of JSON semantics and false of
// this ledger, in three ways that compound:
//
//   * `JSON.stringify` emits object keys in INSERTION order, so merging
//     `<NODE-ID>.json` shards in sorted filename order rewrites the key order
//     and the generated monolith stops round-tripping;
//   * no track has its spine keys in sorted order — all 23 checked;
//   * and the order is not arbitrary. Every track lists its spine keys in
//     exactly `core/spine.d/`'s ladder order, pre-A1 -> C2. Re-sorting them
//     would scramble the ladder, silently, in 23 files at once.
//
// So `spine` carries ordinals like everything else, and the tests below assert
// both halves: that sorted shard order reproduces authored order, and that
// plain `<ID>.json` names would NOT have.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { SHARD_PLANS, runShardCli, shardContents, unshardContents } from "../src/shard-cli.js";
import { defaultCurriculumRoot, loadCurriculumSpine, loadLanguageCurricula } from "../src/loader.js";
import { listShardNames, mergeSectionedShards } from "../src/shard.js";

const root = defaultCurriculumRoot();
const CURRICULUM_PLANS = SHARD_PLANS.filter((plan) => plan.path.endsWith("/curriculum.json"));

/**
 * The one track left on its monolith.
 *
 * `marwadi/curriculum.json` writes its `lessons` arrays inline on one line, so
 * `JSON.stringify(…, null, 2)` does not reproduce its bytes. Data identical,
 * bytes not — reported rather than reformatted, per HL21 §8.9.
 */
const UNMIGRATED = "marwadi";

const codeUnit = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);

describe("the curriculum.d migration covers what it claims to", () => {
  it("sharded 22 tracks and left the one that does not round-trip", () => {
    expect(CURRICULUM_PLANS).toHaveLength(22);
    expect(CURRICULUM_PLANS.some((p) => p.path === `${UNMIGRATED}/curriculum.json`)).toBe(false);
    expect(existsSync(join(root, UNMIGRATED, "curriculum.json"))).toBe(true);
    expect(existsSync(join(root, UNMIGRATED, "curriculum.d"))).toBe(false);
  });

  it("passes --check for every sharded ledger", () => {
    expect(runShardCli(["--check"])).toBe(0);
  });

  it.each(CURRICULUM_PLANS.map((p) => p.path))(
    "%s: the generated monolith matches the shards byte for byte",
    (path) => {
      const plan = SHARD_PLANS.find((p) => p.path === path)!;
      expect(readFileSync(join(root, path), "utf8")).toBe(unshardContents(root, plan));
    },
  );

  it.each(CURRICULUM_PLANS.map((p) => p.path))("%s: has all three section dirs", (path) => {
    const dir = join(root, `${path.slice(0, -".json".length)}.d`);
    for (const section of ["path", "spine", "extensions"]) {
      expect(existsSync(join(dir, section)), `${path}: missing ${section}/`).toBe(true);
    }
    expect(existsSync(join(dir, "_meta.json"))).toBe(true);
  });
});

describe("spine: sorted shard order reproduces the authored ladder", () => {
  // The authored ladder, from the shared spine that every track mirrors.
  const ladder = loadCurriculumSpine(root).nodes.map((node) => node.id);

  it.each(CURRICULUM_PLANS.map((p) => p.path))("%s", (path) => {
    const plan = SHARD_PLANS.find((p) => p.path === path)!;
    const rebuilt = JSON.parse(unshardContents(root, plan)) as {
      spine: Record<string, unknown>;
    };
    const keys = Object.keys(rebuilt.spine);

    // Rebuilt key order is the ladder order, not alphabetical.
    expect(keys).toEqual(ladder);
    expect(keys).not.toEqual([...keys].sort(codeUnit));
  });

  it.each(CURRICULUM_PLANS.map((p) => p.path))(
    "%s: plain <ID>.json names would have re-sorted the ladder",
    (path) => {
      // THE test that fails if someone decides the ordinal prefix on `spine/` is
      // noise — which is exactly what HL21 §5.2 proposed. Renaming the shards to
      // `SPINE-MEET-GREET.json` makes sorted order alphabetical, and the ladder
      // is not alphabetical in any track.
      const plan = SHARD_PLANS.find((p) => p.path === path)!;
      const keys = Object.keys(
        (JSON.parse(unshardContents(root, plan)) as { spine: Record<string, unknown> }).spine,
      );
      expect([...keys].sort(codeUnit)).not.toEqual(keys);
    },
  );
});

describe("path and extensions", () => {
  it.each(CURRICULUM_PLANS.map((p) => p.path))("%s: ids are unique and filename-safe", (path) => {
    const plan = SHARD_PLANS.find((p) => p.path === path)!;
    const rebuilt = JSON.parse(unshardContents(root, plan)) as {
      path: { id: string }[];
      extensions: { id: string }[];
    };
    for (const key of ["path", "extensions"] as const) {
      const ids = rebuilt[key].map((e) => e.id);
      expect(new Set(ids).size, `${path}: duplicate ${key} id`).toBe(ids.length);
      for (const id of ids) expect(id).toMatch(/^[A-Z][A-Z0-9-]*$/);
    }
  });

  it("would be re-sorted in most tracks without their ordinals", () => {
    // Corpus-wide rather than per-track, because the claim is NOT true of every
    // track and pretending otherwise made this suite fail honestly on its first
    // run. `japanese` and `urdu` happen to have both lists already in sorted
    // order — they are small and were authored in one pass — so those two alone
    // would survive plain `<ID>.json` names.
    //
    // That is exactly why the check belongs at corpus level. "Ordinals are
    // unnecessary" has to be false for the CONVENTION to be justified, and a
    // convention that holds for 20 of 22 tracks must apply to all 22: the two
    // coincidences are one authored id away from joining the other twenty, and
    // nothing would announce it.
    const broken: string[] = [];
    for (const plan of CURRICULUM_PLANS) {
      const rebuilt = JSON.parse(unshardContents(root, plan)) as {
        path: { id: string }[];
        extensions: { id: string }[];
      };
      const sortedInOrder = (ids: string[]) =>
        [...ids].sort(codeUnit).every((v, i) => v === ids[i]);
      const pathIds = rebuilt.path.map((e) => e.id);
      const extIds = rebuilt.extensions.map((e) => e.id);
      if (!sortedInOrder(pathIds) || !sortedInOrder(extIds)) {
        broken.push(plan.path.slice(0, plan.path.indexOf("/")));
      }
    }
    expect(broken.length).toBeGreaterThanOrEqual(20);
    // Spanish is the spec's worked example: authored `ES-PATH-004` meets sorted
    // `ES-PATH-003-CASA`, because a bare prefix sorts before the same prefix
    // extended.
    expect(broken).toContain("spanish");
  });

  it("spanish diverges at the exact index the spec recorded", () => {
    const plan = SHARD_PLANS.find((p) => p.path === "spanish/curriculum.json")!;
    const ids = (JSON.parse(unshardContents(root, plan)) as { path: { id: string }[] }).path.map(
      (e) => e.id,
    );
    const sorted = [...ids].sort(codeUnit);
    const first = sorted.findIndex((v, i) => v !== ids[i]);
    expect(first).toBe(3);
    expect(ids[3]).toBe("ES-PATH-004");
    expect(sorted[3]).toBe("ES-PATH-003-CASA");
  });
});

describe("the shards are exactly what re-sharding the rebuild would produce", () => {
  it.each(CURRICULUM_PLANS.map((p) => p.path))("%s", (path) => {
    const plan = SHARD_PLANS.find((p) => p.path === path)!;
    const rebuilt = JSON.parse(unshardContents(root, plan)) as Record<string, unknown>;
    const expected = shardContents(rebuilt, plan);
    expect(listShardNames(join(root, path)).sort(codeUnit)).toEqual(
      [...expected.keys()].sort(codeUnit),
    );
    const dir = join(root, `${path.slice(0, -".json".length)}.d`);
    for (const [name, body] of expected) {
      expect(readFileSync(join(dir, name), "utf8"), `${path}: ${name}`).toBe(body);
    }
  });
});

describe("_meta.json records the key order only where it is needed", () => {
  it("spanish records it, because conceptAliases follows the sharded keys", () => {
    const meta = JSON.parse(
      readFileSync(join(root, "spanish", "curriculum.d", "_meta.json"), "utf8"),
    ) as { _keys?: string[] };
    expect(meta._keys).toEqual([
      "version",
      "language",
      "path",
      "spine",
      "extensions",
      "conceptAliases",
    ]);
  });

  it("the other tracks do not, because extensions is already last", () => {
    // Not a detail: emitting `_keys` unconditionally would add a line to every
    // `_meta.json` in the corpus that nothing reads, and the rule that avoids it
    // is the same one that kept the 21 chapters/spine shard sets untouched.
    for (const plan of CURRICULUM_PLANS) {
      if (plan.path === "spanish/curriculum.json") continue;
      const dir = join(root, `${plan.path.slice(0, -".json".length)}.d`);
      const meta = JSON.parse(readFileSync(join(dir, "_meta.json"), "utf8")) as
        Record<string, unknown>;
      expect(Object.hasOwn(meta, "_keys"), `${plan.path} should not need _keys`).toBe(false);
    }
  });
});

describe("keys that come from a FILENAME are validated, not trusted", () => {
  // The read path is a different trust boundary from the write path. `shard-cli`
  // validates ids that come out of authored JSON; these come out of a filename
  // on disk, which a pull request chooses and no write-side check constrains.
  //
  // This was a real regression for one revision: moving the object-section merge
  // into `shard.ts` dropped the id check, so `0010-__proto__.json` produced
  // `value["__proto__"] = …` — which invokes the setter rather than creating a
  // key. The node's realization vanished, the object's prototype changed, and
  // `Object.hasOwn` could not even see it to report the collision.
  const meta = { name: "_meta.json", path: "_meta.json", value: { version: 1 } };
  const sections = [{ key: "spine", dir: "spine", kind: "object" as const }];

  it.each(["__proto__", "constructor", "prototype"])(
    "refuses a shard named 0010-%s.json",
    (dangerous) => {
      expect(() =>
        mergeSectionedShards(
          [meta, { name: `spine/0010-${dangerous}.json`, path: "x", value: { segments: [] } }],
          sections,
        ),
      ).toThrow();
    },
  );

  it("refuses a shard whose filename carries no id", () => {
    expect(() =>
      mergeSectionedShards([meta, { name: "spine/0010.json", path: "x", value: {} }], sections),
    ).toThrow(/no usable id in its filename/);
  });

  it("refuses a lowercase id, which SAFE_ID does not admit", () => {
    expect(() =>
      mergeSectionedShards(
        [meta, { name: "spine/0010-spine-meet.json", path: "x", value: {} }],
        sections,
      ),
    ).toThrow(/no usable id/);
  });

  it("leaves Object.prototype untouched after a refusal", () => {
    try {
      mergeSectionedShards(
        [meta, { name: "spine/0010-__proto__.json", path: "x", value: { polluted: true } }],
        sections,
      );
    } catch {
      /* expected */
    }
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });
});

describe("_keys is attacker-controlled and is treated that way", () => {
  const sections = [{ key: "nodes" }];
  const shard = { name: "0010-ALPHA.json", path: "x", value: { id: "ALPHA" } };

  it.each(["__proto__", "constructor", "prototype"])("refuses _keys naming %s", (dangerous) => {
    expect(() =>
      mergeSectionedShards(
        [
          { name: "_meta.json", path: "m", value: { _keys: ["nodes", dangerous] } },
          shard,
        ],
        sections,
      ),
    ).toThrow(/must not name/);
  });

  it("refuses a repeated key, which would silently drop the first value", () => {
    expect(() =>
      mergeSectionedShards(
        [
          { name: "_meta.json", path: "m", value: { version: 1, _keys: ["version", "version"] } },
          shard,
        ],
        sections,
      ),
    ).toThrow(/more than once/);
  });

  it("refuses a non-string key", () => {
    expect(() =>
      mergeSectionedShards(
        [{ name: "_meta.json", path: "m", value: { _keys: [1] } }, shard],
        sections,
      ),
    ).toThrow(/must be a non-empty string/);
  });
});

describe("a shard that belongs to no section is refused, not discarded", () => {
  // The silence that let a poisoned shard reach the loader while `--check`
  // stayed green: sections take their shards by directory prefix, so a file
  // matching no prefix was read, parsed, and then dropped. The rebuild simply
  // omitted it, so the generated monolith still matched the committed one and
  // nothing anywhere said a word.
  const meta = { name: "_meta.json", path: "_meta.json", value: { version: 1 } };
  const sections = [{ key: "path", dir: "path" }];

  it("refuses a stray shard at the top level", () => {
    expect(() =>
      mergeSectionedShards([meta, { name: "0010-STRAY.json", path: "x", value: {} }], sections),
    ).toThrow(/belongs to no section/);
  });

  it("refuses a shard under a directory no section names", () => {
    expect(() =>
      mergeSectionedShards(
        [meta, { name: "unknown/0010-X.json", path: "x", value: {} }],
        sections,
      ),
    ).toThrow(/belongs to no section/);
  });
});

describe("_keys must be a permutation, not an arbitrary subset", () => {
  // The rebuild emits only the keys `_keys` names, so a short list silently
  // truncates the document. `--check` catches that today only by luck — every
  // plan is `monolith: "generated"`, so the rebuild is byte-compared against a
  // committed file. A `"removed"` ledger has no such file, which makes this a
  // trap laid for the next migration rather than a hypothetical.
  const shard = { name: "0010-ALPHA.json", path: "x", value: { id: "ALPHA" } };

  it("refuses _keys that omits a document key", () => {
    expect(() =>
      mergeSectionedShards(
        [
          {
            name: "_meta.json",
            path: "m",
            value: { version: 1, language: "toy", _keys: ["version", "nodes"] },
          },
          shard,
        ],
        [{ key: "nodes" }],
      ),
    ).toThrow(/omits 'language'/);
  });

  it("accepts _keys that names every key exactly once", () => {
    const merged = mergeSectionedShards(
      [
        {
          name: "_meta.json",
          path: "m",
          value: { version: 1, language: "toy", _keys: ["version", "nodes", "language"] },
        },
        shard,
      ],
      [{ key: "nodes" }],
    );
    // and the recorded order is the emitted order
    expect(Object.keys(merged)).toEqual(["version", "nodes", "language"]);
  });
});

describe("the loader sees the sharded curricula", () => {
  const loaded = loadLanguageCurricula(root);

  it("still returns all twenty-three tracks", () => {
    // The `existsSync` canary again: `loadLanguageCurricula` skips a track with
    // no `curriculum.json`, and a migrated track may not have one. Without the
    // `isSharded` half, tracks would vanish from every gate in silence.
    expect(loaded).toHaveLength(23);
  });

  it("reads the same document the generated monolith holds", () => {
    // The loader and `--unshard` must not have two ideas of what these files
    // mean; `--check` only compares the monolith against `unshardContents`, so a
    // divergent loader would go unreported.
    for (const plan of CURRICULUM_PLANS) {
      const language = plan.path.slice(0, plan.path.indexOf("/"));
      const fromLoader = loaded.find((c) => c.language === language);
      expect(fromLoader, `${language} missing`).toBeDefined();
      expect(`${JSON.stringify(fromLoader, null, 2)}\n`).toBe(unshardContents(root, plan));
    }
  });

  it("keeps every track's spine on the shared ladder", () => {
    const ladder = loadCurriculumSpine(root).nodes.map((n) => n.id);
    for (const curriculum of loaded) {
      expect(Object.keys(curriculum.spine), `${curriculum.language}`).toEqual(ladder);
    }
  });
});
