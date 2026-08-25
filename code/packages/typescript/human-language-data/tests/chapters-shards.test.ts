// The `<track>/chapters.d/` migration (HL21 §5.1), tested against the REAL
// committed corpus rather than a fixture.
//
// ---------------------------------------------------------------------------
// What these tests are actually guarding
// ---------------------------------------------------------------------------
//
// The migration itself was proved byte-exact at the moment it landed: for each
// of the twenty tracks, folding the shards back together reproduced the
// committed `chapters.json` with an unchanged SHA-256. Those hashes are in the
// CHANGELOG and the commit message, where a one-time proof belongs — pinning
// them in a test would mean every future chapter append had to edit a hash to
// stay green, which teaches people to edit hashes to stay green.
//
// What survives as a permanent gate is the INVARIANT that made the round trip
// exact, and it is a different thing from the snapshot:
//
//   * sorted filename order reproduces authored chapter order, and
//   * the shard set is exactly what re-sharding the rebuild would produce.
//
// Both hold for a corpus that has grown by a thousand chapters, and both fail
// the moment somebody breaks the naming scheme — which is the failure mode
// worth having a test for, because it is silent.

import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { SHARD_PLANS, runShardCli, shardContents, unshardContents, unshardLedger } from "../src/shard-cli.js";
import { defaultCurriculumRoot, loadTrackChapters } from "../src/loader.js";
import { listShardNames } from "../src/shard.js";

const root = defaultCurriculumRoot();

/** Every plan the migration moved into a `.d/` and deleted the monolith of. */
const CHAPTER_PLANS = SHARD_PLANS.filter((plan) => plan.path.endsWith("/chapters.json"));

/**
 * The three tracks deliberately left on their monoliths.
 *
 * Their committed `chapters.json` is hand-formatted with inline one-line arrays,
 * which `JSON.stringify(…, null, 2)` expands. The DATA is identical either way;
 * the BYTES are not. HL21's rule is that a ledger which does not round-trip
 * byte-exactly gets REPORTED rather than quietly reformatted into agreement
 * with the serialiser, so these wait for that decision instead of being swept
 * into this commit.
 */
const UNMIGRATED = ["french", "japanese", "marwadi"];

describe("the chapters.d migration covers what it claims to", () => {
  it("sharded twenty tracks and left exactly the three that do not round-trip", () => {
    expect(CHAPTER_PLANS).toHaveLength(20);
    for (const track of UNMIGRATED) {
      expect(CHAPTER_PLANS.some((plan) => plan.path === `${track}/chapters.json`)).toBe(false);
    }
  });

  it("passes --check for every sharded ledger", () => {
    expect(runShardCli(["--check"])).toBe(0);
  });

  it.each(CHAPTER_PLANS.map((plan) => plan.path))(
    "%s: the monolith survives, and matches the shards byte for byte",
    (path) => {
      // The monolith is kept as a GENERATED artifact rather than deleted,
      // because language-ladder reads these ledgers through `import.meta.glob`
      // and a glob's key table is eager: sharding took it from 23 entries to
      // ~1,020 and grew the app's eager chunk by 191 kB, through a hard budget.
      // See the note on `chaptersPlan` for why the obvious dodges are worse.
      //
      // A derived file nothing verifies is worse than no file, so the bytes are
      // asserted here as well as by `--check`.
      expect(existsSync(join(root, path))).toBe(true);
      const plan = SHARD_PLANS.find((p) => p.path === path)!;
      expect(readFileSync(join(root, path), "utf8")).toBe(unshardContents(root, plan));
    },
  );

  it.each(UNMIGRATED)("%s has no shard directory at all", (track) => {
    expect(existsSync(join(root, track, "chapters.json"))).toBe(true);
    expect(existsSync(join(root, track, "chapters.d"))).toBe(false);
  });
});

describe("sorted shard order reproduces authored chapter order", () => {
  // THE trap, and the reason the shard filename is zero-padded rather than the
  // bare chapter number. See the fixture test at the bottom for the version that
  // fails loudly if anyone "simplifies" the naming.
  it.each(CHAPTER_PLANS.map((plan) => plan.path))("%s", (path) => {
    const rebuilt = JSON.parse(unshardContents(root, SHARD_PLANS.find((p) => p.path === path)!)) as {
      chapters: { chapter: number }[];
    };
    const numbers = rebuilt.chapters.map((chapter) => chapter.chapter);

    // Authored order is ascending, and sorted filename order produced it.
    expect(numbers).toEqual([...numbers].sort((a, b) => a - b));

    // Every shard is named for the chapter it holds, zero-padded to four.
    const names = listShardNames(join(root, path)).filter((name) => name !== "_meta.json");
    expect(names).toEqual(numbers.map((n) => `${String(n).padStart(4, "0")}.json`));
  });

  it.each(CHAPTER_PLANS.map((plan) => plan.path))(
    "%s: UNPADDED names would have scrambled it",
    (path) => {
      // The assertion that makes the padding load-bearing rather than decorative.
      // If someone renames the shards to `7.json`, this stops being true and the
      // test above starts failing — but this one says WHY in one line.
      //
      // It holds for every track in the corpus, including the smallest: with
      // eleven chapters, "10.json" and "11.json" both sort before "2.json".
      const numbers = loadTrackChapters(root)
        .find((track) => path.startsWith(`${track.language}/`))!
        .chapters.map((chapter) => chapter.chapter);
      const unpadded = numbers.map((n) => `${n}.json`);
      expect([...unpadded].sort()).not.toEqual(unpadded);
    },
  );
});

describe("the shards are exactly what re-sharding the rebuild would produce", () => {
  // The closure property `--check` rests on: shard(unshard(shards)) === shards.
  // A shard dropped by a bad merge, or an extra one nothing produces, shows up
  // here as a set difference rather than as a chapter quietly missing from a book.
  it.each(CHAPTER_PLANS.map((plan) => plan.path))("%s", (path) => {
    const plan = SHARD_PLANS.find((p) => p.path === path)!;
    const rebuilt = JSON.parse(unshardContents(root, plan)) as Record<string, unknown>;
    const expected = shardContents(rebuilt, plan);
    expect(listShardNames(join(root, path)).sort()).toEqual([...expected.keys()].sort());
    // and byte-for-byte, not merely name-for-name
    const dir = join(root, `${path.slice(0, -".json".length)}.d`);
    for (const [name, body] of expected) {
      expect(readFileSync(join(dir, name), "utf8")).toBe(body);
    }
  });
});

describe("the loader sees the sharded tracks", () => {
  const loaded = loadTrackChapters(root);

  it("still returns all twenty-three tracks", () => {
    // The canary for the `existsSync` bug this migration could have introduced:
    // `loadTrackChapters` skips a track with no `chapters.json`, and twenty of
    // them no longer have one. If the shard branch were missing, those tracks
    // would vanish from the corpus SILENTLY — the gap report treats an absent
    // ledger as honest un-authored debt, so every gate would go green on a
    // corpus a thousand chapters smaller.
    expect(loaded).toHaveLength(23);
  });

  it("reads the same chapter counts the shards hold", () => {
    for (const plan of CHAPTER_PLANS) {
      const language = plan.path.slice(0, plan.path.indexOf("/"));
      const track = loaded.find((entry) => entry.language === language);
      expect(track, `${language} missing from loadTrackChapters`).toBeDefined();
      const shards = listShardNames(join(root, plan.path)).filter((n) => n !== "_meta.json");
      expect(track!.chapters).toHaveLength(shards.length);
    }
  });

  it("keeps a FLOOR under the corpus, which may grow but not shrink", () => {
    // A floor, not an equality: content lands daily and this must not be a line
    // people edit to make a red build green. It exists to catch a loader
    // regression that drops chapters, which is the one thing that would make
    // this number fall.
    const total = loaded.reduce((sum, track) => sum + track.chapters.length, 0);
    expect(total).toBeGreaterThanOrEqual(1_000);
  });
});

describe("the drift message does not walk an author into destroying their work", () => {
  it("names BOTH recovery directions, not just unshard", () => {
    // The message used to say only "run 'npm run unshard'". That was correct
    // while core/spine.json was the only sharded ledger, because its monolith
    // is purely derived. It is not correct for an AUTHORED ledger:
    // <track>/chapters.json is rewritten wholesale by the Python authoring
    // scripts in learning/human-languages/data/scripts/, and an author who
    // appends a chapter that way and then follows a bare "unshard" overwrites
    // the monolith from shards that never saw their chapter.
    const root = mkdtempSync(join(tmpdir(), "hl-drift-msg-"));
    try {
      mkdirSync(join(root, "spanish", "chapters.d"), { recursive: true });
      writeFileSync(
        join(root, "spanish", "chapters.d", "_meta.json"),
        `${JSON.stringify({ version: 1, language: "spanish" }, null, 2)}\n`,
        "utf8",
      );
      writeFileSync(
        join(root, "spanish", "chapters.d", "0001.json"),
        `${JSON.stringify({ chapter: 1 }, null, 2)}\n`,
        "utf8",
      );
      // A monolith that disagrees — the shape an author's direct edit leaves.
      writeFileSync(join(root, "spanish", "chapters.json"), "{}\n", "utf8");

      const errors: string[] = [];
      const write = process.stderr.write.bind(process.stderr);
      process.stderr.write = ((chunk: string) => {
        errors.push(String(chunk));
        return true;
      }) as typeof process.stderr.write;
      try {
        expect(runShardCli(["--check", "spanish/chapters.json"], root)).toBe(1);
      } finally {
        process.stderr.write = write;
      }

      const message = errors.join("");
      expect(message).toMatch(/npm run unshard spanish\/chapters\.json/);
      expect(message).toMatch(/npm run shard spanish\/chapters\.json/);
      expect(message).toMatch(/If you edited the monolith/);
      expect(message).toMatch(/Do not hand-merge/);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});

describe("the shards are the source of truth and the monolith is derived", () => {
  it("--unshard reproduces the committed monolith without changing it", () => {
    // Idempotence of the generator, which is what makes "on a conflict, take
    // either side and re-run the generator" a safe instruction rather than a
    // hopeful one.
    const plan = CHAPTER_PLANS.find((p) => p.path === "spanish/chapters.json")!;
    const before = readFileSync(join(root, plan.path), "utf8");
    expect(unshardLedger(root, plan)).toBe(before);
    expect(readFileSync(join(root, plan.path), "utf8")).toBe(before);
  });
});
