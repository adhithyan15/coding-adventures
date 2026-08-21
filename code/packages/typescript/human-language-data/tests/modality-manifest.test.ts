import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { afterEach, describe, expect, it, vi } from "vitest";
import { loadEverything, loadModalityManifest, modalityManifestById } from "../src/loader.js";
import {
  generatedModalityOutputs,
  generatedModalityOutputsFromLessons,
  runModalityManifest,
  safeOutput,
} from "../src/modality-cli.js";
import {
  MODALITY_MANIFEST_DIR,
  MODALITY_MANIFEST_VERSION,
  buildModalityManifest,
  modalityCorpusHash,
  serializeModalityManifest,
  type ModalityManifest,
  type ModalityManifestLesson,
} from "../src/modality-manifest.js";
import { parseLesson, type ParsedLesson } from "../src/parse.js";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/** Build a lesson from parts, so each test names only what it is actually testing. */
function lesson(options: {
  id: string;
  language?: string;
  chapter?: number | null;
  sequence?: number;
  type?: string;
  modality?: string;
  modalityReason?: string;
  body?: string;
}): ParsedLesson {
  const frontmatter = ["schema_version: 2", `id: ${options.id}`, `type: ${options.type ?? "word"}`];
  if (options.chapter !== null) frontmatter.push(`chapter: ${options.chapter ?? 1}`);
  frontmatter.push("headword: hola", "gloss: hello", "concept_tag: GREETING-HELLO");
  if (options.sequence !== undefined) frontmatter.push(`sequence: ${options.sequence}`);
  if (options.modality !== undefined) frontmatter.push(`modality: ${options.modality}`);
  if (options.modalityReason !== undefined) {
    frontmatter.push(`modality_reason: ${options.modalityReason}`);
  }
  const body = options.body ?? "## Warm-up\n\nSay *hola* out loud.";
  return parseLesson(
    `---\n${frontmatter.join("\n")}\n---\n\n# ${options.id}\n\n${body}\n`,
    options.language ?? "spanish",
  );
}

/** A body with a paradigm grid, which is the corpus's commonest reason to need eyes. */
const TABLE_BODY = "## Warm-up\n\n| yo | tú | él | ella |\n|---|---|---|---|\n| soy | eres | es | es |";

const roots: string[] = [];

/** A minimal curriculum root on disk, for the CLI's filesystem behaviour. */
function fixtureRoot(lessons: Array<{ file: string; source: string }>): string {
  const root = mkdtempSync(join(tmpdir(), "human-language-modality-"));
  roots.push(root);
  mkdirSync(join(root, "core"), { recursive: true });
  mkdirSync(join(root, "spanish", "lessons"), { recursive: true });
  for (const entry of lessons) {
    writeFileSync(join(root, "spanish", "lessons", entry.file), entry.source, "utf8");
  }
  return root;
}

function simpleRoot(): string {
  return fixtureRoot([
    {
      file: "hola.md",
      source: `---
schema_version: 2
id: ES-C01-hola
chapter: 1
sequence: 10
type: word
headword: hola
gloss: hello
concept_tag: GREETING-HELLO
---

# hola

## Warm-up

Say *hola* out loud.
`,
    },
    {
      file: "ser.md",
      source: `---
schema_version: 2
id: ES-C01-ser
chapter: 1
sequence: 20
type: grammar
headword: ser
gloss: to be
concept_tag: GRAMMAR-SER
---

# ser

## Warm-up

| yo | tú | él | ella |
|---|---|---|---|
| soy | eres | es | es |
`,
    },
  ]);
}

afterEach(() => {
  vi.restoreAllMocks();
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// The artifact's shape
// ---------------------------------------------------------------------------

describe("the modality manifest", () => {
  it("records every lesson with the fields an edition filter needs", () => {
    const manifest = buildModalityManifest([
      lesson({ id: "ES-C01-a", chapter: 1, sequence: 10 }),
      lesson({ id: "ES-C01-b", chapter: 1, sequence: 20, body: TABLE_BODY }),
      lesson({ id: "ES-C01-c", chapter: 1, sequence: 30, type: "writing" }),
    ]);

    expect(manifest.version).toBe(MODALITY_MANIFEST_VERSION);
    expect(manifest.algorithm).toBe("fnv1a64");
    expect(manifest.lessons.map((entry) => entry.id)).toEqual([
      "ES-C01-a",
      "ES-C01-b",
      "ES-C01-c",
    ]);
    expect(manifest.lessons[0]).toMatchObject({
      id: "ES-C01-a",
      language: "spanish",
      chapter: 1,
      sequence: 10,
      modality: "voice",
      derived: "voice",
      drivable: true,
      reasons: ["no-visual-dependency"],
    });
    expect(manifest.lessons[0]?.sourceHash).toMatch(/^fnv1a64:[0-9a-f]{16}$/);
    expect(manifest.lessons[1]).toMatchObject({ modality: "sight", drivable: false });
    expect(manifest.lessons[2]).toMatchObject({ modality: "pen", drivable: false });
  });

  it("records the policy the numbers were measured under", () => {
    // 3, not the 0 this shipped at. The knob was 0 while no lineariser existed —
    // claiming a table was speakable would have claimed a capability nothing
    // implemented. HL-C16 built it, so the default is now its measured value.
    expect(buildModalityManifest([]).policy).toEqual({ maxLinearisableTableColumns: 3 });
    // The manifest must say which world its counts came from, or a reader cannot tell a
    // remediated corpus from a relaxed detector.
    const relaxed = buildModalityManifest([lesson({ id: "ES-C01-a", body: TABLE_BODY })], {
      maxLinearisableTableColumns: 4,
    });
    expect(relaxed.policy.maxLinearisableTableColumns).toBe(4);
    expect(relaxed.lessons[0]?.drivable).toBe(true);
  });

  it("omits the override fields on the common case and emits them on the rare one", () => {
    const manifest = buildModalityManifest([
      lesson({ id: "ES-C01-a", sequence: 10 }),
      lesson({
        id: "ES-C01-b",
        sequence: 20,
        type: "writing",
        modality: "voice",
        modalityReason: "the drill is dictated aloud",
      }),
    ]);

    // Absent means "the author said nothing" — the manifest does not carry 1,096
    // copies of the empty string to say so.
    expect(manifest.lessons[0]).not.toHaveProperty("authored");
    expect(manifest.lessons[0]).not.toHaveProperty("authoredReason");
    expect(manifest.lessons[0]).not.toHaveProperty("overridden");

    expect(manifest.lessons[1]).toMatchObject({
      modality: "voice",
      derived: "pen",
      overridden: true,
      authored: "voice",
      authoredReason: "the drill is dictated aloud",
    });
    expect(manifest.summary.overriddenLessons).toBe(1);
    // An explained override is legitimate, so it produces no finding.
    expect(manifest.findings).toEqual([]);
  });

  it("carries unexplained and unknown overrides through as findings, not exceptions", () => {
    const manifest = buildModalityManifest([
      lesson({ id: "ES-C01-a", sequence: 10, type: "writing", modality: "voice" }),
      lesson({ id: "ES-C01-b", sequence: 20, modality: "telepathy" }),
    ]);
    expect(manifest.findings.map((finding) => finding.code)).toEqual([
      "modality-unexplained-override",
      "modality-unknown-value",
    ]);
    // An unusable value falls back to the derivation rather than poisoning the corpus
    // with a channel nothing can render.
    expect(manifest.lessons[1]?.modality).toBe("voice");
    expect(manifest.lessons[1]?.authored).toBe("telepathy");
  });
});

// ---------------------------------------------------------------------------
// Ordering, which is what makes the bytes stable
// ---------------------------------------------------------------------------

describe("manifest ordering", () => {
  it("orders by track, chapter, authored sequence, then id", () => {
    const manifest = buildModalityManifest([
      lesson({ id: "ES-C02-a", language: "spanish", chapter: 2, sequence: 10 }),
      lesson({ id: "FR-C01-a", language: "french", chapter: 1, sequence: 10 }),
      lesson({ id: "ES-C01-b", language: "spanish", chapter: 1, sequence: 20 }),
      lesson({ id: "ES-C01-a", language: "spanish", chapter: 1, sequence: 10 }),
    ]);
    expect(manifest.lessons.map((entry) => entry.id)).toEqual([
      "FR-C01-a",
      "ES-C01-a",
      "ES-C01-b",
      "ES-C02-a",
    ]);
  });

  it("sorts a missing sequence and a missing chapter LAST, never as zero", () => {
    const manifest = buildModalityManifest([
      lesson({ id: "ES-legacy", chapter: null }),
      lesson({ id: "ES-C01-nosequence", chapter: 1 }),
      lesson({ id: "ES-C01-first", chapter: 1, sequence: 10 }),
    ]);
    // A legacy lesson with no `sequence` has not claimed to come first. Treating a
    // missing number as 0 would put it ahead of every authored lesson and silently
    // rewrite the drivable prefix.
    expect(manifest.lessons.map((entry) => entry.id)).toEqual([
      "ES-C01-first",
      "ES-C01-nosequence",
      "ES-legacy",
    ]);
    expect(manifest.summary.lessonsWithoutChapter).toBe(1);
    // A chapterless lesson still counts toward its track, but belongs to no chapter.
    expect(manifest.tracks[0]?.lessonCount).toBe(3);
    expect(manifest.tracks[0]?.chapters).toHaveLength(1);
    expect(manifest.tracks[0]?.chapters[0]?.lessonCount).toBe(2);
  });

  it("produces identical bytes no matter what order the lessons arrived in", () => {
    // `loadLessons` walks directories, and directory order is a property of the disk,
    // not of the curriculum. If it leaked into the output, `--check` would fail on a
    // colleague's machine for no reason at all.
    const lessons = [
      lesson({ id: "ES-C01-a", sequence: 10 }),
      lesson({ id: "ES-C01-b", sequence: 20, body: TABLE_BODY }),
      lesson({ id: "ES-C02-a", chapter: 2, sequence: 10 }),
    ];
    const forwards = serializeModalityManifest(buildModalityManifest(lessons));
    const backwards = serializeModalityManifest(buildModalityManifest([...lessons].reverse()));
    expect(backwards).toBe(forwards);
  });

  it("fingerprints the corpus independently of input order", () => {
    const lessons = [lesson({ id: "ES-C01-a" }), lesson({ id: "ES-C01-b", body: TABLE_BODY })];
    expect(modalityCorpusHash([...lessons].reverse())).toBe(modalityCorpusHash(lessons));
    // And it actually depends on the lessons, so a content edit moves it.
    expect(modalityCorpusHash([lesson({ id: "ES-C01-a", body: "## Warm-up\n\nDifferent." })])).not.toBe(
      modalityCorpusHash([lesson({ id: "ES-C01-a" })]),
    );
  });

  it("round-trips through JSON unchanged", () => {
    const manifest = buildModalityManifest([
      lesson({ id: "ES-C01-a", sequence: 10 }),
      lesson({ id: "ES-C01-b", sequence: 20, type: "writing" }),
    ]);
    const text = serializeModalityManifest(manifest);
    expect(text.endsWith("\n")).toBe(true);
    expect(JSON.parse(text)).toEqual(manifest);
  });
});

// ---------------------------------------------------------------------------
// Chapters — the number a commuter actually asks for
// ---------------------------------------------------------------------------

describe("chapter rollups", () => {
  it("stops the drivable prefix at the first blocker and names it", () => {
    const manifest = buildModalityManifest([
      lesson({ id: "ES-C01-a", sequence: 10 }),
      lesson({ id: "ES-C01-b", sequence: 20 }),
      lesson({ id: "ES-C01-blocker", sequence: 30, body: TABLE_BODY }),
      // A voice lesson BEHIND a blocker is not reachable in the car: the lessons are
      // prerequisite ordered, so the prefix stops even though this one is ear-only.
      lesson({ id: "ES-C01-d", sequence: 40 }),
    ]);
    const chapter = manifest.tracks[0]?.chapters[0];
    expect(chapter).toMatchObject({
      chapter: 1,
      lessonCount: 4,
      voice: 3,
      sight: 1,
      pen: 0,
      drivablePrefix: 2,
      firstNonVoiceLesson: "ES-C01-blocker",
      drivable: false,
      modalities: ["voice", "sight"],
    });
    expect(chapter?.drivableLessonIds).toEqual(["ES-C01-a", "ES-C01-b"]);
  });

  it("marks a chapter drivable only when every lesson is voice", () => {
    const manifest = buildModalityManifest([
      lesson({ id: "ES-C01-a", sequence: 10 }),
      lesson({ id: "ES-C01-b", sequence: 20 }),
    ]);
    expect(manifest.tracks[0]?.chapters[0]).toMatchObject({
      drivable: true,
      drivablePrefix: 2,
      firstNonVoiceLesson: null,
      modalities: ["voice"],
    });
    expect(manifest.summary.fullyDrivableChapters).toBe(1);
    expect(manifest.summary.unstartableChapters).toBe(0);
  });

  it("counts a chapter a commuter cannot even start", () => {
    const manifest = buildModalityManifest([
      lesson({ id: "ES-C01-a", sequence: 10, type: "writing" }),
      lesson({ id: "ES-C01-b", sequence: 20 }),
    ]);
    expect(manifest.tracks[0]?.chapters[0]).toMatchObject({
      drivablePrefix: 0,
      firstNonVoiceLesson: "ES-C01-a",
      drivable: false,
      // `pen` implies `sight`, so the union names every channel the chapter needs —
      // including `voice`, from the second lesson the commuter cannot reach.
      modalities: ["voice", "sight", "pen"],
    });
    expect(manifest.summary.unstartableChapters).toBe(1);
    expect(manifest.summary.fullyDrivableChapters).toBe(0);
  });

  it("rolls tracks up with a whole-percent drivable share", () => {
    const manifest = buildModalityManifest([
      lesson({ id: "ES-C01-a", chapter: 1, sequence: 10 }),
      lesson({ id: "ES-C02-a", chapter: 2, sequence: 10, type: "writing" }),
      lesson({ id: "ES-C02-b", chapter: 2, sequence: 20 }),
    ]);
    expect(manifest.tracks[0]).toMatchObject({
      language: "spanish",
      lessonCount: 3,
      voice: 2,
      sight: 0,
      pen: 1,
      drivablePercent: 67, // SYNTHETIC FIXTURE -- 2 of 3 lessons in the hand-built case above. Do NOT repin from the corpus: HL-C196, the patcher matched this by field name and oscillated against line 998 for 60 rounds.
      // Chapter 1 contributes 1; chapter 2 contributes 0, because its pen lesson is
      // first. The second voice lesson exists but is not reachable by ear.
      drivablePrefixTotal: 1,
      modalities: ["voice", "sight", "pen"],
    });
    expect(manifest.summary.drivablePrefixTotal).toBe(1);
  });

  it("survives an empty corpus without claiming a drivable one", () => {
    const manifest = buildModalityManifest([]);
    expect(manifest.summary).toMatchObject({
      totalLessons: 0,
      drivableLessons: 0,
      drivablePercent: 0,
      trackCount: 0,
      chapterCount: 0,
      fullyDrivableChapters: 0,
    });
    expect(manifest.tracks).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Forward compatibility with block-level modality (HL-C41)
// ---------------------------------------------------------------------------

describe("block-level modality", () => {
  it("declares that this build carries core modality beside the whole-lesson answer", () => {
    // A capability flag, not a version bump: block modality is strictly additional
    // information, so both kinds of manifest stay version 1 and both answer the
    // driving question correctly.
    //
    // False until HL-C48 — and honestly so. The flag says "this build carries block
    // data", and the build did not: no row emitted `coreModality` at all. Flipping the
    // flag without emitting the data would have been the real bug, and reading the flag
    // as "which field does `drivable` use" was a misreading that cost five waves of
    // treating a missing feature as a design tension.
    expect(buildModalityManifest([]).features).toEqual({ blockModality: true });
    expect(buildModalityManifest([]).version).toBe(1);
  });

  it("publishes the core answer WITHOUT moving the conservative one", () => {
    // The compatibility promise this file's header makes, asserted rather than trusted.
    // A lesson whose only visual dependency is an inline-letters section still reports
    // `modality: sight` and `drivable: false` — a consumer that never learns about
    // detachable blocks is unaffected by this change — while `coreModality`/
    // `coreDrivable` say what a renderer that CAN set that section aside should do.
    const manifest = buildModalityManifest([
      lesson({
        id: "ES-C01-a",
        body: "## Warm-up\n\nSay it aloud.\n\n## The letters in this word\n\nThe first is round.",
      }),
    ]);
    const row = manifest.lessons[0]!;
    expect(row.modality).toBe("sight");
    expect(row.drivable).toBe(false);
    expect(row.coreModality).toBe("voice");
    expect(row.coreDrivable).toBe(true);
    // Auditable, not asserted: a reader can see which section was discounted.
    expect(row.detachableSegments).toEqual(["The letters in this word"]);
  });

  it("omits detachableSegments entirely when a lesson has none", () => {
    // Same reasoning as the override fields: most lessons have no detachable section,
    // and an empty array on a thousand rows is noise every reader must skip.
    const row = buildModalityManifest([lesson({ id: "ES-C01-a" })]).lessons[0]!;
    expect(row).not.toHaveProperty("detachableSegments");
    expect(row.coreModality).toBe("voice");
  });

  it("never lets the core answer be stronger than the whole-lesson one", () => {
    // The structural invariant. `coreModality` is derived from a SUBSET of the blocks,
    // so it can only ever be weaker or equal. If this ever fails, the derivation has a
    // bug that would hand a driver a lesson needing a pen.
    const { lessons } = loadEverything();
    const order = { voice: 0, sight: 1, pen: 2 } as const;
    for (const row of buildModalityManifest(lessons).lessons) {
      expect(order[row.coreModality]).toBeLessThanOrEqual(order[row.modality]);
      if (row.coreDrivable) expect(row.coreModality).toBe("voice");
    }
  });

  it("keeps `modality` the conservative whole-lesson answer a naive reader can trust", () => {
    // The contract HL-C41 must not break: `modality` is the STRONGEST channel the
    // lesson needs anywhere in it. A reader that never learns about `coreModality`
    // therefore drops this lesson from a driving edition — pessimistic, but never
    // wrong in the dangerous direction.
    const manifest = buildModalityManifest([
      lesson({
        id: "TE-C01-inline-writing",
        body: "## Warm-up\n\nSay it aloud.\n\n## Script — the letter అ\n\nTrace it.",
      }),
    ]);
    expect(manifest.lessons[0]?.modality).toBe("sight");
    expect(manifest.lessons[0]?.drivable).toBe(false);
  });

  it("lets a consumer opt into `coreModality` with a read that works before and after", () => {
    // Rows are JSON objects, so HL-C41 adds a key rather than changing a shape. This
    // is the exact expression a driving-edition renderer will use; it must already be
    // correct against today's manifest, which is what makes the change additive.
    const before = buildModalityManifest([lesson({ id: "ES-C01-a", type: "writing" })]).lessons[0];
    expect(before).toBeDefined();
    const readCore = (entry: ModalityManifestLesson & { coreModality?: string }) =>
      entry.coreModality ?? entry.modality;
    expect(readCore(before as ModalityManifestLesson)).toBe("pen");
    expect(readCore({ ...(before as ModalityManifestLesson), coreModality: "voice" })).toBe("voice");
  });
});

// ---------------------------------------------------------------------------
// The CLI shell
// ---------------------------------------------------------------------------

describe("the modality manifest CLI", () => {
  it("writes the manifest, then passes its own check", () => {
    const root = simpleRoot();
    vi.spyOn(process.stdout, "write").mockImplementation(() => true);

    expect(runModalityManifest(["--write"], root)).toBe(0);
    const output = join(root, "core", "lesson-modality", "spanish.json");
    expect(existsSync(output)).toBe(true);
    expect(process.stdout.write).toHaveBeenCalledWith(
      "generated core/lesson-modality/spanish.json\n",
    );

    const manifest = JSON.parse(readFileSync(output, "utf8")) as ModalityManifest;
    expect(manifest.summary).toMatchObject({ totalLessons: 2, voice: 1, sight: 1 });
    expect(runModalityManifest(["--check"], root)).toBe(0);
  });

  it("detects a stale manifest and exits 1", () => {
    const root = simpleRoot();
    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runModalityManifest(["--write"], root)).toBe(0);

    // The drift that matters: a lesson gains a paradigm table and nobody regenerates.
    // Without this gate the manifest still says `drivable: true` and the driving
    // edition hands a chart to somebody at 70mph.
    const hola = join(root, "spanish", "lessons", "hola.md");
    writeFileSync(hola, `${readFileSync(hola, "utf8")}\n| yo | tú | él | ella |\n`, "utf8");

    expect(runModalityManifest(["--check"], root)).toBe(1);
    expect(process.stderr.write).toHaveBeenCalledWith(
      "core/lesson-modality/spanish.json: generated output is missing or stale\n",
    );
  });

  it("treats a missing manifest as drift", () => {
    const root = simpleRoot();
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runModalityManifest(["--check"], root)).toBe(1);
  });

  it("compares bytes, so even a reformat counts as drift", () => {
    const root = simpleRoot();
    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runModalityManifest(["--write"], root)).toBe(0);

    const output = join(root, "core", "lesson-modality", "spanish.json");
    const compacted = JSON.stringify(JSON.parse(readFileSync(output, "utf8")));
    writeFileSync(output, compacted, "utf8");
    expect(runModalityManifest(["--check"], root)).toBe(1);
  });

  it("rejects an unsupported mode with the usage text", () => {
    const root = simpleRoot();
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runModalityManifest([], root)).toBe(2);
    expect(runModalityManifest(["--check", "--write"], root)).toBe(2);
    expect(runModalityManifest(["--wipe"], root)).toBe(2);
    expect(process.stderr.write).toHaveBeenCalledWith("usage: modality-cli (--check | --write)\n");
  });

  it("exposes the bytes as a path -> content map so write and check cannot diverge", () => {
    const root = simpleRoot();
    const outputs = generatedModalityOutputs(root);
    expect([...outputs.keys()]).toEqual([`${MODALITY_MANIFEST_DIR}/spanish.json`]);
    expect(outputs.get(`${MODALITY_MANIFEST_DIR}/spanish.json`)).toContain(
      '"algorithm": "fnv1a64"',
    );
  });

  it("changes exactly one shard for a one-language lesson change", () => {
    const base = [
      lesson({ id: "ES-C01-a", language: "spanish" }),
      lesson({ id: "FR-C01-a", language: "french" }),
    ];
    const before = generatedModalityOutputsFromLessons(base);
    const after = generatedModalityOutputsFromLessons([
      ...base,
      lesson({ id: "ES-C01-b", language: "spanish", sequence: 20 }),
    ]);
    expect([...before.keys()].filter((path) => before.get(path) !== after.get(path))).toEqual([
      `${MODALITY_MANIFEST_DIR}/spanish.json`,
    ]);
  });
});

// ---------------------------------------------------------------------------
// The path guard
// ---------------------------------------------------------------------------

describe("the output path guard", () => {
  it("fails closed on every way out of the curriculum root", () => {
    const root = simpleRoot();
    for (const escape of [
      "../escape.json",
      "../../escape.json",
      // The classic hole: no leading `..`, still escapes once resolved. Containment is
      // decided after `resolve`, not by inspecting the input string.
      "core/../../escape.json",
      "core/a/b/../../../../escape.json",
      "/etc/passwd.json",
    ]) {
      expect(() => safeOutput(root, escape)).toThrow(/unsafe generated modality output/);
      expect(() => generatedModalityOutputs(root, escape)).toThrow(
        /unsafe generated modality output/,
      );
    }
  });

  it("refuses the root itself and anything that is not JSON", () => {
    const root = simpleRoot();
    expect(() => safeOutput(root, "")).toThrow(/unsafe generated modality output/);
    expect(() => safeOutput(root, ".")).toThrow(/unsafe generated modality output/);
    // A `.tex` or `.md` target would let a mistake overwrite an authored book chapter
    // or a lesson, which is the failure this extension check exists to prevent.
    expect(() => safeOutput(root, "spanish/book/chapters/ch01.tex")).toThrow(
      /unsafe generated modality output/,
    );
    expect(() => safeOutput(root, "spanish/lessons/hola.md")).toThrow(
      /unsafe generated modality output/,
    );
  });

  it("accepts a contained JSON path", () => {
    const root = simpleRoot();
    expect(safeOutput(root, `${MODALITY_MANIFEST_DIR}/spanish.json`)).toBe(
      join(root, "core", "lesson-modality", "spanish.json"),
    );
    expect(generatedModalityOutputs(root, "core/driving-edition").has("core/driving-edition/spanish.json")).toBe(
      true,
    );
  });
});

// ---------------------------------------------------------------------------
// The consumer-side loader
// ---------------------------------------------------------------------------

describe("reading the manifest back", () => {
  it("loads what the CLI wrote", () => {
    const root = simpleRoot();
    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    runModalityManifest(["--write"], root);

    const manifest = loadModalityManifest(root);
    expect(manifest.version).toBe(MODALITY_MANIFEST_VERSION);
    expect(manifest.lessons.map((entry) => entry.id)).toEqual(["ES-C01-hola", "ES-C01-ser"]);
  });

  it("throws rather than inventing an empty corpus when the manifest is absent", () => {
    // "No modality data" and "no lesson needs eyes" are opposite facts. A loader that
    // returned the second when it meant the first would hand a driver the drills.
    expect(() => loadModalityManifest(simpleRoot())).toThrow();
  });

  it("indexes by id with a Map, so a `__proto__` id cannot poison the lookup", () => {
    // The keys come out of parsed JSON. `index[lesson.id] = lesson` on a plain object
    // with an id of `__proto__` writes the prototype, and every later lookup inherits
    // fields nobody authored. A Map key is plain data.
    const index = modalityManifestById({
      ...buildModalityManifest([]),
      lessons: [
        { id: "__proto__", drivable: false } as unknown as ModalityManifestLesson,
        { id: "ES-C01-a", drivable: true } as unknown as ModalityManifestLesson,
      ],
    });
    expect(index.get("__proto__")?.drivable).toBe(false);
    expect(index.get("ES-C01-a")?.drivable).toBe(true);
    expect(index.get("toString")).toBeUndefined();
    expect(Object.prototype.hasOwnProperty.call({}, "drivable")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// The real corpus
// ---------------------------------------------------------------------------

describe("the script strand is declared, not inferred", () => {
  // A spoken-only edition has to know which lessons are writing lessons. `type: writing`
  // already implies it, but a book target should not have to know that `type` doubles as
  // a strand marker, so writing lessons declare `delivery: script` outright and the
  // manifest carries it.
  //
  // Every assertion below runs through `buildModalityManifest`, not the frontmatter,
  // because the manifest is what a consumer reads. Reading `lesson.frontmatter.delivery`
  // here would pass even if the export were deleted or wired to the wrong field.
  const built = () => buildModalityManifest(loadEverything().lessons);

  it("never marks a lesson that is not a writing lesson, in any track", () => {
    const { lessons } = loadEverything();
    const type = new Map(lessons.map((l) => [l.realization.lessonId, l.realization.type]));
    const misapplied = buildModalityManifest(lessons)
      .lessons.filter((entry) => entry.delivery !== undefined)
      .filter((entry) => type.get(entry.id) !== "writing")
      .map((entry) => entry.id);
    expect(misapplied).toEqual([]);

    // `delivery` stays a two-state field: "script", or absent. A typo like `sript`
    // reaches the published manifest unchallenged otherwise — nothing in validate.ts
    // constrains it.
    const values = new Set(
      built()
        .lessons.map((entry) => entry.delivery)
        .filter((value) => value !== undefined),
    );
    expect([...values]).toEqual(["script"]);
  });

  it("covers every writing lesson of every track that has adopted it", () => {
    const { lessons } = loadEverything();
    const manifest = buildModalityManifest(lessons);
    const marked = manifest.lessons.filter((entry) => entry.delivery === "script");
    expect(marked.length).toBeGreaterThan(0);

    const language = new Map(lessons.map((l) => [l.realization.lessonId, l.language]));
    const adopted = new Set(marked.map((entry) => language.get(entry.id)));
    for (const track of adopted) {
      const writing = lessons
        .filter((l) => l.language === track && l.realization.type === "writing")
        .map((l) => l.realization.lessonId)
        .sort();
      const declared = marked
        .filter((entry) => language.get(entry.id) === track)
        .map((entry) => entry.id)
        .sort();
      expect(declared).toEqual(writing);
      // Guard the vacuous case per track, not just globally: a track that adopted the
      // marker and then lost every one of it would otherwise drop out of `adopted`.
      expect(declared.length).toBeGreaterThan(0);
    }
  });

  it("carries the marker into the committed manifest, not just a freshly built one", () => {
    const fresh = built().lessons.filter((entry) => entry.delivery === "script").map((e) => e.id);
    const committed = loadModalityManifest()
      .lessons.filter((entry) => entry.delivery === "script")
      .map((entry) => entry.id);
    expect(fresh.length).toBeGreaterThan(0);
    expect(committed.sort()).toEqual([...fresh].sort());
    // Absent, not empty, on everything else — the manifest's house rule for optionals.
    expect(loadModalityManifest().lessons.filter((entry) => "delivery" in entry).length).toBe(
      fresh.length,
    );
  });
});

describe("corpus regression", () => {
  // Exact corpus state is checked per language in tests/corpus/*.test.ts.
  // This shared file keeps only size-independent manifest invariants.
  it("keeps every rollup internally consistent", () => {
    const { lessons } = loadEverything();
    const manifest = buildModalityManifest(lessons);
    expect(manifest.summary.totalLessons).toBe(manifest.lessons.length);
    let trackLessons = 0;
    for (const track of manifest.tracks) {
      expect(track.voice + track.sight + track.pen).toBe(track.lessonCount);
      trackLessons += track.lessonCount;
      for (const chapter of track.chapters) {
        expect(chapter.voice + chapter.sight + chapter.pen).toBe(chapter.lessonCount);
        expect(chapter.drivableLessonIds).toHaveLength(chapter.drivablePrefix);
        expect(chapter.drivablePrefix).toBeLessThanOrEqual(chapter.lessonCount);
        expect(chapter.drivable).toBe(chapter.drivablePrefix === chapter.lessonCount);
        for (const id of chapter.drivableLessonIds) {
          expect(manifest.lessons.find((entry) => entry.id === id)?.drivable).toBe(true);
        }
      }
    }
    expect(trackLessons).toBe(manifest.summary.totalLessons);
    expect(manifest.lessons.filter((entry) => entry.drivable)).toHaveLength(
      manifest.summary.drivableLessons,
    );
  });

  it("keeps the committed manifest in step with the lessons", () => {
    // The gate itself, run against the real curriculum. If this fails, run
    // `npm run generate:modality` and commit the result — exactly what CI will say.
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runModalityManifest(["--check"])).toBe(0);
    const { lessons } = loadEverything();
    expect(loadModalityManifest()).toEqual(buildModalityManifest(lessons));
  });

  it("carries no unexplained overrides", () => {
    const { lessons } = loadEverything();
    expect(buildModalityManifest(lessons).findings).toEqual([]);
  });
});
