import { describe, expect, it } from "vitest";
import { parseLesson } from "../src/parse.js";
import { loadEverything } from "../src/loader.js";
import { buildCurriculumGapReport, renderCurriculumGapReport } from "../src/report.js";
import {
  DEFAULT_LINEARISABLE_TABLE_COLUMNS,
  DETACHABLE_BLOCK_TYPES,
  MODALITIES,
  MODALITY_SIGNS,
  SIGHT_CUES,
  deriveBlockModality,
  deriveLessonModality,
  drivablePrefix,
  isDetachableBlock,
  lessonCoreText,
  lessonModalities,
  lessonText,
  matchedSightCues,
  modalityFindings,
  modalityRank,
  orderChapterLessons,
  requiredChannels,
  strongerModality,
  summarizeModality,
  tableRowColumns,
  unionModalities,
  weakerModality,
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

describe("sight cues match words, not substrings", () => {
  it("no longer fires inside a longer word", () => {
    // `column` used to match `columns`, `the table` used to match `the tables`. Seven
    // corpus lessons were marked `sight` on matches like these.
    expect(matchedSightCues("the columns of a temple")).toEqual([]);
    expect(matchedSightCues("overlooked")).toEqual([]);
  });

  it("CONTROL: still fires on a real instruction", () => {
    // The cue exists to catch an instruction to the reader. Boundary matching must not
    // blunt that — a false NEGATIVE tells a driver at speed to look at a chart.
    expect(matchedSightCues("Now look at the two forms")).toContain("look at");
    expect(matchedSightCues("see the chart below")).toContain("see the");
  });

  it("keeps every cue plain lowercase, since they go into a pattern unescaped", () => {
    for (const cue of SIGHT_CUES) expect(cue).toMatch(/^[a-z ]+$/);
  });
});

// ---------------------------------------------------------------------------
// Anchoring (HL-C? / issue #12665)
// ---------------------------------------------------------------------------
//
// Word boundaries stopped `column` matching inside `columns`, but they could not tell
// "look at the chart above" from "Look at what English built on that jar". Both are the
// word `look at` standing as its own word, and the corpus is full of the second kind:
// 96 lessons were `sight` on a prose cue and nothing else, and hand-reading them found
// the majority pointing at nothing on the page — idioms ("put a fact on the table"),
// glosses (*la mesa* — "the table"), and invitations to notice a fact.
//
// The fix is NOT a longer list of phrases to exclude — that is the enumerate-the-cases
// mistake, and the next figurative sentence an author writes defeats it. It is a claim
// about what a pointing expression needs in order to be pointing: something to point at.
const TABLE = "| word | gloss |\n|---|---|\n| día | day |";

describe("a sight cue must be anchored to something on the page", () => {
  it("drops a definite reference to an artifact the document does not contain", () => {
    // THE CORE CLAIM, and it is safe by construction: a lesson holding no table cannot
    // send a reader to one, whatever its prose says. All three of these are real corpus
    // sentences that used to cost their lesson the driving edition.
    expect(matchedSightCues("you put a fact on the table and stood behind it")).toEqual([]);
    expect(matchedSightCues('**la mesa** — "the table." Latin **mensa** was the table')).toEqual(
      [],
    );
    expect(matchedSightCues("the table, the cupboard and the window")).toEqual([]);
  });

  it("CONTROL: keeps the same reference when the document DOES contain the artifact", () => {
    // The identical sentence, with a table under it. This is the pair that proves the
    // rule is reading structure rather than just deleting a phrase from the list.
    expect(matchedSightCues(`Read the table.\n${TABLE}`)).toContain("the table");
    expect(matchedSightCues(`Read the chart.\n${TABLE}`)).toContain("the chart");
    expect(matchedSightCues("look at the chart above\n![figure](f.png)")).toContain("the chart");
  });

  it("drops a pointing expression whose object is a proposition, not a thing", () => {
    // "Look at what X" asks the reader to notice a fact carried by the surrounding
    // sentences. Nothing is indicated on the page, so nothing is lost by hearing it.
    // The first is the sentence from the issue; the rest are corpus sentences.
    expect(matchedSightCues("Look at what English built on that jar")).toEqual([]);
    expect(matchedSightCues("Look at what is missing: there is no verb")).toEqual([]);
    expect(matchedSightCues("Look at how the stress moves off the stem")).toEqual([]);
  });

  it("CONTROL: keeps a pointing expression aimed at an actual object", () => {
    // A false NEGATIVE here is a driver told to look at something at speed, so these
    // are the assertions that matter most in this file.
    expect(matchedSightCues("Now look at the two forms")).toContain("look at");
    expect(matchedSightCues("Look at the accent: tê-te")).toContain("look at");
    expect(matchedSightCues("Look at 女儿, cover it, and write it from memory")).toContain(
      "look at",
    );
  });

  it("drops a cue that is being quoted rather than addressed to the reader", () => {
    // Use versus mention. The root \*spek'- is glossed “to look at, to observe”; the
    // lesson is citing words, not instructing anyone.
    expect(matchedSightCues("from the root “to look at, to observe”")).toEqual([]);
    // ...but a quotation long enough to be running prose is not a gloss, and still counts.
    expect(
      matchedSightCues(
        '"Now look at the two forms and notice which one carries the written accent, then say both"',
      ),
    ).toContain("look at");
  });

  it("keeps a self-anchored cue unconditionally, because it names its own direction", () => {
    // "below" is not open to a figurative reading the way "look at" is.
    expect(matchedSightCues("the forms shown below")).toContain("shown below");
    expect(matchedSightCues("the letter written above")).toContain("written above");
  });

  it("still over-reports `column`, on purpose, because structure cannot adjudicate it", () => {
    // THE DELIBERATE FALSE POSITIVE. `column` names part of a table, so the obvious move
    // is to anchor it to a table the way `the table` is anchored. That is wrong: an
    // author may reasonably call any aligned display a column, and `ES-C56-cion` does
    // exactly that over a blockquote ("Read those without the English column") with no
    // Markdown table anywhere in the lesson. Anchoring it would have sent that lesson to
    // the driving edition with an instruction no listener can follow. It keeps firing,
    // and "a whole column of the family table" keeps costing its lesson the car — the
    // price of not guessing in the dangerous direction.
    expect(matchedSightCues("a whole column of the family table")).toContain("column");
    expect(matchedSightCues("Read those without the English column")).toContain("column");
  });

  it("still over-reports a figurative `look at` with a concrete object, on purpose", () => {
    // The residual, recorded so the next author knows it was a decision and not an
    // oversight. "Look at your collarbone" needs no eyes on THIS page, but no mechanical
    // test separates it from "Look at the accent" without a lexicon of every thing a page
    // can hold — and a wrong guess sends a driver to a page they cannot read.
    expect(matchedSightCues("Look at your collarbone. It is the clavicle")).toContain("look at");
    expect(matchedSightCues("you can still see the seams in its everyday forms")).toContain(
      "see the",
    );
  });

  it("does not let an apostrophe forge a quoted gloss", () => {
    // REGRESSION. The mention rule first shipped with `'` as both an opening and a
    // closing delimiter, so any two contractions within sixty characters manufactured a
    // "gloss" and silently ate the cue inside it. Ordinary English, no adversary, failing
    // in the one direction this module may never fail in.
    expect(matchedSightCues("Don't look at the chart's third bar unless you're sure")).toContain(
      "look at",
    );
    expect(
      matchedSightCues("You don't need the table's rows to hear this", {
        hasPageArtifact: true,
      }),
    ).toContain("the table");
  });

  it("keeps a quoted artifact reference when the document really has the artifact", () => {
    // Mention-suppression must not out-rank the evidence: with a table present, a quoted
    // "the table" is still pointing at it.
    expect(matchedSightCues(`He said "the table" and meant it.\n${TABLE}`)).toContain("the table");
  });

  it("keeps a wh-clause whose proposition is itself visual", () => {
    // "Look at how X" is normally audible, but not when the fact lives in the layout.
    expect(matchedSightCues("Look at how these two letters differ in shape")).toContain("look at");
    expect(matchedSightCues("Look at what the third column does to the ending")).toContain(
      "look at",
    );
    // ...while the plain propositional case still drops.
    expect(matchedSightCues("Look at what English built on that jar")).toEqual([]);
  });

  it("counts an HTML figure as a page artifact, not only a pipe table", () => {
    // Markdown passes raw HTML through, so a chart can arrive as <img> or <table>. Missing
    // those would send the lesson to the driving edition.
    expect(matchedSightCues('the chart <img src="vowels.png">')).toContain("the chart");
    expect(matchedSightCues("the table <table><tr><td>a</td></tr></table>")).toContain("the table");
    expect(matchedSightCues("![vowel chart][fig1] — the chart shows it")).toContain("the chart");
  });

  it("counts a figure whose alt text is longer than the scan bound", () => {
    // The bound that fixed the quadratic must not become a false negative of its own. A
    // formant chart with a descriptive caption can easily pass 200 characters, and
    // reporting "no figure" there would gate off every artifact cue and call the lesson
    // drivable. Hitting the cap is treated as evidence of a figure, not absence of one.
    const longAlt = `![${"a vowel formant chart, ".repeat(30)}](chart.png)`;
    expect(longAlt.length).toBeGreaterThan(200);
    expect(matchedSightCues(`${longAlt}\n\nthe chart shows it`)).toContain("the chart");
    // ...while the things that merely look like an image opener still do not count.
    expect(matchedSightCues("![unclosed and the table")).toEqual([]);
    expect(matchedSightCues("![sic] more prose about the table")).toEqual([]);
  });

  it("scans a pathological body in linear time", () => {
    // The alt-text scan was `!\[[^\]]*\]\(`, which walks to end-of-text from every `!` in
    // the document: 400 KB of "![" took 49 seconds. The same body now takes ~70 ms.
    //
    // The ceiling separates the two ALGORITHMIC CLASSES and nothing finer. Local is under
    // 100 ms and CI runs perhaps 25x slower, so 15 s leaves two orders of magnitude of
    // headroom over the linear reading while still failing the quadratic one by 3.5x.
    // Do not tighten it to track the local number — that measures runner load, not the
    // thing this guards.
    const started = Date.now();
    expect(matchedSightCues("![".repeat(200_000))).toEqual([]);
    expect(matchedSightCues("look at what ".repeat(20_000))).toEqual([]);
    // ...and the quoted-span membership test, which was the second place the same
    // quadratic hid. The cue must be INSIDE the quotes: an occurrence that fires returns
    // immediately, so only a text where every occurrence is dropped walks the whole span
    // list. That shape took 39.6 s at 80,000 spans; a monotone cursor makes it ~6 ms.
    expect(matchedSightCues('"look at" '.repeat(40_000))).toEqual([]);
    expect(Date.now() - started).toBeLessThan(15_000);
  });

  it("judges a block against its whole lesson, not against its own text", () => {
    // A cue in one block routinely points at a table in another. Judging the block alone
    // would call that figurative and drop a real requirement — a false negative.
    const entry = deriveLessonModality(
      lesson({
        id: "ES-C99",
        body: `## Warm-up\n\nRead the table.\n\n## Paradigm\n\n${TABLE}`,
      }),
    );
    expect(entry.sightCues).toContain("the table");
    // The cue is in the first block ("Warm-up"); the table is in the second.
    expect(entry.blocks[0]?.title).toBe("Warm-up");
    expect(entry.blocks[0]?.modality).toBe("sight");
    expect(entry.blocks[0]?.reasons).toContain("sight-cue");
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

// ---------------------------------------------------------------------------
// Block-level modality — the interspersed writing pattern (HL08 amendment).
//
// The shape under test: an ordinary five-minute lesson, voice from end to end,
// carrying ONE short section that teaches the hand how to form a letter met
// earlier. The book prints that section like any other. A hands-free renderer
// sets it aside and delivers the rest. So the lesson has two honest answers to
// "what does this need?", and both are recorded.
// ---------------------------------------------------------------------------

/** A voice lesson with one interspersed writing segment in the middle. */
const INTERSPERSED_BODY = [
  "## Warm-up",
  "",
  "Say the greeting once more.",
  "",
  "## Writing: the letter you met on Monday",
  "",
  "Take a pen. Copy the letter three times, slowly.",
  "",
  "## Wrap-up Recall",
  "",
  "Say the greeting again.",
].join("\n");

describe("block-level modality", () => {
  it("classifies a 'Writing:' heading as the writing block type", () => {
    const entry = deriveLessonModality(lesson({ id: "TE-C01-x", body: INTERSPERSED_BODY }));
    expect(entry.blocks.map((block) => block.type)).toEqual(["warmup", "writing", "recall"]);
  });

  it("a writing block needs a pen even when its prose is plain", () => {
    const parsed = lesson({ id: "TE-C01-y", body: INTERSPERSED_BODY });
    const block = deriveBlockModality(parsed.blocks[1]!, 1);
    expect(block.modality).toBe("pen");
    expect(block.reasons).toContain("writing-block");
    expect(block.detachable).toBe(true);
  });

  it("an ordinary block with no visual dependency is voice", () => {
    const parsed = lesson({ id: "TE-C01-z", body: INTERSPERSED_BODY });
    const block = deriveBlockModality(parsed.blocks[0]!, 0);
    expect(block.modality).toBe("voice");
    expect(block.reasons).toEqual(["no-visual-dependency"]);
    expect(block.detachable).toBe(false);
  });

  it("a block's own table and cues are scanned, title included", () => {
    // The table is four columns wide on purpose. At the shipped
    // `maxLinearisableTableColumns` of 3 a two-column table is READ ALOUD, so the old
    // two-column fixture stopped raising `wide-table` the moment HL-C16 landed the
    // lineariser — it was asserting the detector fires by handing it something the
    // detector is now right to pass. A genuinely unspeakable grid keeps the test
    // testing what its name says.
    const parsed = lesson({
      id: "ES-C01-tbl",
      body:
        "## Warm-up\n\nSay it.\n\n## Grammar Lens — look at the chart\n\n" +
        "| a | b | c | d |\n| - | - | - | - |\n| 1 | 2 | 3 | 4 |\n\n## Wrap-up Recall\n\nSay it.",
    });
    const block = deriveBlockModality(parsed.blocks[1]!, 1);
    expect(block.modality).toBe("sight");
    expect(block.reasons).toEqual(expect.arrayContaining(["sight-cue", "wide-table"]));
  });

  it("both writing and script are detachable — a table block is not", () => {
    // `script` joined `writing`: HL00 makes the inline-letters section optional
    // scaffolding, so a hands-free renderer may skip it. Detachable is about what a
    // renderer may set aside, NOT about what the learner's hand must do — which is why
    // adding it here no longer drags a lesson to `pen`.
    expect([...DETACHABLE_BLOCK_TYPES]).toEqual(["writing", "script"]);
    const parsed = lesson({
      id: "HI-C01-s",
      body: "## Warm-up\n\nSay it.\n\n## Script — क\n\nA vertical bar.\n\n## Wrap-up Recall\n\nSay it.",
    });
    // Warm-up and Wrap-up are load-bearing prose; the Script section is not.
    expect(parsed.blocks.map(isDetachableBlock)).toEqual([false, true, false]);

    // A table-bearing block is NOT detachable: the table is the content, so skipping it
    // would silently drop what the lesson teaches rather than defer optional scaffolding.
    const tabular = lesson({
      id: "ES-C01-t",
      body: "## Warm-up\n\nSay it.\n\n## Grammar Lens — forms\n\n| a | b |\n| - | - |\n| 1 | 2 |",
    });
    expect(tabular.blocks.map(isDetachableBlock)).toEqual([false, false]);
  });

  it("the full modality is pen and the core is voice — one lesson, two answers", () => {
    const entry = deriveLessonModality(lesson({ id: "TE-C01-both", body: INTERSPERSED_BODY }));
    // What the BOOK signs: this lesson does contain handwriting.
    expect(entry.derived).toBe("pen");
    expect(entry.modality).toBe("pen");
    expect(entry.reasons).toContain("writing-block");
    // What a hands-free view can deliver: everything but the writing segment.
    expect(entry.coreDerived).toBe("voice");
    expect(entry.coreModality).toBe("voice");
    expect(entry.coreReasons).toEqual(["no-visual-dependency"]);
    expect(entry.writingSegments).toEqual(["Writing: the letter you met on Monday"]);
  });

  it("sight cues inside the writing segment do not follow it out of the core", () => {
    const entry = deriveLessonModality(
      lesson({
        id: "TE-C01-cue",
        body: [
          "## Warm-up",
          "",
          "Say it.",
          "",
          "## Writing: the tick on top",
          "",
          "Look at the shape above, then copy it.",
          "",
          "## Wrap-up Recall",
          "",
          "Say it.",
        ].join("\n"),
      }),
    );
    expect(entry.sightCues).toContain("look at");
    expect(entry.coreModality).toBe("voice");
    expect(entry.coreReasons).toEqual(["no-visual-dependency"]);
  });

  it("a sight cue in the ORDINARY prose still reaches the core", () => {
    const entry = deriveLessonModality(
      lesson({
        id: "TE-C01-cue2",
        body: [
          "## Warm-up",
          "",
          "Look at the table on the previous page.",
          "",
          "## Writing: the tick on top",
          "",
          "Copy it three times.",
          "",
          "## Wrap-up Recall",
          "",
          "Say it.",
        ].join("\n"),
      }),
    );
    expect(entry.coreModality).toBe("sight");
    expect(entry.coreReasons).toContain("sight-cue");
  });

  it("a type: writing lesson has a pen core — there is nothing to set aside", () => {
    const entry = deriveLessonModality(
      lesson({ id: "TA-W01", type: "writing", body: INTERSPERSED_BODY }),
    );
    expect(entry.modality).toBe("pen");
    expect(entry.coreModality).toBe("pen");
    expect(entry.coreReasons).toContain("writing-type");
  });

  it("the core is never stronger than the full modality, even under an override", () => {
    const entry = deriveLessonModality(
      lesson({
        id: "ES-C01-ov",
        modality: "voice",
        modalityReason: "the table is decorative",
        // Four columns, so the core really does derive as `sight` at the shipped
        // width — otherwise the override has nothing to cap and the test passes
        // vacuously.
        body: "## Warm-up\n\n| yo | tú | él | ella |\n| - | - | - | - |\n| soy | eres | es | es |\n\n## Wrap-up Recall\n\nSay it.",
      }),
    );
    expect(entry.coreDerived).toBe("sight");
    // The author took responsibility for `voice`; the core cannot sit above it.
    expect(entry.modality).toBe("voice");
    expect(entry.coreModality).toBe("voice");
  });

  it("lessonCoreText drops the writing segment and keeps everything else", () => {
    const parsed = lesson({ id: "TE-C01-t", body: INTERSPERSED_BODY });
    expect(lessonText(parsed)).toContain("Copy the letter three times");
    expect(lessonCoreText(parsed)).not.toContain("Copy the letter three times");
    expect(lessonCoreText(parsed)).toContain("Say the greeting once more");
  });

  it("reports a lesson that sprouts more than one writing segment", () => {
    const entry = deriveLessonModality(
      lesson({
        id: "TE-C01-many",
        body: [
          "## Warm-up",
          "",
          "Say it.",
          "",
          "## Writing: the first letter",
          "",
          "Copy it.",
          "",
          "## Writing: the second letter",
          "",
          "Copy it too.",
          "",
          "## Wrap-up Recall",
          "",
          "Say it.",
        ].join("\n"),
      }),
    );
    const findings = modalityFindings(entry);
    expect(findings).toHaveLength(1);
    expect(findings[0]?.code).toBe("modality-writing-segment-not-separable");
    expect(findings[0]?.message).toContain("may carry one");
  });

  it("a type: writing lesson may carry as many writing blocks as it likes", () => {
    const entry = deriveLessonModality(
      lesson({
        id: "TA-W09",
        type: "writing",
        body: [
          "## Warm-up",
          "",
          "Say it.",
          "",
          "## Writing: ம",
          "",
          "Copy it.",
          "",
          "## Writing: ண",
          "",
          "Copy it too.",
          "",
          "## Wrap-up Recall",
          "",
          "Say it.",
        ].join("\n"),
      }),
    );
    expect(modalityFindings(entry)).toEqual([]);
  });

  it("the drivable prefix runs through an interspersed lesson instead of stopping at it", () => {
    const entries = lessonModalities([
      lesson({ id: "TE-C01-a", sequence: 10 }),
      lesson({ id: "TE-C01-b", sequence: 20, body: INTERSPERSED_BODY }),
      lesson({ id: "TE-C01-c", sequence: 30 }),
      lesson({ id: "TE-C01-d", sequence: 40, body: "## Warm-up\n\nLook at the chart." }),
    ]);
    // Under the pre-amendment rule the pen lesson at sequence 20 ended the run at 1.
    expect(drivablePrefix(entries)).toBe(3);
  });

  it("the rollup keeps the book's counts and the hands-free count separate", () => {
    const summary = summarizeModality([
      lesson({ id: "TE-C01-a", chapter: 1, sequence: 10 }),
      lesson({ id: "TE-C01-b", chapter: 1, sequence: 20, body: INTERSPERSED_BODY }),
    ]);
    const track = summary.tracks[0]!;
    // The book prints one voice lesson and one pen lesson…
    expect(track.voice).toBe(1);
    expect(track.pen).toBe(1);
    // …while both are reachable without a hand.
    expect(track.coreVoice).toBe(2);
    expect(track.drivablePercent).toBe(100);
    expect(summary.lessonsWithWritingSegments).toBe(1);
    expect(track.chapters[0]?.coreVoice).toBe(2);
  });

  it("strongerModality and weakerModality order by the MODALITIES rank", () => {
    expect(strongerModality("voice", "pen")).toBe("pen");
    expect(strongerModality("sight", "voice")).toBe("sight");
    expect(weakerModality("pen", "sight")).toBe("sight");
    expect(weakerModality("voice", "sight")).toBe("voice");
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
    // 75, not 50. `drivablePercent` counts the CORE, and ES-C02-a's only obstacle is a
    // detachable `## Script` section — so once that section is set aside the lesson is
    // listenable, and three of the four lessons are. Only the `writing`-TYPE lesson
    // (ES-C01-c) is genuinely lost to a driver: the whole lesson is the handwriting, so
    // there is nothing separable to skip.
    expect(track?.drivablePercent).toBe(75);
    // ...and chapter 2 now starts by ear, which is the point of the split.
    expect(track?.drivablePrefixTotal).toBe(3);

    const [first, second] = track?.chapters ?? [];
    expect(first).toMatchObject({
      chapter: 1,
      lessonCount: 3,
      drivablePrefix: 2,
      firstNonVoiceLesson: "ES-C01-c",
      modalities: ["voice", "sight", "pen"],
    });
    // Chapter 2 used to be prefix-0, blocked at its first lesson by the script section.
    // That section detaches, so the chapter is startable and has no blocker at all.
    expect(second).toMatchObject({ chapter: 2, drivablePrefix: 1, firstNonVoiceLesson: null });
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
    // `the chart` is an ARTIFACT cue, so it needs a chart to be pointing at; with a table
    // present both fire, and the order is the declaration order of SIGHT_CUE_RULES.
    expect(matchedSightCues("LOOK AT the chart\n| a | b |\n|---|---|\n| c | d |")).toEqual([
      "look at",
      "the chart",
    ]);
    expect(matchedSightCues("say it aloud")).toEqual([]);
    // Every cue still fires on a bare occurrence once its anchor is satisfied. The
    // artifact cues get a table; the rest need nothing.
    for (const cue of SIGHT_CUES) {
      expect(matchedSightCues(`x ${cue} y`, { hasPageArtifact: true })).toContain(cue);
    }
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
    // NEITHER chapter is blocked now. Chapter 3's only obstacle is a `## Script`
    // section, which detaches — a driver skips the glyphs and keeps the lesson — so it
    // starts by ear. Chapter 4 was never blocked: its override is unexplained and
    // reported, but nothing here gates, so the authored `voice` still takes effect.
    expect(report.summary.chaptersWithoutDrivablePrefix).toBe(0);
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
          // A four-column paradigm, NOT a script section: the lineariser refuses it and
          // it is not detachable, so it genuinely blocks the chapter. The fixture used to
          // be a `## Script` block, which stopped blocking once script became detachable —
          // and this test is about whether a blocker gets NAMED, so it needs a real one.
          lesson({
            id: "ES-C05",
            chapter: 5,
            sequence: 10,
            body: "## Warm-up\n\n| a | b | c | d |\n| - | - | - | - |\n| 1 | 2 | 3 | 4 |",
          }),
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
  // The HL08 baseline was 51 `pen`, 7 script-block lessons and 322 table-bearing
  // lessons among the remaining 1,038, giving 694 drivable at 63%. HL-C32 then
  // remediated the Russian track: fourteen of its lessons were `sight` only because
  // a cross-language word→gloss list had been set as a Markdown table, so the tables
  // became speakable prose and the lessons became `voice`. That moved the corpus to
  // 708 drivable (65%) and left 308 table-bearing lessons. The numbers below are
  // re-pinned to that measurement, not relaxed — they still fail on any silent change
  // to the parser, the table detector or the cue list, most dangerously a block field
  // rename that would make every lesson look clean.
  //
  // HL08 itself recorded 695 drivable from 56 cue-bearing lessons; this
  // implementation's published cue list matches 61 and therefore started one lesson
  // lower. The spec's exact cue list was never recorded, and the detector is
  // deliberately NOT tuned to close a one-lesson gap.
  // WHY THESE ARE INVARIANTS, NOT ABSOLUTE COUNTS.
  //
  // These assertions used to hard-code the corpus totals (1096 lessons, 708 voice,
  // 337 sight, 1038 non-script, ...). That made this file a serialization point:
  // EVERY pull request that adds, splits or reclassifies a lesson had to edit the
  // same three lines, so concurrent work collided here rather than in the content.
  // Five branches conflicted on exactly this file in one afternoon.
  //
  // The drift protection did not need to live here. `core/lesson-modality.json`
  // (HL-C44) is regenerated by `modality-cli --write`, carries an fnv1a64
  // sourceHash, and CI runs `--check`. A silent parser change, a renamed block
  // field, or a broken table detector all move that manifest and fail the gate --
  // and the fix is to re-run the generator, which merges cleanly, instead of
  // hand-reconciling numbers across branches.
  //
  // So what stays here is what a generated snapshot CANNOT catch: the internal
  // consistency the derivation must always satisfy, whatever the corpus size.
  it("keeps the modality partition internally consistent", () => {
    const { lessons } = loadEverything();
    const summary = summarizeModality(lessons);

    // Every lesson lands in exactly one channel, and nothing is lost or double-counted.
    expect(summary.voice + summary.sight + summary.pen).toBe(summary.totalLessons);
    expect(summary.totalLessons).toBe(lessons.length);

    // The published percentage is honestly derived from the voice count, rather
    // than being a separately maintained number that could drift away from it.
    // (`drivableLessons` lives on the generated manifest's summary, not here --
    // ModalitySummary exposes the channel counts and the percentage.)
    // The published percentage describes the DRIVING EDITION, which reads the core —
    // the lesson minus the sections a hands-free renderer sets aside. It was equal to
    // voice/total only while `writing` was the sole detachable type and no lesson had
    // one; now that the inline-letters section detaches, core and whole legitimately
    // differ and the percentage must follow the core or it would advertise the wrong
    // book. Still derived, never separately maintained.
    expect(summary.drivablePercent).toBe(
      Math.round((summary.coreVoice / summary.totalLessons) * 100),
    );
    // The whole-lesson partition still has to close.
    expect(summary.voice + summary.sight + summary.pen).toBe(summary.totalLessons);
    // And the core is never weaker than the whole: detaching can only help.
    expect(summary.coreVoice).toBeGreaterThanOrEqual(summary.voice);

    // A corpus that derived as entirely one channel would mean the detector had
    // stopped detecting -- the failure mode a fixed number was really guarding.
    expect(summary.voice).toBeGreaterThan(0);
    expect(summary.sight).toBeGreaterThan(0);
    expect(summary.pen).toBeGreaterThan(0);
  });

  it("keeps the structural partition the derivation is built on", () => {
    const { lessons } = loadEverything();
    const writing = lessons.filter((entry) => entry.realization.type === "writing");
    const nonWriting = lessons.filter((entry) => entry.realization.type !== "writing");
    const scripted = nonWriting.filter((entry) =>
      entry.blocks.some((block) => block.type === "script"),
    );
    const remaining = nonWriting.filter(
      (entry) => !entry.blocks.some((block) => block.type === "script"),
    );

    // The three groups partition the corpus exactly -- this is what a rename of
    // `realization.type` or `block.type` would break, and it holds at any size.
    expect(writing.length + scripted.length + remaining.length).toBe(lessons.length);
    expect(writing.length).toBeGreaterThan(0);
    expect(remaining.length).toBeGreaterThan(0);

    // Table detection still finds tables. A field rename would silently zero this.
    expect(
      remaining.filter((entry) => widestTableColumns(lessonText(entry)) > 0).length,
    ).toBeGreaterThan(0);
  });

  // The block-level amendment is a no-op on the corpus AS IT STANDS, and that is the
  // claim worth pinning: no track has yet authored an interspersed writing segment, so
  // every lesson's core equals its full modality and no published number moves.
  //
  // It is pinned as an EQUALITY, not as a literal. An earlier draft also asserted
  // `coreVoice === 708`, which re-serialized the very file that was rewritten to stop
  // being a serialization point — four content branches moved that number (Latin
  // payoffs, the Spanish ramp split, Mandarin, Japanese) before this one landed, and
  // each would have had to edit it here for no added coverage. `coreVoice === voice`
  // is the whole claim; the absolute corpus totals are pinned once, in
  // `modality-manifest.test.ts`, against the generated manifest.
  //
  // When the first interspersed lesson lands, `lessonsWithWritingSegments` rises and
  // this equality breaks — which is the point. Record which track did it; never loosen.
  it("pins the core as strictly better than the whole, now that writing segments exist", () => {
    const { lessons } = loadEverything();
    const summary = summarizeModality(lessons);
    // This read 0 for the whole life of the corpus: HL-C41 built the block-level
    // modality machinery and nothing used it. Tamil's drizzled letter segments
    // (HL11) are the first lessons to author a `## Writing:` section, so the
    // machinery is finally exercised by real content rather than by fixtures.
    //
    // Derive the count from the language-owned lesson corpus. A global literal
    // made every new writing microstep edit this shared algorithm suite.
    expect(summary.lessonsWithWritingSegments).toBe(
      lessons.filter((lesson) => lesson.blocks.some((block) => block.type === "writing")).length,
    );
    expect(summary.lessonsWithWritingSegments).toBeGreaterThan(0);
    // coreVoice NO LONGER equals voice, and that is the whole point of the split: 240
    // inline-letters sections detach, so the core of those lessons is listenable even
    // though the lesson as printed needs eyes.
    expect(summary.coreVoice).toBeGreaterThan(summary.voice);
    for (const entry of lessonModalities(lessons)) {
      // The invariant a hands-free view relies on: the core never asks for more
      // than the whole lesson does.
      expect(modalityRank(entry.coreModality)).toBeLessThanOrEqual(modalityRank(entry.modality));
    }
  });

  // HL-C16 built the lineariser HL08's migration step 3 promised, and moved the shipped
  // `maxLinearisableTableColumns` from 0 to 3. That is a large claim — it converts a
  // great many `sight` lessons into `voice` ones — so it needs a control proving the
  // jump came from the lineariser and not from a detector that quietly stopped
  // detecting.
  //
  // The control is written as a DIFFERENCE, not as two absolute corpus counts. An
  // earlier draft pinned `voice === 925` at width 3 and `voice === 694` at width 0;
  // both numbers were measured against a 1,096-lesson corpus that four content branches
  // had already moved before this one landed, and each would have had to edit them here
  // for no added protection. Re-deriving the same corpus at two widths and comparing is
  // strictly stronger: it holds at any corpus size, and it still fails loudly if the
  // lineariser stops linearising or starts swallowing lessons it should refuse.
  it("attributes the drivable gain to the lineariser, at any corpus size", () => {
    const { lessons } = loadEverything();
    const shipped = summarizeModality(lessons);
    const preLineariser = summarizeModality(lessons, { maxLinearisableTableColumns: 0 });

    // The shipped width is a configuration fact, not a corpus measurement, so it is
    // pinned absolutely. Three is where a table stops being labelled facts a listener
    // can hold and starts being a grid whose meaning lives across rows.
    expect(shipped.maxLinearisableTableColumns).toBe(3);
    expect(preLineariser.maxLinearisableTableColumns).toBe(0);

    // The lineariser only ever moves lessons from `sight` to `voice`...
    expect(shipped.voice).toBeGreaterThan(preLineariser.voice);
    expect(shipped.sight).toBeLessThan(preLineariser.sight);
    // ...never into or out of `pen`, and never creates or loses a lesson.
    expect(shipped.pen).toBe(preLineariser.pen);
    expect(shipped.totalLessons).toBe(preLineariser.totalLessons);
    expect(shipped.voice - preLineariser.voice).toBe(preLineariser.sight - shipped.sight);
  });

  // The lessons that STILL need eyes, by cause. `wide-table` alone is the burn-down list
  // HL08's migration step 4 names: reshaping just those tables would move exactly those
  // lessons into the car, because they have no other reason to need eyes.
  it("keeps every sight lesson attributable to a known cause", () => {
    const { lessons } = loadEverything();
    const sight = lessonModalities(lessons).filter((entry) => entry.modality === "sight");
    const known = new Set(["script-block", "sight-cue", "wide-table"]);

    // A `sight` lesson with no recorded reason would be unexplainable to a learner and
    // unfixable by an author — this is what a broken detector actually looks like.
    expect(sight.length).toBeGreaterThan(0);
    for (const entry of sight) {
      expect(entry.reasons.length).toBeGreaterThan(0);
      for (const reason of entry.reasons as string[]) expect(known.has(reason)).toBe(true);
    }

    // The burn-down list is non-empty and is a strict subset of the wide-table lessons.
    const wideTable = sight.filter((entry) => (entry.reasons as string[]).includes("wide-table"));
    const wideTableOnly = sight.filter(
      (entry) => entry.reasons.length === 1 && entry.reasons[0] === "wide-table",
    );
    expect(wideTable.length).toBeGreaterThan(0);
    expect(wideTableOnly.length).toBeGreaterThan(0);
    expect(wideTableOnly.length).toBeLessThanOrEqual(wideTable.length);
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
