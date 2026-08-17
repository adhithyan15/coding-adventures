import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { afterEach, describe, expect, it, vi } from "vitest";
import { loadEverything, loadModalityManifest, modalityManifestById } from "../src/loader.js";
import { generatedModalityOutputs, runModalityManifest, safeOutput } from "../src/modality-cli.js";
import {
  MODALITY_MANIFEST_PATH,
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
    const output = join(root, "core", "lesson-modality.json");
    expect(existsSync(output)).toBe(true);
    expect(process.stdout.write).toHaveBeenCalledWith("generated core/lesson-modality.json\n");

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
      "core/lesson-modality.json: generated output is missing or stale\n",
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

    const output = join(root, "core", "lesson-modality.json");
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
    expect([...outputs.keys()]).toEqual([MODALITY_MANIFEST_PATH]);
    expect(outputs.get(MODALITY_MANIFEST_PATH)).toContain('"algorithm": "fnv1a64"');
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
    expect(safeOutput(root, MODALITY_MANIFEST_PATH)).toBe(join(root, "core", "lesson-modality.json"));
    expect(generatedModalityOutputs(root, "core/driving-edition.json").has("core/driving-edition.json")).toBe(
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
  // A pinned measurement, not a taste test. Everything upstream — the Markdown parser,
  // the table detector, the cue list, the chapter grouping — feeds these numbers, so a
  // silent change in any of them (most dangerously a block field rename that makes
  // every lesson look clean) moves them and fails here rather than shipping a
  // curriculum falsely advertised as drivable.
  //
  // This is now the ONLY place the corpus totals are pinned absolutely. `modality.test.ts`
  // used to carry a mirror of them and no longer does — it was rewritten to assert
  // size-independent invariants precisely because every content branch had to edit the
  // same three lines and collided there. Drift protection lives here and in the generated
  // `core/lesson-modality.json`, which `check:modality` compares byte for byte.
  //
  // The post-HL-C32 baseline was 1,096 lessons: 51 `pen`, 708 `voice`, 337 `sight`, 65%
  // drivable, 551 reachable in prerequisite order. HL-C24 then added four Latin
  // chapter-payoff lessons (ch19, ch21, ch33, ch36). All four are terminal consolidation
  // lessons built only from already-taught material — no table, no sight cue, no pen — so
  // they land as `voice` and move exactly four counters by exactly four:
  //
  //   totalLessons  1096 -> 1100      sight   337 -> 337  (unchanged)
  //   voice          708 ->  712      pen      51 ->  51  (unchanged)
  //   drivableLessons 708 -> 712      chapters 375 -> 375 (unchanged — no new chapters)
  //   drivablePrefixTotal 551 -> 555
  //
  // `drivablePrefixTotal` moves with them because each payoff is appended to a chapter
  // whose prefix already ran to its end, so each extends by one. `fullyDrivableChapters`
  // holds at 199 for the same reason: appending a `voice` lesson cannot make a chapter
  // stop being fully drivable, nor make a blocked one start.
  //
  // HL-C18A then split the fifteen over-budget Spanish lessons into thirty-three
  // prerequisite-ordered micro-lessons, a net +18 (two of them `writing`):
  //
  //   totalLessons  1100 -> 1118      voice   712 -> 719  (+7)
  //   sight          337 ->  346      pen      51 ->  53  (+2, the two `writing` splits)
  //   drivablePercent 65 ->   64      drivablePrefixTotal 555 -> 557
  //   fullyDrivableChapters 199 -> 195
  //
  // TWO COUNTERS MOVE THE WRONG WAY, AND THAT IS THE HONEST RESULT, NOT A REGRESSION TO
  // PAPER OVER. Splitting a table-bearing lesson does not delete its table — it copies the
  // relevant rows into several of the micro-lessons, so one `sight` lesson becomes several.
  // That is why `sight` takes +9 of the +18 while `voice` takes only +7, why the drivable
  // share rounds down from 65% to 64%, and why four chapters that were fully drivable no
  // longer are. The gentle-ramp goal (zero over-budget Spanish lessons) is met; the tables
  // those splits inherited belong to HL-C17, which linearises or honestly reclassifies
  // them. Tuning the splits to protect this percentage would mean writing steeper lessons
  // to flatter a metric, which is the exact trade the ramp budget exists to refuse.
  // HL-C39 then added Mandarin Chinese as the 21st track, +7 Chapter 1 lessons:
  //
  //   totalLessons  1118 -> 1125      voice   719 -> 724  (+5)
  //   sight          346 ->  348      pen      53 ->  53  (unchanged)
  //   trackCount      20 ->   21      chapterCount 375 -> 376  (+1)
  //   drivablePrefixTotal 557 -> 558
  //
  // The two `sight` lessons are `ZH-C01-ni` and `ZH-C01-hao`, which each carry a `script`
  // block teaching a character's components — a shape cannot be read aloud. No Chinese
  // lesson needs a pen and none carries a table, so `pen` holds and the drivable share
  // stays at 64%. `drivablePrefixTotal` gains only 1 because Chinese ch1 opens with one
  // `voice` lesson before its first character-composition lesson blocks the prefix.
  // HL-C40 then added Japanese as the 22nd track, +8 Chapter 1 lessons — and its shape
  // is the inverse of Chinese's, which is the finding worth keeping:
  //
  //   totalLessons  1125 -> 1133      voice   724 -> 725  (+1 only)
  //   sight          348 ->  355      pen      53 ->  53  (unchanged)
  //   trackCount      21 ->   22      chapterCount 376 -> 377  (+1)
  //   drivablePrefixTotal 558 -> 558  unstartableChapters 121 -> 122  (+1)
  //
  // Seven of the eight Japanese lessons carry a `script` block and therefore derive as
  // `sight`: a kana or kanji shape cannot be read aloud. Only the practice lesson is
  // `voice`. Because the very first lesson of Japanese ch1 is one of the seven, the
  // chapter's drivable prefix is 0 — which is why `drivablePrefixTotal` does not move at
  // all and `unstartableChapters` gains one. Routing that content through `input` blocks
  // would have held the drivable share flat by mislabelling it; the honest classification
  // is the one that costs the metric.
  //
  // HL-C05-bolta-hun (the Hindi present-habitual paradigm, split out of the assembly
  // lesson) then added one `voice` lesson: 1133 -> 1134, voice 956 -> 957. It is `voice`
  // deliberately — its paradigm grid was narrowed from four columns to three so the
  // narration lineariser can read it, and a \"once you see them as\" aside was reworded,
  // because a lesson teaching the engine of the present tense is exactly the kind a
  // commuter should not be locked out of.
  // HL-C16 then built the narration lineariser and moved the shipped table width from
  // 0 to 3. This is the largest single move the corpus has ever taken, and none of it
  // is new content — it is the same 1,133 lessons, re-judged by a detector that can now
  // actually say what it means to speak a table aloud:
  //
  //   voice   725 -> 956  (+231)      sight  355 -> 124  (-231)
  //   pen      53 ->  53  (unchanged) totalLessons 1133 (unchanged)
  //   drivablePercent 64 -> 84
  //   drivablePrefixTotal   558 -> 824
  //   fullyDrivableChapters 195 -> 284
  //   unstartableChapters   122 ->  44
  //
  // Every lesson that moved went `sight` -> `voice` and nothing else changed: the +231
  // on `voice` is exactly the -231 on `sight`, `pen` is untouched, and no lesson was
  // created or lost. `modality.test.ts` asserts that equality directly, at both widths,
  // so this snapshot is corroborated by a size-independent control rather than standing
  // alone. The prefix and chapter rollups move much further than the raw counts because
  // a single unspeakable table near the front of a chapter used to block everything
  // behind it — which is why unstartable chapters fall by nearly two thirds.
  //
  // The Latin core-verb chapter (chapter 37: sum, habeō, eō, veniō, dīcō, videō, sciō,
  // dō) then added eight lessons, and every one of them is `voice`:
  //
  //   totalLessons  1134 -> 1142      voice   957 -> 965  (+8)
  //   sight          124 ->  124      pen      53 ->  53  (both unchanged)
  //   drivableLessons 957 ->  965     drivablePercent 84 -> 85
  //   chapterCount    377 ->  378     fullyDrivableChapters 284 -> 285
  //   drivablePrefixTotal 825 -> 833  unstartableChapters 44 -> 44 (unchanged)
  //
  // All eight counters that move, move together and by the same eight, which is the
  // signature of a chapter that needs no eyes at all: each lesson teaches one verb, its
  // six present-tense forms as a bullet list rather than a paradigm grid, and its English
  // cousins in prose. No table, no script block, no pen. Because the whole chapter is
  // `voice` and it is a NEW chapter, its drivable prefix runs to its full length — so
  // `drivablePrefixTotal` gains the full 8 and `fullyDrivableChapters` gains the one new
  // chapter, while `unstartableChapters` cannot move: an all-voice chapter is startable
  // by definition. The drivable share crossing from 84% to 85% is a real ratchet, not a
  // rounding accident — 965/1142 is 84.5%, which rounds up.
  // The whole-lesson figures fell when the inline-letters section became the `script`
  // block it always was (231 lessons across 12 tracks): voice 1011 -> 780, sight 124 ->
  // 355. THE BOOK IS NOW HONEST AND THE DRIVER LOST NOTHING — `coreModality` sets those
  // detachable sections aside, so the driving edition reads 1,026 lessons (86%), above
  // the 84% that stood before the reclassification. This snapshot is the book's number;
  // `modality.test.ts` asserts the core relationship.
  // Sight cues then moved to word-boundary matching: voice 798 -> 805, sight 355 -> 348.
  // Seven lessons had been marked `sight` by a cue matching INSIDE a longer word
  // (`columns` matching `column`). No lesson lost a real cue — the control assertions in
  // `modality.test.ts` pin that instructions still fire.
  it("pins the corpus summary the manifest publishes", () => {
    const { lessons } = loadEverything();
    const manifest = buildModalityManifest(lessons);
    // The second verb tranche then added 24 lessons — the same eight verbs (THINK,
    // UNDERSTAND, READ, WRITE, TAKE, ASK, HELP, LIKE-LOVE) in Spanish, Latin and
    // Portuguese, authored in parallel. All 24 derive `voice`, so `sight` and `pen` do
    // not move at all and the drivable share ratchets 67% -> 68%. Six chapters, not
    // three: eight one-verb-per-lesson lessons introduce ~17 atoms against
    // `maxNewAtomsPerChapter: 12`, so each track ships the tranche as a PAIR of
    // four-lesson chapters. That is the budget working as intended — it was fitted to
    // chapters that teach a topic, and a verb tranche is a denser shape — and splitting
    // is the honest fix rather than raising the threshold. Page count is never a cost.
    // Wave 7 then took the same eight verbs to French, German, Italian and Hindi — 32
    // lessons, eight chapters, so all eight concepts are now SEVEN-way cross-language
    // joins. `sight` moves for the first time in these tranches (348 -> 352): four Hindi
    // lessons genuinely teach a Devanagari letter (झ, ढ़ with nuqtā, the preposed ि,
    // छ/ू) under the canonical `## The letters in this word` heading. Those blocks are
    // DETACHABLE, so `coreModality` stays `voice` and both Hindi chapters remain 4-of-4
    // drivable — the book is honest and the driver loses nothing, which is exactly the
    // arrangement #10011/#10012 were built to make possible.
    // Wave 8 (Arabic, Russian, Tamil, Bengali) then took the eight to ELEVEN tracks.
    //
    // `sight` jumps 352 -> 376 and `unstartableChapters` 90 -> 96, and this is the
    // `sight`-penalty seam, not a regression. Three of these four tracks are non-Latin,
    // so their lessons carry a `## The letters in this word` block; those blocks are
    // DETACHABLE, so every one of the 32 keeps `coreModality: voice` and the driving
    // edition is untouched — core drivability actually ROSE in each track (Bengali
    // 97->98%, Tamil 84->86%, Russian 79->83%, Arabic 73->75%).
    //
    // The two counters genuinely disagree by design, which is worth knowing before
    // reading either: `modality-manifest.ts` computes `unstartableChapters` from FULL
    // modality, while `modality.ts`'s `drivablePrefix` — what the gap report publishes —
    // uses CORE. Script blocks land exactly on that seam. Publishing `coreVoice` as the
    // headline per-track number is the standing recommendation; until then, expect this
    // figure to rise whenever a non-Latin track authors honestly.
    // +1 lesson and +1 `pen`: TA-W19-read-muunru. It carries `type: writing`, so it
    // derives `pen` from `writing-type` before its script block is even considered —
    // the same `["writing-type","script-block"]` pair that, counted in
    // `core/lesson-modality.json`, 20 other Tamil lessons already carry. No other
    // counter moves, so the sight seam above is untouched by this lesson.
    // Chapter 39 moves five counters and no others. +4 `totalLessons` and +1
    // `chapterCount` are the chapter itself. `pen` +1 is TA-W20-read-onru, from
    // `writing-type`. `sight` +3 is the three speaking lessons, each carrying a
    // detachable `## The letters in this word` block — so `coreDrivable` is untouched
    // and the chapter opening still reads "first 3 of 4 lessons".
    // `unstartableChapters` +1 is the same seam described above: that counter is
    // computed from FULL modality, where the chapter's first lesson is already
    // sight-dependent. The CORE prefix, which the driving edition actually uses, is 3.
    // HL11 moved exactly TWO of these, and the ones that did NOT move are the
    // point. Tamil's nine drizzled letter segments are pen lessons -- you cannot
    // learn a letter's shape by ear -- so `totalLessons` and `pen` each rise by
    // nine.
    //
    // `drivableLessons`, `drivablePercent`, `drivablePrefixTotal`,
    // `fullyDrivableChapters` and `unstartableChapters` are ALL unchanged. Not
    // one existing lesson became undrivable, no chapter lost a lesson from the
    // prefix a commuter can do before hitting something that needs eyes, and no
    // chapter became impossible to start. That is HL11's own falsification test
    // and it passes exactly.
    //
    // It passes because of WHERE the segments sit, and an earlier revision
    // proved that the hard way. Placed in chapters 1-3 they cost 13 prefix
    // lessons and two fully-drivable chapters, and one landed at sequence 175 --
    // making it the first lesson of chapter 3 and leaving that chapter
    // impossible to begin in the car, which `unstartableChapters` caught at 174.
    // Each now sits immediately before the word-writing lesson that uses its
    // letter, inside a chapter whose prefix a writing lesson had already ended.
    //
    // HL12 then added 30 recognition segments to Telugu, Kannada, Malayalam and
    // Sanskrit, and the same falsification test still passes on the number that
    // matters: `drivablePrefixTotal` is UNCHANGED at 1136. Not one existing
    // lesson fell out of the run a commuter can do before hitting something that
    // needs eyes, and `unstartableChapters` holds at 173 -- no segment opens a
    // chapter.
    //
    // Two numbers do move, and both are the honest cost rather than a
    // regression. `pen` rises by exactly the 30 new segments, taking
    // `drivablePercent` from 68 to 67 -- a larger denominator, not fewer
    // drivable lessons, which is why `drivableLessons` is unchanged at 1336. And
    // `fullyDrivableChapters` falls 489 -> 472, because a chapter that teaches a
    // letter now contains something that cannot be done at the wheel. It is
    // 17 rather than 30 because 13 of those chapters already held a sight lesson.
    //
    // Placing each segment LAST in its chapter is what buys the unchanged
    // prefix. Second-in-chapter was measured too and cost 11 prefix lessons: the
    // prefix ends at the first lesson needing eyes, so a segment at the front
    // truncates its whole chapter and one at the back truncates nothing.
    //
    // HL12 payment two adds Hindi's eight, and the shape repeats exactly:
    // `drivablePrefixTotal` still 1136, `unstartableChapters` still 173,
    // `drivableLessons` still 1336. `pen` takes the eight new segments 108 -> 116,
    // carrying `drivablePercent` 67 -> 66 on the denominator alone, and
    // `fullyDrivableChapters` falls 472 -> 465 for the seven Hindi chapters that
    // now contain something a commuter cannot do.
    expect(manifest.summary).toEqual({
      // Both Spanish B1 chapters, 38 and 41, are entirely ear-only, so each is fully
      // drivable. They moved the whole-corpus figure from 66% to 67% — then the
      // pre-A1 vocabulary probe (hindi/arabic/tamil) added honest script sections
      // under the canonical heading, and the whole-lesson figure fell back to 66%.
      // `coreDrivable` is unaffected: those blocks are detachable, so the driving
      // edition itself lost nothing. This is the sight-share seam, not a regression.
      // Vocabulary wave 4 (marathi/punjabi/sanskrit/urdu, 51 pre-A1 nouns) is the same
      // seam again, and it is why voice, sight AND drivablePercent all move together:
      // most of the wave is ear-only `voice`, but Sanskrit's Devanagari citations and
      // several tracks' honest `## The letters in this word` blocks add `sight`, so
      // the whole-lesson share dips 66% -> 65% even though every one of these blocks
      // is detachable and `coreDrivable` again loses nothing.
      // Chapter 15 splits two legacy teaching lessons into five bounded teaching
      // steps. All five remain voice-first; only the mapped terminal comparison is
      // sight-dependent, so the split adds three lessons without widening the seam.
      // Chapter 16 replaces three legacy lessons with eight bounded steps.
      // Chapter 17 replaces four legacy lessons with eight bounded steps.
      // Chapter 18 replaces ten legacy lessons with nine bounded steps.
      // +4: TA-W10-read-naan, TA-W11-read-niingal, TA-W12-read-eppadi and
      // TA-W13-read-irukkirirgal extend the writing strand over chapters 2-3's glyphs.
      // +3: TA-W14-read-pesu, TA-W15-read-po and TA-W16-read-tamizh close chapters 4-5.
      // +2: TA-W17-read-unavu and TA-W18-read-uur close the last two glyphs untaught
      // in the chapter 33-38 sections — NOT in the corpus, which this entry originally
      // failed to qualify. FOURTEEN chapter-7 glyphs are still untaught after them.
      // +1: TA-W19-read-muunru teaches one of the fourteen, ூ, leaving thirteen at
      // that point — ஏ, ஐ, ஒ and the ten digits ௧-௰ — and exhausting the runway that
      // existed after it. Chapter 39 below then extends the track and teaches ஒ,
      // leaving twelve.
      totalLessons: 2823, // tamil pre-A1 tranche: +35 lessons, +7 chapters (chapters 44-50) // HL-C200: +35 telugu pre-A1 lessons, +7 chapters (chapters 46-52) // HL: +35 -- Sanskrit chapters 24-30, 35 pre-A1 vocabulary lessons // HL-C194: +16 Spanish words // HL-C192: +24 family words // HL-C190: see/say verbs across four tracks // HL-C189: +8 -- Tamil and Sanskrit verb tranche // HL-C187: +20 -- verb tranche across the five behind tracks // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C175: +5 -- chapter 272, reading between the lines // HL-C173: +3 -- B2 closes (chapter 271) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C168: +1 -- Kannada's ledger closes at 24 of 24 // HL-C166: +11 -- Sanskrit chapters 19 and 20 // +3: HL-C97 adds the repair kit (no entiendo, mas despacio) at chapter 14 // +40: vocabulary wave 5 (persian 12, telugu 13, malayalam 15) // +4: HL-C88 slices 5-6 // +1: HL-C88 slice 7 (ES-C09-ncia) // +3: HL-C88 slice 8 (-ario, review, synthesis) // +54: vocabulary wave 6 (russian 14, persian 14, urdu 13, bengali 13) // +1: HL-C88 slice 9 (falsos amigos) // +3: B1 si-condition rung // +3: HL-C113 preterite plural // +4: HL-C113 preterite close (strong plurals, review, synthesis) // +2: HL-C113 imperfect subjunctive // +3: HL-C113 unreal condition // HL-C113 step 7: +4 // HL-C113 step 8: +3 // HL-C128 step 2: +5 // HL-C128 step 3: +4 // HL-C128 step 4: +6 // HL-C128 step 5: +5 // HL-C127: +5 // HL-C128 step 7: +5 // HL-C128 step 8: +6 // HL-C128 step 9: +5 // HL-C128 step 10: +5 // HL-C136 wave I: +42, seven lessons in each of six new chapters // HL-C137 wave II: +36 adjective lessons, +6 chapters, and drivablePercent rises AGAIN // HL-C152: Spanish realizes SPINE-NEGATE-AND-ASK — five lessons, one chapter, A2 complete at 5/5 // HL-C157: +2 // HL-C156: 85 script segments across the six Indic tracks // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // HL-C163: +6 -- Sanskrit chapter 16 // HL-C165: +11 -- Sanskrit chapters 17 and 18 // kannada pre-A1 tranche: +35 lessons, +7 chapters (chapters 46-52) // malayalam pre-A1 tranche: 35 lessons over 7 chapters -- voice +34 and sight +1 sum to 35; the prefix figures follow from the per-chapter walk // hindi pre-A1 tranche: +35 lessons, +7 chapters (chapters 45-51) // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // sanskrit pre-A1 round 2: +35 lessons, +7 chapters (chapters 31-37) // telugu pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // kannada pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295) // chinese script chapter: +7 lessons, +1 chapter -- the seven components 你好 is built from, one glyph each // japanese hiragana tranche: +10 lessons, +1 chapter -- eight signs, one per lesson, plus two assemblies that introduce none
      // Chapters 10 and 13 replace wide legacy tables with small singular-only
      // comparisons, so three more lessons move from sight to voice.
      // All seven Chapter-16 teaching steps are voice-first. Generating the book
      // exposed the terminal Chapter-15 and Chapter-16 recap tables as too wide,
      // so their same person-by-person comparisons now use speakable bullet rows.
      // All eight Chapter-17 lessons remain voice-first.
      // +6, and it is the same six lessons that leave `sight` below.
      // +8, the eight chapter 4-5 lessons that drop their inline script sections.
      voice: 2024, // tamil pre-A1 tranche: +35 lessons, +7 chapters (chapters 44-50) // HL-C200: +35 telugu pre-A1 lessons, +7 chapters (chapters 46-52) // HL: +35 -- Sanskrit chapters 24-30, 35 pre-A1 vocabulary lessons // HL-C194: +16 Spanish words // HL-C192: +24 family words // HL-C190: see/say verbs across four tracks // HL-C189: +8 -- Tamil and Sanskrit verb tranche // HL-C187: +20 -- verb tranche across the five behind tracks // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C175: +5 -- chapter 272, reading between the lines // HL-C173: +3 -- B2 closes (chapter 271) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C166: +11 -- Sanskrit chapters 19 and 20 // HL-C157 // +35: vocabulary wave 5, mostly ear-only lessons // +3: HL-C88 slices 5-6 // +1: HL-C88 slice 7 (ES-C09-ncia) // +3: HL-C88 slice 8 (-ario, review, synthesis) // +36: vocabulary wave 6 // +1: HL-C88 slice 9 (falsos amigos) // +3: B1 si-condition rung // +3: HL-C113 preterite plural // +4: HL-C113 preterite close (strong plurals, review, synthesis) // +2: HL-C113 imperfect subjunctive // +3: HL-C113 unreal condition // HL-C113 step 7: +2 // HL-C113 step 8: +3 // HL-C128 step 2: +4 // HL-C128 step 3: +3 // HL-C128 step 4: +6 // HL-C128 step 5: +4 // HL-C127: +3 // HL-C128 step 7: +3 // HL-C128 step 8: +6 // HL-C128 step 9: +5 // HL-C128 step 10: +5 // HL-C136 wave I: +42 -- ALL of it. Not one of the 42 needs eyes, so `sight` and `pen` below do not move at all // HL-C137 wave II: +36 adjective lessons, +6 chapters, and drivablePercent rises AGAIN // HL-C152: Spanish realizes SPINE-NEGATE-AND-ASK — five lessons, one chapter, A2 complete at 5/5 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // HL-C163: +6 -- Sanskrit chapter 16 // HL-C165: +11 -- Sanskrit chapters 17 and 18 // kannada pre-A1 tranche: +35 lessons, +7 chapters (chapters 46-52) // malayalam pre-A1 tranche: 35 lessons over 7 chapters -- voice +34 and sight +1 sum to 35; the prefix figures follow from the per-chapter walk // hindi pre-A1 tranche: +35 lessons, +7 chapters (chapters 45-51) // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // sanskrit pre-A1 round 2: +35 lessons, +7 chapters (chapters 31-37) // telugu pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // kannada pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295)
      // -3 sight / +3 voice: TA-C02-en, -enna and -peyar dropped their "The letters in
      // this word" sections once TA-W08 and TA-W09 gave those glyphs a home in the
      // strand. Verified against the GENERATED manifest, not the source — all three now
      // record reasons ["no-visual-dependency"], so the script teaching genuinely left
      // the lesson rather than a heading being renamed out from under the classifier.
      // Chapter 18 removes its one remaining sight-only lesson. Publishing
      // Chapters 7-18 from the AST moves the Chapter-15 and Chapter-16 terminal
      // recaps from sight to voice without changing what either checkpoint asks.
      // -6 sight / +6 voice, the ch02/ch03 repeat of the TA-C02 move above: once
      // TA-W10..TA-W13 gave chapters 2-3's glyphs a home, TA-C02-nii-niingal and all
      // five ch3 lessons (eppadi, eppadi-irukkirirgal, naan, nalam, paravayillai)
      // dropped their "The letters in this word" sections. Verified against the
      // GENERATED manifest: every one flips reasons ["script-block"] ->
      // ["no-visual-dependency"] and its detachableSegments list empties, so the
      // script teaching genuinely left the lesson rather than a heading being renamed.
      // -8 sight / +8 voice, the ch04/ch05 repeat of the same move: TA-C04-po,
      // -poy-varugiren, -naalai, -mindum-sandippom and TA-C05-pesu, -velai-sey, -vaazh,
      // -naan-tamizh-pesugiren all flip ["script-block"] -> ["no-visual-dependency"]
      // with an empty detachableSegments, verified against the GENERATED manifest.
      sight: 580, // HL-C200: +35 telugu pre-A1 lessons, +7 chapters (chapters 46-52) // HL: +35 -- Sanskrit chapters 24-30, 35 pre-A1 vocabulary lessons // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C175: +5 -- chapter 272, reading between the lines // HL-C166: +11 -- Sanskrit chapters 19 and 20 // HL-C157 // +5: vocabulary wave 5's honest cousin-script citations in a handful of lessons // +1: HL-C88 slices 5-6 // +18: vocabulary wave 6 // HL-C113 step 7: +2 -- 214 (question marks) and 215 (the accent) cannot be taught by voice // HL-C128 step 2: +1 -- ch223 // HL-C128 step 3: +1 // HL-C128 step 5: +1 // HL-C127: +2 -- ch243 and ch244 turn on written accents, which cannot be heard // kannada pre-A1 tranche: +35 lessons, +7 chapters (chapters 46-52) // malayalam pre-A1 tranche: 35 lessons over 7 chapters -- voice +34 and sight +1 sum to 35; the prefix figures follow from the per-chapter walk
      // +3 pen: the Tamil ch1 writing lessons. Rule 1 in src/modality.ts derives
      // this from the lesson TYPE alone — it says outright that it does not look at
      // the body — so all three record reasons ["writing-type","script-block"], and
      // TA-W07 is pen even though it deliberately gives no stroke instructions.
      // voice and drivable are unchanged, which is what a writing lesson should do
      // to this table.
      // +2: both new lessons are writing lessons.
      // +4: TA-W10..TA-W13 are `type: writing` too, so rule 1 makes them pen even
      // though all four are reading-only and give no stroke order.
      // +3: TA-W14/15/16 are `type: writing` too.
      // +2: both new lessons are `type: writing`. voice, sight, drivableLessons,
      // drivablePrefixTotal, fullyDrivableChapters and unstartableChapters ALL hold —
      // this tranche adds lessons and removes no inline section, so nothing flips.
      pen: 219, // HL-C168: +1 -- Kannada's ledger closes at 24 of 24 // HL-C156: 85 script segments across the six Indic tracks // chinese script chapter: +7 lessons, +1 chapter -- the seven components 你好 is built from, one glyph each // japanese hiragana tranche: +10 lessons, +1 chapter -- eight signs, one per lesson, plus two assemblies that introduce none
      // +6: exactly the six lessons that moved sight -> voice; no other lesson changes.
      // +8: exactly the eight lessons that moved sight -> voice.
      drivableLessons: 2024, // tamil pre-A1 tranche: +35 lessons, +7 chapters (chapters 44-50) // HL-C200: +35 telugu pre-A1 lessons, +7 chapters (chapters 46-52) // HL: +35 -- Sanskrit chapters 24-30, 35 pre-A1 vocabulary lessons // HL-C194: +16 Spanish words // HL-C192: +24 family words // HL-C190: see/say verbs across four tracks // HL-C189: +8 -- Tamil and Sanskrit verb tranche // HL-C187: +20 -- verb tranche across the five behind tracks // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C175: +5 -- chapter 272, reading between the lines // HL-C173: +3 -- B2 closes (chapter 271) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C166: +11 -- Sanskrit chapters 19 and 20 // HL-C157 // +35: vocabulary wave 5 // +3: HL-C88 slices 5-6 // +1: HL-C88 slice 7 (ES-C09-ncia) // +3: HL-C88 slice 8 (-ario, review, synthesis) // +36: vocabulary wave 6 // +1: HL-C88 slice 9 (falsos amigos) // +3: B1 si-condition rung // +3: HL-C113 preterite plural // +4: HL-C113 preterite close (strong plurals, review, synthesis) // +2: HL-C113 imperfect subjunctive // +3: HL-C113 unreal condition // HL-C113 step 7: +2 -- the review and synthesis narrate; 214 and 215 are sight-cue, being ABOUT written marks // HL-C113 step 8: +3 // HL-C128 step 2: +4 -- ch223 is sight-cue, being about the letters visible inside aquel // HL-C128 step 3: +3 // HL-C128 step 4: +6 -- all six narrate // HL-C128 step 5: +4 // HL-C127: +3 // HL-C128 step 7: +3 -- all five narrate // HL-C128 step 8: +6 -- ch252 and ch255 are sight-cue -- both turn on written accents // HL-C128 step 9: +5 // HL-C128 step 10: +5 // HL-C136 wave I: +42, the same 42 // HL-C137 wave II: +36 adjective lessons, +6 chapters, and drivablePercent rises AGAIN // HL-C152: Spanish realizes SPINE-NEGATE-AND-ASK — five lessons, one chapter, A2 complete at 5/5 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // HL-C163: +6 -- Sanskrit chapter 16 // HL-C165: +11 -- Sanskrit chapters 17 and 18 // kannada pre-A1 tranche: +35 lessons, +7 chapters (chapters 46-52) // malayalam pre-A1 tranche: 35 lessons over 7 chapters -- voice +34 and sight +1 sum to 35; the prefix figures follow from the per-chapter walk // hindi pre-A1 tranche: +35 lessons, +7 chapters (chapters 45-51) // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // sanskrit pre-A1 round 2: +35 lessons, +7 chapters (chapters 31-37) // telugu pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // kannada pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295)
      drivablePercent: 72, // HL-C194: +16 Spanish words, all drivable // kannada pre-A1 tranche: +35 lessons, +7 chapters (chapters 46-52) // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // sanskrit pre-A1 round 2: +35 lessons, +7 chapters (chapters 31-37) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295)
      trackCount: 22,
      chapterCount: 901, // tamil pre-A1 tranche: +35 lessons, +7 chapters (chapters 44-50) // HL-C200: +35 telugu pre-A1 lessons, +7 chapters (chapters 46-52) // HL: +35 -- Sanskrit chapters 24-30, 35 pre-A1 vocabulary lessons // HL-C194: +16 Spanish words // HL-C192: +24 family words // HL-C190: see/say verbs across four tracks // HL-C189: +8 -- Tamil and Sanskrit verb tranche // HL-C187: +20 -- verb tranche across the five behind tracks // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C175: +5 -- chapter 272, reading between the lines // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C166: +11 -- Sanskrit chapters 19 and 20 // +15: vocabulary wave 5 (persian +3, telugu +6, malayalam +6) // +4: HL-C88 slices 5-6 // +1: HL-C88 slice 7 (ES-C09-ncia) // +3: HL-C88 slice 8 (-ario, review, synthesis) // +12: vocabulary wave 6 // +1: HL-C88 slice 9 (falsos amigos) // +3: B1 si-condition rung // +3: HL-C113 preterite plural // +4: HL-C113 preterite close (strong plurals, review, synthesis) // +2: HL-C113 imperfect subjunctive // +3: HL-C113 unreal condition // HL-C113 step 7: +4 // HL-C113 step 8: +3 // HL-C128 step 2: +5 // HL-C128 step 3: +4 // HL-C128 step 4: +6 // HL-C128 step 5: +5 // HL-C127: +5 // HL-C128 step 7: +5 // HL-C128 step 8: +6 // HL-C128 step 9: +5 // HL-C128 step 10: +5 // HL-C136 wave I: +6, one chapter per Indic track // HL-C137 wave II: +36 adjective lessons, +6 chapters, and drivablePercent rises AGAIN // HL-C152: Spanish realizes SPINE-NEGATE-AND-ASK — five lessons, one chapter, A2 complete at 5/5 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C163: +6 -- Sanskrit chapter 16 // HL-C165: +11 -- Sanskrit chapters 17 and 18 // kannada pre-A1 tranche: +35 lessons, +7 chapters (chapters 46-52) // malayalam pre-A1 tranche: 35 lessons over 7 chapters -- voice +34 and sight +1 sum to 35; the prefix figures follow from the per-chapter walk // hindi pre-A1 tranche: +35 lessons, +7 chapters (chapters 45-51) // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // sanskrit pre-A1 round 2: +35 lessons, +7 chapters (chapters 31-37) // telugu pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // kannada pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295) // chinese script chapter: +7 lessons, +1 chapter -- the seven components 你好 is built from, one glyph each // japanese hiragana tranche: +10 lessons, +1 chapter -- eight signs, one per lesson, plus two assemblies that introduce none
      // Prerequisite order still costs a commuter 132 of the 965 ear-only lessons:
      // they sit behind a blocker in their own chapter and stay unreachable in the car
      // until HL-C17 reshapes the remaining wide tables.
      // A chapter's drivable prefix is "how far you get by ear from its START", so it
      // depends on ORDER. HL09 step 2 gave 50 Spanish lessons a declared `sequence`,
      // and the alphabetical fallback it replaced had been flattering this number by
      // putting eyes-needed lessons later than they really come. The real order is
      // worse, which is the measurement becoming honest, not the corpus regressing.
      // Two independent changes land on these three numbers in the same merge:
      // vocabulary wave 4 (16 new chapters, several needing eyes from their own first
      // lesson: 870 -> 873, 327 -> 328, 129 -> 142 claimed in isolation) and Tamil's
      // script-interleaving restructure (chapter 1 becomes fully drivable for the first
      // time; six other Tamil chapters each pick up one writing lesson: 870 -> 873,
      // 327 -> 322, 129 -> 129 claimed in isolation). Re-measured against the merged
      // corpus: drivablePrefixTotal 876 (both gains add), fullyDrivableChapters 323
      // (wave 4's +1 and Tamil's net -5 combine), unstartableChapters 142 (wave 4's
      // rise; Tamil's restructure did not touch this count in the merged state).
      // +2, and Tamil chapter 2 is the only chapter in the corpus that moves: 0 -> 2.
      // Measured, not reasoned — ch2 runs peyar(100), en(110), en-peyar(120), so
      // freeing the first two extends the ear-only run until en-peyar, which is still
      // sight and stops it. TA-C02-enna sits behind that blocker and became drivable
      // without adding to any prefix.
      // Chapter 7's real order begins with comer's four-column comparison rather
      // than the old alphabetical fallback's ear-only beber lesson.
      // Chapter 9 replaces three wide-table sight lessons with voice-first
      // micro-lessons and extends the Spanish chapter prefix through all five steps.
      // The Chapter-10 migration extends the safe prefix by two lessons; Chapter 13
      // adds four more reachable lessons by migrating its full terminal checkpoint.
      // Spanish Chapter 15 now offers five consecutive voice lessons before its
      // sight checkpoint, raising the useful prefix by four.
      // Chapter 16's reachable voice-first prefix grows by six.
      // Chapter 17's reachable voice-first prefix grows by five.
      // All nine redesigned Chapter-18 steps are reachable by ear. The terminal
      // Chapter-15 and Chapter-16 recaps now extend both prefixes by one.
      // +5 net, and TWO chapters move, in opposite directions. Tamil chapter 3 gains 6:
      // its first lesson was sight, so nothing after it counted; with all six now voice
      // the whole chapter is one ear-only run. Tamil chapter 25 LOSES 1 (2 -> 1), because
      // the 3:1 cadence puts TA-W10 between its two speaking lessons rather than after
      // them — the same mid-chapter placement TA-W06 already has in chapter 18. Chapters
      // 27/29/31 take their writing lesson after the prefix, so they hold at 1/1/2.
      // +6, and only two chapters move, both upward this time: Tamil chapter 4 gains 4
      // and chapter 5 gains 2. Nothing is lost, because TA-W14/15/16 are placed to skip
      // chapter 32 — see the ramp test for why — so no chapter's prefix is cut short.
      drivablePrefixTotal: 1802, // tamil pre-A1 tranche: +35 lessons, +7 chapters (chapters 44-50) // HL-C200: +35 telugu pre-A1 lessons, +7 chapters (chapters 46-52) // HL: +35 -- Sanskrit chapters 24-30, 35 pre-A1 vocabulary lessons // HL-C194: +16 Spanish words // HL-C192: +24 family words // HL-C190: see/say verbs across four tracks // HL-C189: +8 -- Tamil and Sanskrit verb tranche // HL-C187: +20 -- verb tranche across the five behind tracks // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C175: +5 -- chapter 272, reading between the lines // HL-C173: +3 -- B2 closes (chapter 271) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C166: +11 -- Sanskrit chapters 19 and 20 // HL-C157: the future-tense conjugation table is sight so ch49 leaves the drivable prefix, // +32: vocabulary wave 5's new chapters, mostly ear-only from their own start // +3: HL-C88 slices 5-6 // +1: HL-C88 slice 7 (ES-C09-ncia) // +3: HL-C88 slice 8 (-ario, review, synthesis) // +26: vocabulary wave 6 // +1: HL-C88 slice 9 (falsos amigos) // +3: B1 si-condition rung // +3: HL-C113 preterite plural // +4: HL-C113 preterite close (strong plurals, review, synthesis) // +2: HL-C113 imperfect subjunctive // +3: HL-C113 unreal condition // HL-C113 step 7: +2 // HL-C113 step 8: +3 // HL-C128 step 2: +4 // HL-C128 step 3: +3 // HL-C128 step 4: +6 // HL-C128 step 5: +4 // HL-C127: +3 // HL-C128 step 7: +3 // HL-C128 step 8: +6 // HL-C128 step 9: +5 // HL-C128 step 10: +5 // HL-C136 wave I: +42 -- each of the six chapters is drivable from its own first lesson to its last, so the prefix is the whole chapter // HL-C137 wave II: +36 adjective lessons, +6 chapters, and drivablePercent rises AGAIN // HL-C152: Spanish realizes SPINE-NEGATE-AND-ASK — five lessons, one chapter, A2 complete at 5/5 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // HL-C163: +6 -- Sanskrit chapter 16 // HL-C165: +11 -- Sanskrit chapters 17 and 18 // kannada pre-A1 tranche: +35 lessons, +7 chapters (chapters 46-52) // malayalam pre-A1 tranche: 35 lessons over 7 chapters -- voice +34 and sight +1 sum to 35; the prefix figures follow from the per-chapter walk // hindi pre-A1 tranche: +35 lessons, +7 chapters (chapters 45-51) // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // sanskrit pre-A1 round 2: +35 lessons, +7 chapters (chapters 31-37) // telugu pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // kannada pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295)
      // -2: chapters 21 and 23 each take a writing lesson and stop being ear-only.
      // Spreading the strand cannot happen without landing a pen lesson somewhere.
      // Spanish Chapters 10 and 13 are now fully drivable from their canonical ASTs.
      // Spanish Chapter 18 also becomes fully drivable. Replacing the two wide
      // terminal recap tables makes Spanish Chapters 15 and 16 fully drivable too.
      // -3, and it goes the WRONG way, which is worth stating plainly. Four Tamil
      // chapters (25, 27, 29, 31) each gain one writing lesson and stop being fully
      // drivable: -4. Tamil chapter 3 becomes fully drivable for the first time: +1.
      // Net -3. That is the honest cost of paying the script debt where it belongs.
      // Corpus `coreDrivable` does not move, but NOT because these blocks detach: rule 1
      // classifies a `type: writing` lesson as pen without reading its body, so all four
      // record `coreDrivable: false`. It holds because the six lessons that flipped were
      // already core-drivable and the four new ones were never counted.
      fullyDrivableChapters: 588, // tamil pre-A1 tranche: +35 lessons, +7 chapters (chapters 44-50) // HL-C200: +35 telugu pre-A1 lessons, +7 chapters (chapters 46-52) // HL: +35 -- Sanskrit chapters 24-30, 35 pre-A1 vocabulary lessons // HL-C194: +16 Spanish words // HL-C192: +24 family words // HL-C190: see/say verbs across four tracks // HL-C189: +8 -- Tamil and Sanskrit verb tranche // HL-C187: +20 -- verb tranche across the five behind tracks // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C179: +5 -- chapter 275, fine shades // HL-C178: +5 -- chapter 274, C2 opens // HL-C177: +5 -- chapter 273, C1 closes // HL-C173: +2 -- B2 closes (chapter 271) // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C168: +1 -- Kannada's ledger closes at 24 of 24 // HL-C166: +11 -- Sanskrit chapters 19 and 20 // HL-C157: same table same chapter, // +6: shorter chapters are more often drivable end to end // +10: vocabulary wave 5's new chapters // +3: HL-C88 slices 5-6 // +1: HL-C88 slice 7 (ES-C09-ncia) // +3: HL-C88 slice 8 (-ario, review, synthesis) // +5: vocabulary wave 6 // +1: HL-C88 slice 9 (falsos amigos) // +3: B1 si-condition rung // +3: HL-C113 preterite plural // +4: HL-C113 preterite close (strong plurals, review, synthesis) // +2: HL-C113 imperfect subjunctive // +3: HL-C113 unreal condition // HL-C113 step 7: +2 // HL-C113 step 8: +3 // HL-C128 step 2: +4 // HL-C128 step 3: +3 // HL-C128 step 4: +6 // HL-C128 step 5: +4 // HL-C127: +3 // HL-C128 step 7: +3 // HL-C128 step 8: +6 // HL-C128 step 9: +5 // HL-C128 step 10: +5 // HL-C136 wave I: +6 -- all six new chapters end to end, which is what a wave with no script section looks like // HL-C137 wave II: +36 adjective lessons, +6 chapters, and drivablePercent rises AGAIN // HL-C152: Spanish realizes SPINE-NEGATE-AND-ASK — five lessons, one chapter, A2 complete at 5/5 // HL-C156: 85 script segments across the six Indic tracks // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C163: +6 -- Sanskrit chapter 16 // HL-C165: +11 -- Sanskrit chapters 17 and 18 // kannada pre-A1 tranche: +35 lessons, +7 chapters (chapters 46-52) // malayalam pre-A1 tranche: 35 lessons over 7 chapters -- voice +34 and sight +1 sum to 35; the prefix figures follow from the per-chapter walk // hindi pre-A1 tranche: +35 lessons, +7 chapters (chapters 45-51) // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // sanskrit pre-A1 round 2: +35 lessons, +7 chapters (chapters 31-37) // telugu pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // kannada pre-A1 round 2: +35 lessons, +7 chapters (chapters 53-59) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295)
      // Tamil chapter 2, Spanish chapter 13, and Spanish chapter 18 can now start by ear.
      // -1: Tamil chapter 3 alone. It was unstartable because its first lesson needed
      // eyes; it now starts by ear. No other chapter moves.
      // -2 more: Tamil chapters 4 and 5 now start by ear as well.
      unstartableChapters: 184, // HL-C200: +35 telugu pre-A1 lessons, +7 chapters (chapters 46-52) // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // +2: two of vocabulary wave 5's new chapters need eyes from their own first lesson // +1: HL-C88 slices 5-6 // +5: vocabulary wave 6 // HL-C113 step 7: +2 -- chapters 214 and 215 are sight-only // HL-C128 step 2: +1 -- ch223 // HL-C128 step 3: +1 // HL-C128 step 5: +1 // HL-C127: +2 // chinese script chapter: +7 lessons, +1 chapter -- the seven components 你好 is built from, one glyph each // japanese hiragana tranche: +10 lessons, +1 chapter -- eight signs, one per lesson, plus two assemblies that introduce none
      overriddenLessons: 0,
      lessonsWithoutChapter: 0,
    }); // hindi pre-A1 round 2: +35 lessons, +7 chapters (chapters 52-58)
  });

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
  });

  it("carries no unexplained overrides", () => {
    const { lessons } = loadEverything();
    expect(buildModalityManifest(lessons).findings).toEqual([]);
  });
});
