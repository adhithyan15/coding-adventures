// ---------------------------------------------------------------------------
// exam-inventory-census.test.ts — the gate over the gates.
//
// WHAT WENT WRONG
// Twenty-five inventories sit in `core/`. Twenty-two were pinned by a test that
// fails when their coverage moves. Three — Hindi, Sanskrit and Telugu — were
// loaded by NO test at all, so a tranche could raise their number, a retired
// atom could lower it, and the suite said the same thing either way. Two more —
// Spanish A1 and German A2 — pinned the number but never checked that the atoms
// their probes name exist, and a misspelt atom resolves to "not introduced":
// fail-safe, silent, and indistinguishable from a genuine gap.
//
// None of that was catchable, because nothing enumerated the inventories. Every
// assertion in this package's exam suites is written per-language and starts
// from a literal — `loadExamInventory("tamil", "A1")` — so an inventory nobody
// wrote a literal for is invisible to all of them, and the suite grows MORE
// confident as the corpus grows less watched.
//
// WHY THIS FILE ENUMERATES THE DIRECTORY AND NOT A LIST
// The obvious repair is a checked-in list of "inventories that must be pinned".
// That is the shape that already failed here once: a check comparing a generated
// set against a declared set agreed perfectly while seventeen appendices were
// hand-authored, because both sides were empty. A declared list cannot notice
// the thing it forgot to declare.
//
// So the enumeration is `readdirSync` over `core/`. It does not shrink when an
// inventory is forgotten, because a forgotten inventory is exactly a file that
// is present and unnamed. Add `exam-inventory-swahili-a1.json` and this file
// starts demanding a coverage pin for Swahili on the next run, with no edit
// here at all.
//
// WHAT IS DELIBERATELY NOT HERE
// A number. `tests/corpus/README.md` says a shared suite must not hold an exact
// corpus-wide list or a literal total that every language PR has to update, and
// the corpus exam-point total was un-pinned for cause in HL-C310: it read 529,
// 686, 793, 792, 775, 774, 786 and 839 in a single day, and two branches that
// both LOWERED it merged quietly because they agreed on a wrong value. Every
// count in this file is derived on the spot; the per-track counts stay in the
// per-track suites, where two tranches on two tracks never touch one line.
// ---------------------------------------------------------------------------
import { describe, expect, it } from "vitest";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { defaultCurriculumRoot, listExamInventories, loadEverything, loadExamInventory } from "../src/loader.js";
import { readLedgerFile } from "../src/shard.js";
import { trackIntroducedAtoms } from "../src/exam-inventory.js";

/** Where the tests themselves live, so the census can read its own suite. */
const TESTS_ROOT = fileURLToPath(new URL(".", import.meta.url));

interface CommittedInventory {
  file: string;
  language: string;
  level: string;
}

/**
 * The shape `loader.ts` will accept in a filename segment, restated here.
 *
 * Not decoration. `pinnedBy` builds a `RegExp` out of these two strings, and an
 * inventory declaring `"language": ".*"` would compile to a pattern matching
 * EVERY track's literal — so the file would borrow Tamil's pin and pass the one
 * gate it exists to fail. A regex metacharacter also makes the source scan a
 * ReDoS target that no test timeout can interrupt, because the backtracking is
 * synchronous. Both close by refusing the string here, before it is a pattern.
 *
 * `loadExamInventory` enforces the same shape, so a hostile value already fails
 * a DIFFERENT test in this file. That is incidental protection in another `it`;
 * the gate needs its own.
 */
const SAFE_INVENTORY_SEGMENT = /^[A-Za-z0-9]+$/;

/**
 * Every committed inventory, from the DIRECTORY LISTING.
 *
 * Read through `readLedgerFile` rather than `listExamInventories`, on purpose.
 * That helper swallows a `LedgerParseError` and returns the file's neighbours,
 * which is right for the planner — one broken file should not blank the plan —
 * and wrong for a census, where a file that stopped parsing must surface as a
 * failure rather than as one fewer thing to check. The two are compared below.
 *
 * `readLedgerFile` and not a bare `readFileSync`, because it is the door every
 * other reader in this package uses: it refuses a symlink and a non-regular file
 * (a symlinked inventory would otherwise be read from outside the tree, and
 * `readFileSync` on a device node never returns), rejects dangerous keys, and
 * refuses a stale monolith beside a sharded ledger.
 */
function committedInventories(root = defaultCurriculumRoot()): CommittedInventory[] {
  const directory = resolve(root, "core");
  return readdirSync(directory)
    .filter((file) => file.startsWith("exam-inventory-") && file.endsWith(".json"))
    .sort()
    .map((file) => {
      const parsed = readLedgerFile<Record<string, unknown>>(join(directory, file));
      const language = Object.hasOwn(parsed, "language") ? parsed["language"] : undefined;
      const level = Object.hasOwn(parsed, "level") ? parsed["level"] : undefined;
      if (typeof language !== "string" || typeof level !== "string") {
        throw new Error(`${file} declares no string language/level, so nothing can measure it`);
      }
      if (!SAFE_INVENTORY_SEGMENT.test(language) || !SAFE_INVENTORY_SEGMENT.test(level)) {
        throw new Error(`${file} declares an unsafe language/level: '${language}' '${level}'`);
      }
      return { file, language, level };
    });
}

/** Every `*.test.ts` under `tests/`, so a pin may live wherever it belongs. */
function suiteSources(directory = TESTS_ROOT): { path: string; text: string }[] {
  const found: { path: string; text: string }[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true }).sort((a, b) =>
    a.name.localeCompare(b.name),
  )) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) found.push(...suiteSources(path));
    else if (entry.name.endsWith(".test.ts")) found.push({ path, text: readFileSync(path, "utf8") });
  }
  return found;
}

/**
 * A test file split at its top-level `describe(` / `it(`.
 *
 * Block scoping is what stops a false pass. Eight files load
 * `loadExamInventory("spanish", "A1")` as the DERIVATION SOURCE for another
 * track's inventory and then measure that other track; a whole-file grep would
 * read those as Spanish pins and report Spanish watched by nine suites while
 * the one real pin could be deleted freely.
 */
function topLevelBlocks(text: string): string[] {
  const starts = [...text.matchAll(/^(?:describe|it)\(/gm)].map((match) => match.index ?? 0);
  if (starts.length === 0) return [];
  return starts.map((start, index) => text.slice(start, starts[index + 1] ?? text.length));
}

/** A block only counts if it READS the measurement, not merely computes one. */
const READS_THE_MEASUREMENT = /expect\(\s*coverage\b|formatExamCoverage\(/;

/**
 * Does any block both measure THIS inventory and assert something about it?
 *
 * Two idioms are in use and both are accepted: measuring the load expression
 * directly, and binding it to a `const` first. What is not accepted is a block
 * that loads the inventory and never measures it — Spanish A1 had a probe-name
 * REGEX check for months, which proves a name is well-shaped and nothing about
 * whether the corpus contains it.
 */
function pinnedBy(sources: ReturnType<typeof suiteSources>, { language, level }: CommittedInventory): string[] {
  const load = `loadExamInventory\\("${language}",\\s*"${level}"\\)`;
  const direct = new RegExp(`measureExamCoverage\\(\\s*${load}`);
  const binding = new RegExp(`(?:const|let)\\s+(\\w+)\\s*=\\s*${load}`);
  const found: string[] = [];
  for (const source of sources) {
    for (const block of topLevelBlocks(source.text)) {
      if (!READS_THE_MEASUREMENT.test(block)) continue;
      const bound = binding.exec(block);
      const measured =
        direct.test(block) ||
        (bound !== null && new RegExp(`measureExamCoverage\\(\\s*${bound[1]}\\b`).test(block));
      if (measured) found.push(source.path.slice(TESTS_ROOT.length));
    }
  }
  return [...new Set(found)].sort();
}

describe("the exam-inventory census", () => {
  const committed = committedInventories();

  it("finds inventories at all, so an empty walk cannot pass every test below", () => {
    // The failure this whole file is about is a check that agrees because both
    // sides are empty. A census whose enumeration returned nothing would satisfy
    // every `for` loop underneath, so the enumeration is asserted first.
    //
    // A FLOOR, not a total. It fails when inventories are deleted and never when
    // one is added, so a new track does not come here to re-pin a number — which
    // is the property the corpus point total did not have, and why it moved
    // eight times in one day and merged wrong twice.
    expect(committed.length).toBeGreaterThanOrEqual(25);
    expect(new Set(committed.map((inventory) => `${inventory.language} ${inventory.level}`)).size).toBe(
      committed.length,
    );
  });

  it("keeps the planner's view equal to the directory, so nothing is silently skipped", () => {
    // `listExamInventories` absorbs a `LedgerParseError` and moves on. That is a
    // reasonable planner and a terrible census: a file that stops parsing leaves
    // the plan, which queues somebody to write an inventory that already exists,
    // and leaves this census, which stops asking for its pin. Compared here so
    // the difference is loud.
    const listed = new Set(listExamInventories().map((entry) => `${entry.language} ${entry.level}`));
    const missing = committed
      .filter((inventory) => !listed.has(`${inventory.language} ${inventory.level}`))
      .map((inventory) => inventory.file);
    expect(missing, "committed but unlisted — did one stop parsing?").toEqual([]);
    expect(listed.size).toBe(committed.length);
  });

  it("PROBES ONLY ATOMS THAT EXIST, for every committed inventory", () => {
    // The check Spanish A1 and German A2 never had. A misspelt or retired atom
    // resolves to "not introduced", so the point reads uncovered and the number
    // moves DOWN — safe, silent, and identical to a real gap, which means the
    // repair somebody schedules is authoring a lesson for content that is
    // already there.
    //
    // Derived over the directory listing, so it covers the inventories nobody
    // wrote a literal for, and so a twenty-sixth is covered on the day it lands.
    const { lessons } = loadEverything();
    const taughtByTrack = new Map<string, Set<string>>();
    const unknown: string[] = [];
    for (const inventory of committed) {
      let taught = taughtByTrack.get(inventory.language);
      if (taught === undefined) {
        taughtByTrack.set(inventory.language, (taught = trackIntroducedAtoms(lessons, inventory.language)));
      }
      for (const point of loadExamInventory(inventory.language, inventory.level).points) {
        for (const atom of point.probe ?? []) {
          if (!taught.has(atom)) unknown.push(`${inventory.file} ${point.id}:${atom}`);
        }
      }
    }
    expect(unknown).toEqual([]);
  }, 180_000);

  it("HAS A COVERAGE PIN for every committed inventory, wherever that pin lives", () => {
    // The (a) half. The numbers themselves stay per-track — this asserts only
    // that SOMETHING reads each one, which is the property three inventories
    // lacked entirely while looking no different from the twenty-two that had it.
    //
    // A source scan, because the thing being asserted is about the suite rather
    // than about the data: "is this measurement watched?" has no runtime witness.
    // If a restructure moves a pin into a shape this cannot see, the fix is to
    // teach `pinnedBy` the new idiom — not to drop the track from a list, which
    // is unavailable here by construction.
    const sources = suiteSources();
    expect(sources.length).toBeGreaterThan(20);
    const unpinned = committed.filter((inventory) => pinnedBy(sources, inventory).length === 0);
    expect(
      unpinned.map((inventory) => inventory.file),
      "committed inventories whose measured coverage no test reads",
    ).toEqual([]);
  });


  it("REFUSES a language or level that would compile into a pattern", () => {
    // The finding of this PR's own security review, and the sharpest failure the
    // census could have had: `pinnedBy` interpolates `language` and `level` into
    // a `RegExp`, so an inventory declaring `"language": ".*"` compiles to
    // `loadExamInventory\(".*",\s*"A1"\)`, matches the literal in EVERY other
    // track's pin, and passes the one gate it exists to fail — silently, and in
    // the flattering direction. A metacharacter like `(a+)+` is worse still: the
    // backtracking is synchronous, so vitest's timeout cannot interrupt it and
    // the worker hangs rather than going red.
    //
    // `loadExamInventory` refuses the same shape, so today a hostile value also
    // fails the atom-existence test. That is protection in a DIFFERENT `it`, and
    // a gate that depends on its neighbour failing is not a gate.
    const root = mkdtempSync(join(tmpdir(), "exam-inventory-census-"));
    mkdirSync(join(root, "core"));
    const write = (file: string, value: unknown) =>
      writeFileSync(join(root, "core", file), JSON.stringify(value));
    try {
      const shell = { version: 1, level: "A1", about: "x", source: "x", points: [] };
      write("exam-inventory-ok-a1.json", { ...shell, language: "ok" });
      expect(committedInventories(root)).toEqual([
        { file: "exam-inventory-ok-a1.json", language: "ok", level: "A1" },
      ]);
      write("exam-inventory-any-a1.json", { ...shell, language: ".*" });
      expect(() => committedInventories(root)).toThrow(/unsafe language\/level/);
      rmSync(join(root, "core", "exam-inventory-any-a1.json"));
      write("exam-inventory-redos-a1.json", { ...shell, language: "(a+)+" });
      expect(() => committedInventories(root)).toThrow(/unsafe language\/level/);
      rmSync(join(root, "core", "exam-inventory-redos-a1.json"));
      write("exam-inventory-nolang-a1.json", { ...shell });
      expect(() => committedInventories(root)).toThrow(/no string language\/level/);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("recognises a pin only when the block MEASURES the inventory it loads", () => {
    // The falsification of the scanner itself, run against synthetic sources
    // rather than the tree, so it proves the discrimination and not today's
    // arrangement of files. Without block scoping and the binding check, the
    // third of these would pass and Spanish would read as pinned by every file
    // that merely derives from it.
    const fake = (text: string) => [{ path: `${TESTS_ROOT}synthetic.test.ts`, text }];
    const target = { file: "exam-inventory-xx-a1.json", language: "xx", level: "A1" };
    expect(
      pinnedBy(
        fake('it("pins", () => {\n  const c = measureExamCoverage(loadExamInventory("xx", "A1"), l);\n  expect(c.covered).toBe(1);\n});\n'),
        target,
      ),
      "a direct measure with no `coverage` binding is still a pin only if it reads one",
    ).toEqual([]);
    expect(
      pinnedBy(
        fake('it("pins", () => {\n  const coverage = measureExamCoverage(loadExamInventory("xx", "A1"), l);\n  expect(coverage.covered).toBe(1);\n});\n'),
        target,
      ),
    ).toEqual(["synthetic.test.ts"]);
    expect(
      pinnedBy(
        fake('describe("d", () => {\n  const inventory = loadExamInventory("xx", "A1");\n  it("m", () => {\n    const coverage = measureExamCoverage(inventory, l);\n    expect(coverage.covered).toBe(1);\n  });\n});\n'),
        target,
      ),
    ).toEqual(["synthetic.test.ts"]);
    // THE FALSE PASS THIS EXISTS TO REFUSE: one block derives from `xx` and
    // measures `yy`. A whole-file grep calls that a pin for `xx`.
    expect(
      pinnedBy(
        fake('describe("d", () => {\n  const source = loadExamInventory("xx", "A1");\n  const inventory = loadExamInventory("yy", "A1");\n  it("m", () => {\n    const coverage = measureExamCoverage(inventory, l);\n    expect(coverage.covered).toBe(1);\n  });\n});\n'),
        target,
      ),
    ).toEqual([]);
    // And a block that loads it, checks the SHAPE of its probe names, and never
    // measures anything — which is what Spanish A1 had instead of an existence
    // check, and what read as coverage to a coarser scan.
    expect(
      pinnedBy(
        fake('describe("d", () => {\n  const inventory = loadExamInventory("xx", "A1");\n  it("names", () => {\n    for (const p of inventory.points) expect(p.id).toMatch(/^XX-/);\n  });\n});\n'),
        target,
      ),
    ).toEqual([]);
  });
});
