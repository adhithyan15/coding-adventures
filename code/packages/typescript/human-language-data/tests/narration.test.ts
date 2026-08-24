// Tests for the HL08 narration export — the script a voice assistant reads aloud.
//
// The property under test throughout is not "does it produce text" but **does it
// produce text that would actually teach someone who cannot look at anything**. Two
// failure modes get the most attention, because they are the ones that would hurt a
// real learner:
//
//   1. Content that silently disappears — a table skipped, a cue swallowed, a
//      blockquote chopped at its line breaks.
//   2. A lesson advertised as drivable that is not.

import { describe, expect, it } from "vitest";
import { loadEverything } from "../src/loader.js";
import { deriveLessonModality } from "../src/modality.js";
import {
  narrateChapter,
  narrateLesson,
  narrationChapters,
  pairRomanization,
  parseNarrationCue,
  renderChapterNarrationText,
  renderLessonNarrationText,
  splitNarrationCues,
  type NarrationSegment,
} from "../src/narration.js";
import { parseLesson } from "../src/parse.js";
import {
  endSentence,
  findMarkdownTables,
  isDelimiterCell,
  linariseTable,
  linariseTables,
  speakableInline,
  splitTableRow,
} from "../src/speech.js";

/** Build a lesson from parts, so each test names only what it is testing. */
function lesson(options: {
  id?: string;
  language?: string;
  script?: string;
  chapter?: number;
  sequence?: number;
  type?: string;
  headword?: string;
  romanization?: string;
  gloss?: string;
  title?: string;
  body: string;
}): ReturnType<typeof parseLesson> {
  const frontmatter = [
    "schema_version: 2",
    `id: ${options.id ?? "ES-C01-hola"}`,
    `chapter: ${options.chapter ?? 1}`,
    `type: ${options.type ?? "word"}`,
    `headword: ${options.headword ?? "hola"}`,
    `gloss: ${options.gloss ?? "hello"}`,
    "concept_tag: GREETING-HELLO",
  ];
  if (options.sequence !== undefined) frontmatter.push(`sequence: ${options.sequence}`);
  if (options.romanization !== undefined) {
    frontmatter.push(`romanization: ${options.romanization}`);
  }
  const title = options.title ?? `${options.headword ?? "hola"} — ${options.gloss ?? "hello"}`;
  return parseLesson(
    `---\n${frontmatter.join("\n")}\n---\n\n# ${title}\n\n${options.body}\n`,
    options.language ?? "spanish",
    (options.script ?? "latin") as never,
  );
}

/** Every segment of a narrated lesson, flattened, so a test can ask "is it in there". */
function segments(narration: ReturnType<typeof narrateLesson>): NarrationSegment[] {
  return narration.blocks.flatMap((block) => block.segments);
}

// ---------------------------------------------------------------------------

describe("speakable inline Markdown", () => {
  it("removes typography a voice would otherwise pronounce", () => {
    expect(speakableInline("**hola** is said *OH-la*")).toBe("hola is said OH-la");
    expect(speakableInline("`silent-h` — the h")).toBe("silent-h — the h");
    expect(speakableInline("~~struck~~ out")).toBe("struck out");
  });

  it("keeps a link's words and drops its destination", () => {
    expect(speakableInline("see the [pronunciation guide](../guide.md) now")).toBe(
      "see the pronunciation guide now",
    );
    // A URL read aloud — "h t t p s colon slash slash" — is worse than useless.
    expect(speakableInline("[Persian Online](https://example.test/a?b=c)")).toBe(
      "Persian Online",
    );
  });

  it("drops the reconstruction asterisk, escaped or not", () => {
    // `\*pēr` is a mark for the eye. Left in, a speech engine says "asterisk pēr".
    expect(speakableInline("Dravidian *\\*pēr*")).toBe("Dravidian pēr");
  });

  it("reads the arrow three different ways, decided by what follows it", () => {
    expect(speakableInline("aqua → ewe")).toBe("aqua becomes ewe");
    expect(speakableInline("→ [pronunciation reference](../ref.md)")).toBe(
      "see pronunciation reference",
    );
    expect(speakableInline("= a-vu-nu →")).toBe("= a-vu-nu, which gives:");
  });

  it("says the etymology arrow and the syllable dot", () => {
    expect(speakableInline("pēru ← Dravidian pēr")).toBe("pēru from Dravidian pēr");
    expect(speakableInline("na · ma · s")).toBe("na, ma, s");
  });

  it("does not glue a full stop onto a sentence that already ends inside brackets", () => {
    // `.).` is a stumble. See `endSentence`.
    expect(endSentence("Is it related? (No — a coincidence.)")).toBe(
      "Is it related? (No — a coincidence.)",
    );
    expect(endSentence("Two pure vowels")).toBe("Two pure vowels.");
    expect(endSentence("   ")).toBe("");
  });
});

describe("reading a Markdown table's shape", () => {
  it("splits cells, honouring the fence and escaped pipes", () => {
    expect(splitTableRow("| word | gloss |")).toEqual(["word", "gloss"]);
    expect(splitTableRow("| a \\| b | c |")).toEqual(["a | b", "c"]);
    expect(splitTableRow("a | b")).toEqual(["a", "b"]);
  });

  it("recognises every GFM delimiter cell spelling", () => {
    for (const cell of ["---", ":--", "--:", ":-:", " - "]) {
      expect(isDelimiterCell(cell)).toBe(true);
    }
    for (const cell of ["", "abc", "-a-", ":"]) {
      expect(isDelimiterCell(cell)).toBe(false);
    }
  });

  it("finds each run of rows and says whether it is a real table", () => {
    const tables = findMarkdownTables("prose\n| a | b |\n|---|---|\n| 1 | 2 |\n\nmore\n| x | y |");
    expect(tables).toHaveLength(2);
    expect(tables[0]?.delimited).toBe(true);
    expect(tables[1]?.delimited).toBe(false);
  });
});

describe("table linearisation", () => {
  const table = (markdown: string) => findMarkdownTables(markdown)[0]!;

  it("reads a two-column word→gloss table as HL08's own 'X means Y'", () => {
    const result = linariseTable(
      table("| Telugu | English |\n|---|---|\n| నా పేరు మీరా | My name is Mira |"),
    );
    expect(result.ok).toBe(true);
    expect(result.ok && result.utterances).toEqual(["నా పేరు మీరా means My name is Mira."]);
  });

  it("reads a three-column table as labelled facts", () => {
    const result = linariseTable(
      table('| Language | "Hello" | Source |\n|---|---|---|\n| Telugu | namaskāram | Sanskrit |'),
    );
    expect(result.ok && result.utterances).toEqual([
      'Language: Telugu. "Hello": namaskāram. Source: Sanskrit.',
    ]);
  });

  it("speaks an unlabelled column as a bare value rather than refusing the table", () => {
    // `| Read | | Meaning |` is the corpus's commonest practice-table shape; the blank
    // middle heading is the romanization, which a sighted reader has no label for
    // either.
    const result = linariseTable(
      table("| Read | | Meaning |\n|---|---|---|\n| سلام | salām | peace |"),
    );
    expect(result.ok && result.utterances).toEqual(["Read: سلام, salām. Meaning: peace."]);
  });

  it("says a blank cell as 'blank', so a gap in the table is not a gap in the narration", () => {
    const result = linariseTable(table("| a | b |\n|---|---|\n| one | |"));
    expect(result.ok && result.utterances).toEqual(["a: one. b: blank."]);
  });

  it("reads a pipe run with no delimiter row as an unlabelled sequence", () => {
    // Markdown renders this literally, pipes and all. There is still nothing wrong
    // with saying it, and refusing would send the lesson to `sight` over a missing
    // `|---|`.
    const result = linariseTable(table("| j'habite · tu habites | (all a-BEET) |"));
    expect(result.ok && result.utterances).toEqual(["j'habite, tu habites, (all a-BEET)."]);
  });

  it("refuses a table beyond the supported width, and says how big it was", () => {
    const wide = table(
      "| | numeral | word | said |\n|---|---|---|---|\n| 1 | ౧ | ఒకటి | okaṭi |",
    );
    const result = linariseTable(wide);
    expect(result.ok).toBe(false);
    expect(!result.ok && result.reason).toBe("too-wide");
    expect(!result.ok && result.columns).toBe(4);
    expect(!result.ok && result.rowCount).toBe(1);
    // The same table is readable once the width is raised — the refusal is the
    // policy's, not the lineariser's inability.
    expect(linariseTable(wide, { maxColumns: 4 }).ok).toBe(true);
  });

  it("refuses a ragged table even when it is narrow enough", () => {
    const result = linariseTable(table("| a | b |\n|---|---|\n| one | two | three |"));
    expect(!result.ok && result.reason).toBe("ragged-row");
  });

  it("refuses a heading row with nothing under it", () => {
    const result = linariseTable(table("| a | b |\n|---|---|"));
    expect(!result.ok && result.reason).toBe("no-rows");
  });

  it("linearises every table in a text, in document order", () => {
    const results = linariseTables("| a | b |\n|---|---|\n| 1 | 2 |\n\n| p | q | r | s |");
    expect(results.map((result) => result.ok)).toEqual([true, false]);
  });
});

describe("cues become structured directives, not prose", () => {
  it("parses the three authored cue shapes", () => {
    expect(parseNarrationCue("PAUSE 2s")).toMatchObject({ kind: "pause", seconds: 2, perItem: false });
    expect(parseNarrationCue("PAUSE 1s each")).toMatchObject({ kind: "pause", perItem: true });
    expect(parseNarrationCue("REPEAT x2")).toMatchObject({ kind: "repeat", times: 2 });
    expect(parseNarrationCue("YOU SAY: hola")).toMatchObject({
      kind: "prompt",
      action: "SAY",
      instruction: "hola",
      spoken: true,
      scored: false,
    });
  });

  it("marks a hands-or-eyes cue as not spoken, so a driver is not asked to write", () => {
    expect(parseNarrationCue("YOU WRITE: alif")).toMatchObject({ action: "WRITE", spoken: false });
    expect(parseNarrationCue("YOU TRACE: the bowl")).toMatchObject({ spoken: false });
  });

  it("refuses to mistake an ordinary bracket for a cue", () => {
    // Deleting one of these would delete real teaching content from the script.
    expect(parseNarrationCue("bonjour")).toBeNull();
    expect(parseNarrationCue("PAUSE soon")).toBeNull();
    expect(parseNarrationCue("REPEAT often")).toBeNull();
    expect(parseNarrationCue("YOU there")).toBeNull();
    expect(parseNarrationCue("You say: hola")).toBeNull();
  });

  it("keeps a cue whole when it has brackets nested inside it", () => {
    const parts = splitNarrationCues('[YOU SAY: the pattern — "[nā] [pēru]"]');
    expect(parts).toHaveLength(1);
    expect(parts[0]).toHaveProperty("cue");
  });

  it("keeps a Markdown link intact when the bracket is not a cue", () => {
    const parts = splitNarrationCues("read the [guide](../g.md) first");
    expect(parts).toEqual([{ text: "read the [guide](../g.md) first" }]);
  });

  it("survives an unbalanced bracket without eating the rest of the paragraph", () => {
    const parts = splitNarrationCues("a [ b c");
    expect(parts).toEqual([{ text: "a [ b c" }]);
  });

  it("bounds the search for a closing bracket, so a wall of '[' cannot go quadratic", () => {
    // An unbalanced `[` makes the scan run to the end of the paragraph and then
    // advance one character. Unbounded, 40k opening brackets is 1.6 billion character
    // comparisons in a build step. Bounded, it is linear and the output is unchanged.
    // The budget is deliberately loose. Its job is to separate LINEAR from
    // QUADRATIC, not to police milliseconds. Unbounded, 40k opening brackets is
    // ~1.6 billion comparisons and takes minutes; bounded, it is linear —
    // measured locally at 130/271/561/947 ms for 10k/20k/40k/80k, i.e. flat per
    // character. A tight budget here only measures how contended the runner is:
    // at 2_000 ms this failed CI at 10,677 ms while the implementation was
    // correct. lessons.md records the general rule — "CI is ~25x slower than
    // local for compute-heavy tests" — so pick a threshold a quadratic scan
    // still cannot meet and a loaded runner comfortably can.
    const started = Date.now();
    const parts = splitNarrationCues("[".repeat(40_000));
    expect(parts).toEqual([{ text: "[".repeat(40_000) }]);
    expect(Date.now() - started).toBeLessThan(30_000);
    // Vitest's default testTimeout is 5000 ms and it kills the test before the
    // assertion above is ever reached — raising the expect() alone did nothing.
    // The explicit timeout is the half that actually matters on a loaded runner.
  }, 60_000);

  it("preserves the cues of a real lesson as directives in the narrated blocks", () => {
    const narration = narrateLesson(
      lesson({
        body:
          "## Warm-up\n\n[PAUSE 2s] The first word.\n\n" +
          "## Guided Practice\n\n[PAUSE 1s]\n- [YOU SAY: \"hola\" — OH-la]\n- [REPEAT x2]",
      }),
    );
    const kinds = segments(narration).map((segment) => segment.kind);
    expect(kinds).toEqual(["pause", "speech", "pause", "prompt", "repeat"]);
    const text = renderLessonNarrationText(narration);
    expect(text).toContain("[pause 2 seconds]");
    expect(text).toContain('[your turn — say: "hola" — OH-la]');
    expect(text).toContain("[repeat that twice]");
    // The cue is a rehearsal, never an answer key.
    expect(segments(narration).every((segment) => segment.kind !== "activity")).toBe(true);
  });
});

describe("prose shaping", () => {
  it("keeps a multi-line blockquote as one utterance", () => {
    // Split per line it came out as "…is building." / "on real family resemblances…" —
    // a paragraph chopped at the page's line breaks rather than its own clauses.
    const narration = narrateLesson(
      lesson({ body: "## Warm-up\n\n> Why flag this so hard? Because the whole\n> method is building\n> on real resemblances." }),
    );
    expect(segments(narration)).toEqual([
      {
        kind: "speech",
        text: "Why flag this so hard? Because the whole method is building on real resemblances.",
      },
    ]);
  });

  it("gives each list item its own utterance", () => {
    const narration = narrateLesson(lesson({ body: "## Warm-up\n\n- one thing\n- another thing" }));
    expect(segments(narration).map((segment) => "text" in segment && segment.text)).toEqual([
      "one thing.",
      "another thing.",
    ]);
  });

  it("finds a table nested inside a blockquote", () => {
    const narration = narrateLesson(
      lesson({ body: "## Warm-up\n\n> | English | German |\n> |---|---|\n> | good | gut |" }),
    );
    const [segment] = segments(narration);
    expect(segment?.kind).toBe("table");
    expect(segment?.kind === "table" && segment.utterances).toEqual(["English: good. German: gut."]);
  });
});

describe("romanization travels with the script", () => {
  it("follows target-script text with how to say it", () => {
    expect(pairRomanization("read خداحافظ aloud", [
      { headword: "خداحافظ", romanization: "khodâ hâfez" },
    ])).toBe("read خداحافظ (khodâ hâfez) aloud");
  });

  it("does not pair twice when the author already wrote the romanization", () => {
    const pairs = [{ headword: "خداحافظ", romanization: "khodâ hâfez" }];
    expect(pairRomanization("خداحافظ — khodâ hâfez — goodbye", pairs)).toBe(
      "خداحافظ — khodâ hâfez — goodbye",
    );
  });

  it("pairs whole words only, so a letter lesson cannot split a word", () => {
    // The Arabic track teaches ا (alif) as its own lesson. A substring replace turned
    // سلام into `سلا (alif)م` — the pronunciation guide spliced into the middle of the
    // word it was meant to help with.
    expect(pairRomanization("say سلام now", [{ headword: "ا", romanization: "alif" }])).toBe(
      "say سلام now",
    );
    expect(pairRomanization("the letter ا here", [{ headword: "ا", romanization: "alif" }])).toBe(
      "the letter ا (alif) here",
    );
  });

  it("uses the whole chapter's glossary, not only the lesson's own headword", () => {
    const chapter = narrateChapter(
      "persian",
      5,
      [
        lesson({
          id: "FA-C05-khoda",
          language: "persian",
          script: "perso-arabic",
          sequence: 10,
          headword: "خدا",
          romanization: "khodâ",
          gloss: "God",
          body: "## Warm-up\n\nA first word.",
        }),
        lesson({
          id: "FA-C05-hafez",
          language: "persian",
          script: "perso-arabic",
          sequence: 20,
          headword: "حافظ",
          romanization: "hâfez",
          gloss: "guardian",
          // Mentions the PREVIOUS lesson's headword, whose romanization lives in that
          // lesson's frontmatter and nowhere in this one.
          body: "## Warm-up\n\nRetrieve خدا before you start.",
        }),
      ],
    );
    const text = renderChapterNarrationText(chapter, "Persian");
    expect(text).toContain("Retrieve خدا (khodâ) before you start.");
  });

  it("leaves Latin-script lessons alone, where the romanization is the headword", () => {
    const narration = narrateLesson(lesson({ body: "## Warm-up\n\nSay hola out loud." }));
    expect(renderLessonNarrationText(narration)).toContain("Say hola out loud.");
    expect(renderLessonNarrationText(narration)).not.toContain("hola (hola)");
  });
});

describe("a table that cannot be spoken", () => {
  const wide =
    "## Grammar Lens\n\n| yo | tú | él | ella |\n|---|---|---|---|\n| soy | eres | es | es |";

  it("forces the lesson to sight rather than dropping the table", () => {
    const parsed = lesson({ body: wide });
    expect(deriveLessonModality(parsed).derived).toBe("sight");
    const narration = narrateLesson(parsed);
    expect(narration.modality).toBe("sight");
    const [segment] = segments(narration);
    expect(segment?.kind).toBe("table-skipped");
  });

  it("speaks what the learner is missing — size, columns, and why", () => {
    const narration = narrateLesson(lesson({ body: wide }));
    const text = renderLessonNarrationText(narration);
    expect(text).toContain("4 columns and 1 row");
    expect(text).toContain("Its columns are: yo, tú, él, ella.");
    expect(text).toContain("more columns than can be held in the ear");
    expect(text).toContain("Come back and look at it when you have stopped.");
  });

  it("names an unlabelled column instead of leaving a hole in the sentence", () => {
    const narration = narrateLesson(
      lesson({ body: "## Grammar Lens\n\n| | a | b | c |\n|---|---|---|---|\n| 1 | 2 | 3 | 4 |" }),
    );
    expect(renderLessonNarrationText(narration)).toContain(
      "Its columns are: one with no heading, a, b, c.",
    );
  });

  it("reports narration-block-unrenderable when an override calls it drivable anyway", () => {
    const parsed = parseLesson(
      "---\nschema_version: 2\nid: ES-C90\nchapter: 1\ntype: word\nheadword: hola\n" +
        "gloss: hello\nmodality: voice\nmodality_reason: the table is decorative\n---\n\n" +
        `# hola\n\n${wide}\n`,
      "spanish",
    );
    const narration = narrateLesson(parsed);
    expect(narration.modality).toBe("voice");
    expect(narration.findings.map((finding) => finding.code)).toEqual([
      "narration-block-unrenderable",
    ]);
  });

  it("becomes drivable once the width is raised to cover it", () => {
    const narration = narrateLesson(lesson({ body: wide }), {
      maxLinearisableTableColumns: 4,
    });
    expect(narration.modality).toBe("voice");
    expect(segments(narration)[0]?.kind).toBe("table");
  });
});

describe("the spoken notice on a sight or pen lesson", () => {
  it("names what the learner needs and what to leave until they stop", () => {
    const narration = narrateLesson(
      lesson({
        body:
          "## Warm-up\n\nSay it aloud.\n\n" +
          "## The ten, in three families\n\n| a | b | c | d |\n|---|---|---|---|\n| 1 | 2 | 3 | 4 |",
      }),
    );
    expect(narration.notice?.modality).toBe("sight");
    expect(narration.notice?.needs).toContain("your eyes for one table that cannot be read aloud");
    expect(narration.notice?.waitUntilStopped).toEqual(["The ten, in three families"]);
    expect(narration.notice?.text).toContain(
      "leave the section called The ten, in three families until you have stopped",
    );
    // The rest of the lesson still exports.
    expect(renderLessonNarrationText(narration)).toContain("Say it aloud.");
  });

  it("tells a pen lesson it is not a driving lesson at all", () => {
    const narration = narrateLesson(
      lesson({ type: "writing", body: "## Guided Practice\n\n- [YOU WRITE: ا three times]" }),
    );
    expect(narration.modality).toBe("pen");
    expect(narration.notice?.text).toContain("needs your hands");
    expect(narration.notice?.needs).toEqual(["a pen and something to write on"]);
    expect(renderLessonNarrationText(narration)).toContain(
      "[once you have stopped driving — write: ا three times]",
    );
  });

  it("gives a drivable lesson no notice at all", () => {
    expect(narrateLesson(lesson({ body: "## Warm-up\n\nSay it." })).notice).toBeNull();
  });
});

describe("scored activities come from the typed AST", () => {
  const activityLesson = (json: string) =>
    lesson({
      body:
        "## Wrap-up Recall\n" +
        "<!-- hl-knowledge: introduces=[]; assesses=[ES-LEX-HOLA] -->\n" +
        `<!-- hl-activity: ${json} -->\n\nWhen do you use it?`,
    });

  const valid =
    '{"id":"es-hola","kind":"text","assesses":["ES-LEX-HOLA"],"prompt":"Say hello in Spanish.",' +
    '"answer":"hola","accepted":["¡hola!"],"feedback":{"correct":"Yes.","incorrect":"Say hola."},' +
    '"response_seconds":9}';

  it("carries the compiled answer set, never a guess from the prose", () => {
    const narration = narrateLesson(activityLesson(valid));
    const activity = segments(narration).find((segment) => segment.kind === "activity");
    expect(activity).toMatchObject({
      scored: true,
      id: "es-hola",
      responseSeconds: 9,
      acceptedResponses: ["hola", "¡hola!"],
    });
    expect(renderLessonNarrationText(narration)).toContain(
      "[question — say your answer, then pause 9 seconds]",
    );
  });

  it("reports an invalid contract instead of refusing to narrate the lesson", () => {
    const broken = valid.replace('"response_seconds":9', '"response_seconds":0');
    const narration = narrateLesson(activityLesson(broken));
    expect(narration.findings.map((finding) => finding.code)).toEqual([
      "narration-activity-invalid",
    ]);
    // The prose still exports; one bad directive does not silence the lesson.
    expect(renderLessonNarrationText(narration)).toContain("When do you use it?");
  });
});

describe("chapters", () => {
  const chapterLessons = [
    lesson({ id: "ES-C01-a", sequence: 20, body: "## Warm-up\n\nSecond, by sequence." }),
    lesson({ id: "ES-C01-b", sequence: 10, body: "## Warm-up\n\nFirst, by sequence." }),
    lesson({
      id: "ES-C01-c",
      sequence: 30,
      body: "## Warm-up\n\n| a | b | c | d |\n|---|---|---|---|\n| 1 | 2 | 3 | 4 |",
    }),
  ];

  it("walks lessons in authored order and reports the drivable prefix", () => {
    const chapter = narrateChapter("spanish", 1, chapterLessons, { chapterTitle: "First Words" });
    expect(chapter.lessonIds).toEqual(["ES-C01-b", "ES-C01-a", "ES-C01-c"]);
    expect(chapter.drivablePrefix).toBe(2);
    expect(chapter.sourceHash).toMatch(/^fnv1a64:[0-9a-f]{16}$/);
  });

  it("opens the script by telling a commuter how far they get", () => {
    const text = renderChapterNarrationText(
      narrateChapter("spanish", 1, chapterLessons, { chapterTitle: "First Words" }),
      "Spanish",
    );
    expect(text.startsWith("Spanish, chapter 1: First Words.\n3 lessons. You can do the first 2")).toBe(
      true,
    );
    expect(text).toContain("Lesson 1 of 3.");
    expect(text.endsWith("\n")).toBe(true);
  });

  it("says so plainly when a whole chapter is drivable, or when none of it is", () => {
    expect(
      renderChapterNarrationText(narrateChapter("spanish", 1, chapterLessons.slice(0, 2))),
    ).toContain("All 2 can be done entirely by ear.");
    expect(
      renderChapterNarrationText(narrateChapter("spanish", 1, [chapterLessons[2]!])),
    ).toContain("save this one for when you have stopped");
  });

  it("groups a corpus into chapters, sorted by track then chapter number", () => {
    const chapters = narrationChapters(
      [
        lesson({ id: "FR-C02", language: "french", chapter: 2, body: "## Warm-up\n\nSalut." }),
        lesson({ id: "ES-C03", chapter: 3, body: "## Warm-up\n\nHola." }),
        lesson({ id: "ES-C01", chapter: 1, body: "## Warm-up\n\nHola." }),
      ],
      { titles: new Map([["spanish/1", "First Words"]]) },
    );
    expect(chapters.map((chapter) => `${chapter.language}/${chapter.chapter}`)).toEqual([
      "french/2",
      "spanish/1",
      "spanish/3",
    ]);
    expect(chapters[1]?.title).toBe("First Words");
    // A track with no authored HL05 ledger is not given an invented title.
    expect(chapters[2]?.title).toBe("Chapter 3");
  });

  it("says the lesson title once, not the same definition twice", () => {
    const text = renderLessonNarrationText(narrateLesson(lesson({ body: "## Warm-up\n\nSay it." })));
    expect(text.split("\n")[0]).toBe("hola — hello.");
    expect(text.split("\n")[1]).toBe("");
  });
});

describe("the whole corpus", () => {
  // The narration export exists to be read by a machine that cannot ask questions, so
  // "it did not throw" is not enough — these pin that nothing vanished on the way out.
  const { lessons } = loadEverything();

  it("narrates every lesson in the corpus", () => {
    const chapters = narrationChapters(lessons, { maxLinearisableTableColumns: 3 });
    const narrated = chapters.reduce((sum, chapter) => sum + chapter.lessons.length, 0);
    expect(narrated).toBe(lessons.length);
    // 378, not the 375 this was authored against: HL-C39 and HL-C40 each added a
    // Chapter 1 (Mandarin Chinese and Japanese) while this branch waited to merge, and
    // the Latin core-verb chapter (chapter 37, eight verb lessons) added one more.
    // 497 -> 513: vocabulary wave 4 (marathi/punjabi/sanskrit/urdu), 16 new chapters.
    // +1: Tamil chapter 39.
    // 659 -> 674: vocabulary wave 5 (persian/telugu/malayalam), 15 new chapters.
    // 674 -> 678: HL-C88 slices 5-6 (Spanish).
    // 682 -> 694: vocabulary wave 6, round 2 (russian/persian/urdu/bengali), 12 new chapters.
    expect(chapters.length).toBeGreaterThanOrEqual(725); // FLOOR — content only grows; // +3: HL-C113 (B1 si-condition rung) // +3: HL-C113 preterite plural // HL-C113: HL-C113 imperfect subjunctive
  });

  it("leaves no Markdown typography in the spoken script", () => {
    const text = narrationChapters(lessons, { maxLinearisableTableColumns: 3 })
      .map((chapter) => renderChapterNarrationText(chapter))
      .join("\n");
    // A speech engine handed any of these either pronounces them or stumbles.
    expect(text).not.toContain("**");
    expect(text).not.toContain("`");
    expect(text.includes("](")).toBe(false);
  });

  it("never emits a sight lesson without telling the learner first", () => {
    for (const chapter of narrationChapters(lessons, { maxLinearisableTableColumns: 3 })) {
      for (const narration of chapter.lessons) {
        if (narration.modality === "voice") continue;
        expect(narration.notice).not.toBeNull();
      }
    }
  });

  it("never drops a table it refused — every refusal is spoken", () => {
    let refusals = 0;
    for (const chapter of narrationChapters(lessons, { maxLinearisableTableColumns: 3 })) {
      for (const narration of chapter.lessons) {
        for (const block of narration.blocks) {
          for (const segment of block.segments) {
            if (segment.kind !== "table-skipped") continue;
            refusals += 1;
            expect(segment.text).toContain("Come back and look at it");
          }
        }
      }
    }
    // 66 four-or-more-column tables at the shipped width. Authored as 71 against a
    // 375-chapter corpus; the Spanish gentle-ramp split (HL-C18A) reshaped one of
    // them out of existence, Chapter 9 replaces another wide conjugation table, and
    // Chapter 10 replaces a four-column possessive table with singular known-noun
    // frames. Chapter 15 replaces its two wide teaching tables with voice-first
    // singular comparisons. Publishing Chapters 7-18 from the canonical AST
    // also replaces the Chapter-15 and Chapter-16 terminal recap tables with
    // speakable person rows. Chapter 18 removes its remaining refused wide table.
    // #12352 removes Italian Chapter 1's five-language sound-change table: the
    // pre-A1 lesson now asks the learner to hear one noctem -> notte change.
    expect(refusals).toBe(62); // #12509: -2 -- both Malayalam five-number wide reveal tables became voice-first two-column quantity/sound tables, so narration can speak them instead of refusing them // HL-C113: unchanged -- ch204 has no table for the narrator to refuse // HL-C157: ayer + hablare close A2
  });
});
