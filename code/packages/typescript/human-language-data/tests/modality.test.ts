import { describe, expect, it } from "vitest";
import { parseLesson } from "../src/parse.js";
import { loadEverything } from "../src/loader.js";
import { buildCurriculumGapReport, renderCurriculumGapReport } from "../src/report.js";
import {
  DEFAULT_LINEARISABLE_TABLE_COLUMNS,
  MODALITIES,
  MODALITY_SIGNS,
  SIGHT_CUES,
  deriveLessonModality,
  drivablePrefix,
  lessonModalities,
  lessonText,
  matchedSightCues,
  modalityFindings,
  modalityRank,
  orderChapterLessons,
  requiredChannels,
  summarizeModality,
  tableRowColumns,
  unionModalities,
  widestTableColumns,
} from "../src/modality.js";

/** Build a lesson from parts, so each test names only what it is actually testing. */
function lesson(options: {
  id: string;
  language?: string;
  chapter?: number;
  sequence?: number;
  type?: string;
  modality?: string;
  modalityReason?: string;
  skills?: string;
  body?: string;
}): ReturnType<typeof parseLesson> {
  const frontmatter = [
    "schema_version: 2",
    `id: ${options.id}`,
    `chapter: ${options.chapter ?? 1}`,
    `type: ${options.type ?? "word"}`,
    "headword: hola",
    "gloss: hello",
    "concept_tag: GREETING-HELLO",
    `skills: [${options.skills ?? "listening, speaking, reading"}]`,
  ];
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

describe("the derivation itself", () => {
  it("rule 1: a writing lesson needs a pen", () => {
    const entry = deriveLessonModality(lesson({ id: "ES-W01", type: "writing" }));
    expect(entry.derived).toBe("pen");
    expect(entry.reasons).toContain("writing-type");
  });

  it("rule 1 wins over the body: a writing lesson stays pen even with a plain body", () => {
    const entry = deriveLessonModality(
      lesson({ id: "ES-W02", type: "writing", body: "## Warm-up\n\nJust talk." }),
    );
    expect(entry.derived).toBe("pen");
  });

  it("rule 2a: a script block needs eyes", () => {
    const entry = deriveLessonModality(
      lesson({ id: "HI-C01", body: "## Script — the letter क\n\nIt has a vertical bar." }),
    );
    expect(entry.derived).toBe("sight");
    expect(entry.reasons).toContain("script-block");
  });

  it("rule 2b: a sight cue in the prose needs eyes", () => {
    const entry = deriveLessonModality(
      lesson({ id: "ES-C02", body: "## Warm-up\n\nNow look at the two forms." }),
    );
    expect(entry.derived).toBe("sight");
    expect(entry.reasons).toContain("sight-cue");
    expect(entry.sightCues).toEqual(["look at"]);
  });

  it("rule 2c: a table wider than the configured width needs eyes", () => {
    const table =
      "## Warm-up\n\n| yo | tú | él | ella |\n| --- | --- | --- | --- |\n| soy | eres | es | es |";
    const entry = deriveLessonModality(lesson({ id: "ES-C03", body: table }));
    expect(entry.derived).toBe("sight");
    expect(entry.reasons).toContain("wide-table");
    expect(entry.widestTableColumns).toBe(4);
  });

  it("rule 2c is configurable: a narrow table is drivable once it can be linearised", () => {
    const table = "## Warm-up\n\n| día | day |\n| --- | --- |\n| noche | night |";
    const lessonWithTable = lesson({ id: "ES-C04", body: table });
    // The shipped default (3) reads this aloud; tightening the knob back to the
    // pre-lineariser 0 puts it back behind the learner's eyes.
    expect(deriveLessonModality(lessonWithTable).derived).toBe("voice");
    expect(
      deriveLessonModality(lessonWithTable, { maxLinearisableTableColumns: 0 }).derived,
    ).toBe("sight");
    // A wider paradigm is still sight at the shipped setting — the width is a limit,
    // not an amnesty for every table.
    const paradigm = lesson({
      id: "ES-C05",
      body: "## Warm-up\n\n| a | b | c | d |\n| - | - | - | - |\n| 1 | 2 | 3 | 4 |",
    });
    expect(deriveLessonModality(paradigm).derived).toBe("sight");
  });

  it("rule 2c is about speakability, not only width: a ragged narrow table needs eyes", () => {
    // Two columns, inside the limit, and still unreadable aloud: the second row has a
    // cell the header has no name for, so nothing can label it in speech.
    const ragged = "## Warm-up\n\n| día | day |\n| --- | --- |\n| noche | night | extra |";
    const entry = deriveLessonModality(lesson({ id: "ES-C4b", body: ragged }));
    expect(entry.derived).toBe("sight");
    expect(entry.reasons).toContain("wide-table");
  });

  it("rule 3: everything else plays in the car", () => {
    const entry = deriveLessonModality(lesson({ id: "ES-C06" }));
    expect(entry.derived).toBe("voice");
    expect(entry.reasons).toEqual(["no-visual-dependency"]);
    expect(entry.widestTableColumns).toBe(0);
    expect(entry.sightCues).toEqual([]);
  });

  it("records every rule that fired, not just the first", () => {
    const entry = deriveLessonModality(
      lesson({
        id: "HI-C02",
        body: "## Script — क\n\nSee the chart.\n\n| a | b | c |\n| - | - | - |",
      }),
    );
    expect(entry.reasons).toEqual(["script-block", "sight-cue", "wide-table"]);
  });

  it("does NOT derive modality from `skills` — declaring `reading` keeps a lesson drivable", () => {
    // The whole point. 501 of 531 schema-v2 lessons declare `[listening, speaking,
    // reading]`, yet *hola* is learnable by ear. Deriving from `skills` would have
    // mislabelled almost the entire corpus as sight.
    const readingSkill = deriveLessonModality(
      lesson({ id: "ES-C07", skills: "listening, speaking, reading" }),
    );
    expect(readingSkill.derived).toBe("voice");
    const writingSkill = deriveLessonModality(
      lesson({ id: "ES-C08", skills: "listening, speaking, reading, writing" }),
    );
    expect(writingSkill.derived).toBe("voice");
  });
});

describe("monotonicity", () => {
  it("orders the channels weakest to strongest", () => {
    expect(MODALITIES).toEqual(["voice", "sight", "pen"]);
    expect(modalityRank("voice")).toBe(0);
    expect(modalityRank("sight")).toBe(1);
    expect(modalityRank("pen")).toBe(2);
  });

  it("pen implies sight, and nothing implies voice", () => {
    expect(requiredChannels("pen")).toEqual(["sight", "pen"]);
    expect(requiredChannels("sight")).toEqual(["sight"]);
    expect(requiredChannels("voice")).toEqual(["voice"]);
  });

  it("carries the implication into a derived lesson's requirements", () => {
    expect(deriveLessonModality(lesson({ id: "ES-W03", type: "writing" })).requires).toEqual([
      "sight",
      "pen",
    ]);
  });

  it("unions a chapter's modalities weakest-first, with pen dragging sight in", () => {
    expect(unionModalities(["voice", "pen"])).toEqual(["voice", "sight", "pen"]);
    expect(unionModalities(["voice", "voice"])).toEqual(["voice"]);
    expect(unionModalities([])).toEqual([]);
  });

  it("publishes a sign for every channel", () => {
    expect(Object.keys(MODALITY_SIGNS).sort()).toEqual(["pen", "sight", "voice"]);
    expect(MODALITY_SIGNS.voice).toBe("🚗");
  });
});

describe("authored overrides", () => {
  it("accepts an override that agrees with the derivation without a reason", () => {
    const entry = deriveLessonModality(lesson({ id: "ES-C10", modality: "voice" }));
    expect(entry.modality).toBe("voice");
    expect(entry.overridden).toBe(false);
    expect(modalityFindings(entry)).toEqual([]);
  });

  it("accepts a contradicting override when the author explains it", () => {
    const entry = deriveLessonModality(
      lesson({
        id: "ES-C11",
        body: "## Warm-up\n\n| día | day |\n| - | - |",
        modality: "voice",
        modalityReason: "the table is a decorative recap of words already taught aloud",
      }),
    );
    expect(entry.derived).toBe("sight");
    expect(entry.modality).toBe("voice");
    expect(entry.overridden).toBe(true);
    expect(modalityFindings(entry)).toEqual([]);
  });

  it("reports — never throws on — a contradicting override with no reason", () => {
    const entry = deriveLessonModality(
      lesson({ id: "ES-C12", body: "## Warm-up\n\n| día | day |\n| - | - |", modality: "voice" }),
    );
    const findings = modalityFindings(entry);
    expect(findings).toHaveLength(1);
    expect(findings[0]?.code).toBe("modality-unexplained-override");
    expect(findings[0]?.message).toContain("wide-table");
  });

  it("reports an unknown value and falls back to the derivation", () => {
    const entry = deriveLessonModality(lesson({ id: "ES-C13", modality: "eyes" }));
    expect(entry.authored).toBe("eyes");
    expect(entry.modality).toBe("voice");
    const findings = modalityFindings(entry);
    expect(findings).toHaveLength(1);
    expect(findings[0]?.code).toBe("modality-unknown-value");
  });

  it("reports the unknown value once, without also reporting an unexplained override", () => {
    const entry = deriveLessonModality(
      lesson({ id: "ES-W04", type: "writing", modality: "shouting" }),
    );
    expect(entry.modality).toBe("pen");
    expect(modalityFindings(entry).map((finding) => finding.code)).toEqual([
      "modality-unknown-value",
    ]);
  });

  it("collects findings across the whole corpus instead of stopping at the first", () => {
    const summary = summarizeModality([
      lesson({ id: "ES-C14", modality: "nonsense" }),
      lesson({ id: "ES-C15", modality: "sight" }),
      lesson({ id: "ES-C16" }),
    ]);
    expect(summary.findings).toHaveLength(2);
    expect(summary.findings.map((finding) => finding.lessonId)).toEqual(["ES-C14", "ES-C15"]);
  });
});

describe("the drivable prefix", () => {
  const ordered = (types: Array<{ id: string; sequence: number; sight?: boolean }>) =>
    lessonModalities(
      types.map((entry) =>
        lesson({
          id: entry.id,
          sequence: entry.sequence,
          body: entry.sight ? "## Warm-up\n\nLook at the shape." : "## Warm-up\n\nSay it.",
        }),
      ),
    );

  it("counts voice lessons from the front, in authored sequence order", () => {
    const entries = ordered([
      { id: "ES-C01-c", sequence: 30, sight: true },
      { id: "ES-C01-a", sequence: 10 },
      { id: "ES-C01-b", sequence: 20 },
      { id: "ES-C01-d", sequence: 40 },
    ]);
    expect(orderChapterLessons(entries).map((entry) => entry.lessonId)).toEqual([
      "ES-C01-a",
      "ES-C01-b",
      "ES-C01-c",
      "ES-C01-d",
    ]);
    // Two voice lessons, then the sight one stops the prefix — the trailing voice
    // lesson is NOT reachable in the car, because the chapter is prerequisite-ordered.
    expect(drivablePrefix(entries)).toBe(2);
  });

  it("is 0 when the chapter opens with a lesson that needs eyes", () => {
    const entries = ordered([
      { id: "ES-C02-a", sequence: 10, sight: true },
      { id: "ES-C02-b", sequence: 20 },
      { id: "ES-C02-c", sequence: 30 },
    ]);
    expect(drivablePrefix(entries)).toBe(0);
  });

  it("is the whole chapter when nothing needs eyes", () => {
    const entries = ordered([
      { id: "ES-C03-a", sequence: 10 },
      { id: "ES-C03-b", sequence: 20 },
    ]);
    expect(drivablePrefix(entries)).toBe(2);
  });

  it("falls back to lesson id when a legacy lesson carries no sequence", () => {
    const entries = lessonModalities([
      lesson({ id: "ES-C04-b" }),
      lesson({ id: "ES-C04-a" }),
    ]);
    expect(entries.every((entry) => entry.sequence === null)).toBe(true);
    expect(orderChapterLessons(entries).map((entry) => entry.lessonId)).toEqual([
      "ES-C04-a",
      "ES-C04-b",
    ]);
  });

  it("sorts sequenced lessons ahead of unsequenced ones", () => {
    const entries = lessonModalities([
      lesson({ id: "ES-C05-legacy" }),
      lesson({ id: "ES-C05-modern", sequence: 90 }),
    ]);
    expect(orderChapterLessons(entries).map((entry) => entry.lessonId)).toEqual([
      "ES-C05-modern",
      "ES-C05-legacy",
    ]);
  });
});

describe("chapter and track rollups", () => {
  it("reports each chapter's prefix, counts, modality union, and first blocker", () => {
    const summary = summarizeModality([
      lesson({ id: "ES-C01-a", chapter: 1, sequence: 10 }),
      lesson({ id: "ES-C01-b", chapter: 1, sequence: 20 }),
      lesson({ id: "ES-C01-c", chapter: 1, sequence: 30, type: "writing" }),
      lesson({ id: "ES-C02-a", chapter: 2, sequence: 40, body: "## Script — ñ\n\nA tilde." }),
    ]);
    const [track] = summary.tracks;
    expect(track?.language).toBe("spanish");
    expect(track?.lessonCount).toBe(4);
    expect(track?.voice).toBe(2);
    expect(track?.sight).toBe(1);
    expect(track?.pen).toBe(1);
    expect(track?.drivablePercent).toBe(50);
    expect(track?.drivablePrefixTotal).toBe(2);

    const [first, second] = track?.chapters ?? [];
    expect(first).toMatchObject({
      chapter: 1,
      lessonCount: 3,
      drivablePrefix: 2,
      firstNonVoiceLesson: "ES-C01-c",
      modalities: ["voice", "sight", "pen"],
    });
    expect(second).toMatchObject({ chapter: 2, drivablePrefix: 0, firstNonVoiceLesson: "ES-C02-a" });
  });

  it("leaves firstNonVoiceLesson null when a whole chapter is drivable", () => {
    const summary = summarizeModality([lesson({ id: "ES-C01-a", chapter: 1, sequence: 10 })]);
    expect(summary.tracks[0]?.chapters[0]).toMatchObject({
      drivablePrefix: 1,
      firstNonVoiceLesson: null,
    });
  });

  it("counts an unparseable chapter in the track but in no chapter", () => {
    const broken = parseLesson(
      "---\nid: ES-CXX\ntype: word\nheadword: hola\ngloss: hi\n---\n\nSay hola.\n",
      "spanish",
    );
    const summary = summarizeModality([broken]);
    expect(summary.tracks[0]?.lessonCount).toBe(1);
    expect(summary.tracks[0]?.chapters).toEqual([]);
  });

  it("sorts tracks by language and reports zero percent for an empty corpus", () => {
    const summary = summarizeModality([
      lesson({ id: "TE-C01", language: "telugu" }),
      lesson({ id: "ES-C01" }),
    ]);
    expect(summary.tracks.map((track) => track.language)).toEqual(["spanish", "telugu"]);
    expect(summarizeModality([])).toMatchObject({
      totalLessons: 0,
      drivablePercent: 0,
      tracks: [],
      findings: [],
    });
  });
});

describe("text scanning helpers", () => {
  it("reads block markdown, not a nonexistent `content` field", () => {
    // Reading the wrong field yields `undefined` for every block, finds no tables
    // anywhere, and silently reports a 100% drivable corpus. Pin the right field.
    const parsed = lesson({ id: "ES-C20", body: "## Warm-up\n\n| a | b | c |\n| - | - | - |" });
    expect(parsed.blocks[0]).toHaveProperty("markdown");
    expect(parsed.blocks[0]).not.toHaveProperty("content");
    expect(lessonText(parsed)).toContain("| a | b | c |");
    expect(widestTableColumns(lessonText(parsed))).toBe(3);
  });

  it("includes the preamble, so a table above the first heading still counts", () => {
    const parsed = parseLesson(
      "---\nid: ES-C21\nchapter: 1\ntype: word\n---\n\n# title\n\n| a | b | c | d |\n\n## Warm-up\n\nSay it.\n",
      "spanish",
    );
    expect(deriveLessonModality(parsed).derived).toBe("sight");
  });

  it("counts table columns with and without an outer fence, and through escapes", () => {
    expect(tableRowColumns("| a | b |")).toBe(2);
    expect(tableRowColumns("a | b")).toBe(2);
    expect(tableRowColumns("| --- | --- | --- |")).toBe(3);
    expect(tableRowColumns("| a |  | c |")).toBe(3);
    expect(tableRowColumns("|")).toBe(0);
    expect(tableRowColumns("no pipes here")).toBe(1);
    // `\|` is a literal pipe inside a cell, not a column fence.
    expect(tableRowColumns("| a \\| b | c |")).toBe(2);
  });

  it("takes the widest row, because a table is as readable as its worst line", () => {
    expect(widestTableColumns("| a | b |\n| - | - |\n| a | b | c | d |")).toBe(4);
    expect(widestTableColumns("no table at all")).toBe(0);
    expect(widestTableColumns("   | a | b |")).toBe(2);
    // Four or more leading spaces is an indented code block, not a table row.
    expect(widestTableColumns("    | a | b |")).toBe(0);
  });

  it("matches sight cues case-insensitively, in list order", () => {
    expect(matchedSightCues("LOOK AT the chart")).toEqual(["look at", "the chart"]);
    expect(matchedSightCues("say it aloud")).toEqual([]);
    for (const cue of SIGHT_CUES) expect(matchedSightCues(`x ${cue} y`)).toContain(cue);
  });
});

describe("the gap report", () => {
  const corpus = [
    lesson({ id: "ES-C01-a", chapter: 1, sequence: 10 }),
    lesson({ id: "ES-C01-b", chapter: 1, sequence: 20, type: "writing" }),
  ];
  const registry = {
    version: 1,
    languages: [
      {
        id: "spanish",
        name: "Spanish",
        family: "Romance",
        script: "latin",
        status: "active",
        bridges: [],
      },
    ],
  };

  it("publishes the modality section in the JSON view", () => {
    const report = buildCurriculumGapReport({
      registry,
      lessons: corpus,
      books: { books: [] },
    });
    expect(report.modality.totalLessons).toBe(2);
    expect(report.modality.drivablePercent).toBe(50);
    expect(report.modality.maxLinearisableTableColumns).toBe(
      DEFAULT_LINEARISABLE_TABLE_COLUMNS,
    );
    expect(report.summary.drivableLessons).toBe(1);
    expect(report.summary.drivablePercent).toBe(50);
    expect(report.summary.chaptersWithoutDrivablePrefix).toBe(0);
    expect(report.summary.unexplainedModalityOverrides).toBe(0);
  });

  it("threads the linearisable width through to the derivation", () => {
    const tabled = [
      lesson({
        id: "ES-C02",
        chapter: 2,
        body: "## Warm-up\n\n| a | b |\n| - | - |\n| uno | one |",
      }),
    ];
    const relaxed = buildCurriculumGapReport({ registry, lessons: tabled, books: { books: [] } });
    expect(relaxed.modality.voice).toBe(1);
    const strict = buildCurriculumGapReport({
      registry,
      lessons: tabled,
      books: { books: [] },
      modality: { maxLinearisableTableColumns: 0 },
    });
    expect(strict.modality.voice).toBe(0);
  });

  it("counts blocked chapters and unexplained overrides in the summary", () => {
    const report = buildCurriculumGapReport({
      registry,
      lessons: [
        lesson({ id: "ES-C03", chapter: 3, sequence: 10, body: "## Script — ñ\n\nA tilde." }),
        lesson({ id: "ES-C04", chapter: 4, sequence: 20, type: "writing", modality: "voice" }),
      ],
      books: { books: [] },
    });
    // Chapter 3 is blocked by its script block. Chapter 4 is NOT: the override is
    // unexplained and reported, but this PR gates nothing, so the authored `voice`
    // still takes effect and the chapter stays drivable.
    expect(report.summary.chaptersWithoutDrivablePrefix).toBe(1);
    expect(report.summary.unexplainedModalityOverrides).toBe(1);
  });

  it("renders a human-readable modality section", () => {
    const text = renderCurriculumGapReport(
      buildCurriculumGapReport({ registry, lessons: corpus, books: { books: [] } }),
    );
    expect(text).toContain("Modality (HL08)");
    expect(text).toContain("never from `skills`");
    expect(text).toContain("1 voice, 0 sight, 1 pen of 2 lessons; 50% drivable");
    expect(text).toContain("spanish: 1 voice, 0 sight, 1 pen");
    expect(text).toContain("Chapters that cannot be started by ear (drivable prefix 0): 0");
    expect(text).toContain("Modality findings (report-only): 0");
    expect(text).toContain("1 drivable lessons (50% of the corpus)");
  });

  it("names blocked chapters and findings in the rendered text", () => {
    const text = renderCurriculumGapReport(
      buildCurriculumGapReport({
        registry,
        lessons: [
          lesson({ id: "ES-C05", chapter: 5, sequence: 10, body: "## Script — ñ\n\nA tilde." }),
          lesson({ id: "ES-C06", chapter: 6, sequence: 20, type: "writing", modality: "voice" }),
        ],
        books: { books: [] },
      }),
    );
    expect(text).toContain("spanish ch5: 0 of 1 (first blocker ES-C05)");
    expect(text).toContain("modality-unexplained-override: ES-C06");
  });
});

describe("corpus regression", () => {
  // A pinned measurement, not a taste test. The parser, the table detector, and the
  // cue list all feed this number, so a silent change in any of them — most
  // dangerously, a block field rename that makes every lesson look clean — moves it
  // and fails here instead of shipping a curriculum falsely advertised as drivable.
  //
  // Reproduces the HL08 baseline: 51 `pen`, 7 script-block lessons, and 322
  // table-bearing lessons among the remaining 1,038. HL08 records 695 drivable from
  // 56 cue-bearing lessons; this implementation's published cue list matches 61.
  //
  // At the pre-lineariser width of 0 — every table means eyes — that lands on 694,
  // pinned below. The shipped default is now 3, because HL-C16 built the lineariser
  // HL08's migration step 3 promised, and the same corpus then measures 925 drivable.
  // BOTH numbers are pinned on purpose: the second is the claim the product makes,
  // and the first is the control that proves the jump came from the lineariser and
  // not from a detector that quietly stopped detecting.
  it("pins the corpus-wide drivable count at the shipped table width", () => {
    const { lessons } = loadEverything();
    const summary = summarizeModality(lessons);
    expect(summary.maxLinearisableTableColumns).toBe(3);
    expect(summary.totalLessons).toBe(1096);
    expect(summary.pen).toBe(51);
    expect(summary.voice).toBe(925);
    expect(summary.sight).toBe(120);
    expect(summary.voice + summary.sight + summary.pen).toBe(summary.totalLessons);
    expect(summary.drivablePercent).toBe(84);
  });

  // The 120 that still need eyes, by cause. `wide-table` is the burn-down list HL08's
  // migration step 4 names: 65 lessons still carry at least one table of four columns
  // or more, and 52 of those have no other reason to need eyes, so reshaping just
  // those tables would move 52 more lessons into the car.
  it("pins why the remaining sight lessons still need eyes", () => {
    const { lessons } = loadEverything();
    const sight = lessonModalities(lessons).filter((entry) => entry.modality === "sight");
    expect(sight).toHaveLength(120);
    const withReason = (reason: string): number =>
      sight.filter((entry) => (entry.reasons as string[]).includes(reason)).length;
    expect(withReason("script-block")).toBe(7);
    expect(withReason("sight-cue")).toBe(61);
    expect(withReason("wide-table")).toBe(65);
    expect(
      sight.filter((entry) => entry.reasons.length === 1 && entry.reasons[0] === "wide-table"),
    ).toHaveLength(52);
  });

  it("pins the pre-lineariser baseline, so the gain is attributable", () => {
    const { lessons } = loadEverything();
    const summary = summarizeModality(lessons, { maxLinearisableTableColumns: 0 });
    expect(summary.pen).toBe(51);
    expect(summary.voice).toBe(694);
    expect(summary.sight).toBe(351);
    expect(summary.drivablePercent).toBe(63);
  });

  it("pins the structural counts the derivation is built on", () => {
    const { lessons } = loadEverything();
    expect(lessons.filter((entry) => entry.realization.type === "writing")).toHaveLength(51);
    const nonWriting = lessons.filter((entry) => entry.realization.type !== "writing");
    expect(
      nonWriting.filter((entry) => entry.blocks.some((block) => block.type === "script")),
    ).toHaveLength(7);
    const remaining = nonWriting.filter(
      (entry) => !entry.blocks.some((block) => block.type === "script"),
    );
    expect(remaining).toHaveLength(1038);
    expect(
      remaining.filter((entry) => widestTableColumns(lessonText(entry)) > 0),
    ).toHaveLength(322);
  });

  it("keeps the corpus free of unexplained overrides", () => {
    const { lessons } = loadEverything();
    expect(summarizeModality(lessons).findings).toEqual([]);
  });

  it("gives every lesson a modality and every chapter a prefix within its length", () => {
    const { lessons } = loadEverything();
    const summary = summarizeModality(lessons);
    for (const track of summary.tracks) {
      for (const chapter of track.chapters) {
        expect(chapter.drivablePrefix).toBeGreaterThanOrEqual(0);
        expect(chapter.drivablePrefix).toBeLessThanOrEqual(chapter.lessonCount);
        expect(chapter.voice + chapter.sight + chapter.pen).toBe(chapter.lessonCount);
      }
    }
  });
});
