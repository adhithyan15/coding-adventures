// HL09 §3.1 criterion 4 — the REINFORCEMENT transition, proved on a fixture.
//
// ---------------------------------------------------------------------------
// Why this file exists
// ---------------------------------------------------------------------------
//
// `level-gate.test.ts` already proves criterion 2b (verb composition) the right
// way: a synthetic track that is short by exactly one verb, and then the SAME
// track with that one verb added, attaining the level. Two runs differing by one
// authored fact, with opposite verdicts. That shape is what makes a gate test
// mean something — "it failed" is not the same as "it failed for the reason the
// test is about", and only the counterfactual can tell those apart.
//
// Criterion 4 has never had that shape. The corpus proves the RED half of it
// abundantly — every real track is blocked on reinforcement, several by
// hundreds of atoms — and proves the GREEN half nowhere at all. No track in the
// corpus has ever crossed the reinforcement line, so no test has ever observed
// the gate stop blocking on it. Delete the `reinforcement` failure branch from
// `runLevelGate` and every existing assertion about it still fails for the right
// tracks; INVERT it (block when an atom is well reinforced) and most of the
// corpus assertions would still pass, because the corpus is uniformly red.
//
// So the transition moves into a fixture, where authoring can never turn it
// green and content growth can never turn it red.
//
// ---------------------------------------------------------------------------
// The anti-vacuity argument, stated explicitly
// ---------------------------------------------------------------------------
//
// The blocked half alone would pass against a gate that blocks EVERYTHING, and
// the attained half alone would pass against a gate that blocks NOTHING. Only
// the pair, over two tracks differing by one revisit and nothing else, pins the
// gate to the actual threshold. `buildLessons(secondRevisit)` is therefore the
// single source of both runs, and the boolean it takes reaches exactly one place
// in the whole fixture: whether the third lesson names the atom in its
// `practises.knowledge`. Lesson count, ids, headwords, concept tags, chapters,
// reading order, the spine and the curriculum are all shared, byte for byte,
// between the two runs — the spine and curriculum are literally the same objects.
//
// ---------------------------------------------------------------------------
// How the fixture clears the OTHER five criteria
// ---------------------------------------------------------------------------
//
// pre-A1 is chosen because it is the cheapest rung: 300 headwords and 5 verbs,
// against A1's 600 and 40. Even so it takes 300 lessons, which is why they are
// generated rather than written out.
//
//   1. spine-nodes     one pre-A1 node, realized by a segment that lists every
//                      lesson. (Levels above pre-A1 have no authored node, so
//                      they can never be attained — see the note on `blockers`
//                      at the bottom of the attained case.)
//   2. vocabulary      exactly `LEVEL_VOCABULARY["pre-A1"]` distinct headwords.
//  2b. verb-vocabulary the first `LEVEL_VERB_VOCABULARY["pre-A1"]` lessons carry
//                      a `*-VERB-*` concept tag, which is the signal
//                      `verbVocabularyOf` reads.
//   3. atom-budget     one lesson introduces one atom; the other 299 introduce
//                      none. Nothing is near the per-lesson budget.
//   4. reinforcement   THE CRITERION UNDER TEST — one revisit, then two.
//   5. writing-stage   `writingStages` is left out of the input, which is the
//                      documented "caller has not loaded HL16 policy" case and
//                      skips criterion 5 entirely.
//
// Continuity is measured for real, by `measureContinuity`, rather than stubbed
// as `{ reinforcement: [] }` the way the neighbouring fixtures stub it. That is
// deliberate: a stub would let this file assert whatever it liked about
// `revisits`, and the thing being proved is precisely that the gate and
// `measureContinuity` agree on what a revisit IS. The count has to come from the
// module that computes it.

import { describe, expect, it } from "vitest";
import { loadChapterPolicy } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";
import type { ParsedLesson } from "../src/parse.js";
import { LEVEL_VOCABULARY, LEVEL_VERB_VOCABULARY, runLevelGate } from "../src/level-gate.js";
import { summarizeLevels } from "../src/levels.js";
import { measureRamp } from "../src/ramp.js";
import { measureContinuity } from "../src/continuity.js";

/** A track name no real corpus track uses, so the gate reports exactly one track. */
const LANGUAGE = "gamma";

/** 300 at the time of writing. Read from the constant so a retune moves the fixture too. */
const LESSON_COUNT = LEVEL_VOCABULARY["pre-A1"];

/** 5 at the time of writing. Same reasoning. */
const VERB_COUNT = LEVEL_VERB_VOCABULARY["pre-A1"];

/**
 * The one atom the whole fixture turns on.
 *
 * Deliberately NOT named `*-ETYMON-*`: `isEtymologyAtom` waives those from
 * criterion 4 entirely, and an atom the gate is allowed to ignore cannot
 * demonstrate the gate's threshold.
 */
const ATOM = "GA-GREETING-HELLO";

/** Stable, zero-padded so lexicographic order and numeric order agree. */
const lessonId = (index: number) => `GA-${String(index + 1).padStart(3, "0")}`;

/**
 * The fixture's lessons. The ONLY thing `secondRevisit` changes is whether the
 * third lesson practises `ATOM`.
 *
 * Reading order matters here in a way it does not for the neighbouring
 * fixtures, because `measureContinuity` counts a revisit as a LATER lesson
 * naming the atom. Every lesson therefore declares an explicit `sequence`, so
 * the order is authored data rather than a tie-break on lesson id.
 *
 * Positions after sorting: index 0 introduces the atom, index 1 practises it,
 * index 2 practises it only in the repaired run.
 */
function buildLessons(secondRevisit: boolean): ParsedLesson[] {
  return Array.from({ length: LESSON_COUNT }, (_, index) => {
    const directives: string[] = [];
    if (index === 0) directives.push(`introduces.knowledge: [${ATOM}]`);
    if (index === 1) directives.push(`practises.knowledge: [${ATOM}]`);
    // THE ONE DIFFERENCE BETWEEN THE TWO RUNS.
    if (index === 2 && secondRevisit) directives.push(`practises.knowledge: [${ATOM}]`);
    return parseLesson(
      [
        "---",
        "schema_version: 2",
        `id: ${lessonId(index)}`,
        "chapter: 1",
        `sequence: ${(index + 1) * 10}`,
        "type: word",
        `headword: gw${index + 1}`,
        `gloss: word ${index + 1}`,
        // The verb tag is what criterion 2b reads; `(^|-)VERB-` anchors on the hyphen.
        `concept_tag: ${index < VERB_COUNT ? "GA-VERB" : "GA-WORD"}-${index + 1}`,
        ...directives,
        "---",
        "",
        `# ${lessonId(index)}`,
        "",
        "Say it.",
        "",
      ].join("\n"),
      LANGUAGE,
    );
  });
}

/**
 * The spine and curriculum, shared by both runs.
 *
 * Held at module scope rather than rebuilt per run so that "the two runs differ
 * by one revisit and nothing else" is a fact about object identity, not a claim
 * about two constructors happening to agree.
 */
const SPINE = {
  version: 1,
  stages: ["pre-A1", "A1", "A2", "B1", "B2", "C1", "C2"],
  nodes: [
    {
      id: "GA-PRE",
      stage: "pre-A1",
      strand: "LEXICON",
      canDo: "name a word",
      prerequisites: [],
      core: true,
      concepts: [],
    },
  ],
};

const CURRICULA = [
  {
    version: 1,
    language: LANGUAGE,
    path: [
      {
        id: "gamma-pre-a1",
        spine_node: "GA-PRE",
        lessons: Array.from({ length: LESSON_COUNT }, (_, index) => lessonId(index)),
        before: [],
        inline: [],
        after: [],
      },
    ],
    spine: { "GA-PRE": { segments: ["gamma-pre-a1"], omits: [], relocates: {} } },
    extensions: [],
  },
];

/**
 * Run the whole HL09 §3.1 gate over the fixture, and hand back the continuity
 * measurement beside it so a test can assert what the gate was reading.
 *
 * `writingStages` is omitted on purpose — see criterion 5 in the header.
 */
function runFixture(secondRevisit: boolean) {
  const lessons = buildLessons(secondRevisit);
  const continuity = measureContinuity(lessons);
  const gate = runLevelGate({
    lessons,
    levels: summarizeLevels(lessons, CURRICULA, SPINE),
    curricula: CURRICULA,
    spine: SPINE,
    ramp: measureRamp(lessons, loadChapterPolicy()),
    continuity,
  });
  const track = gate.tracks.find((candidate) => candidate.language === LANGUAGE)!;
  const defect = continuity.reinforcement.find((entry) => entry.atom === ATOM);
  return { track, defect };
}

describe("criterion 4 — the reinforcement transition", () => {
  it("blocks a level on reinforcement alone when one atom is one revisit short", () => {
    const { track, defect } = runFixture(false);

    // The measurement the gate is about to read. Asserted first, and by NAME,
    // because "the gate blocked" is only interesting once we know it blocked on
    // an atom that really does have one revisit and not two. `measureContinuity`
    // counts distinct LATER lessons whose `practises.knowledge` names the atom —
    // here, exactly one.
    expect(defect).toMatchObject({ atom: ATOM, introducedBy: lessonId(0), revisits: 1 });

    // The verdict. Naming the criterion rather than counting the blockers is the
    // load-bearing part: a fixture short on vocabulary would also produce "one
    // blocker", and this assertion would not notice.
    expect(track.blockers.map((blocker) => blocker.criterion)).toEqual(["reinforcement"]);
    expect(track.blockers[0]).toMatchObject({ criterion: "reinforcement", shortfall: 1 });
    expect(track.blockers[0]!.detail).toContain("1 atom(s) at or below pre-A1");

    // No etymology atom is in play, so the waiver must not be mentioned at all.
    // If this ever starts failing, `isEtymologyAtom` has begun matching `ATOM`
    // and the fixture is no longer testing what it says it is.
    expect(track.blockers[0]!.detail).not.toContain("waived");

    expect(track).toMatchObject({ attained: null, inProgressAt: "pre-A1" });
  });

  it("attains that level when the same track gets the one missing revisit", () => {
    const { track, defect } = runFixture(true);

    // Same atom, same introducing lesson, one more revisit. Nothing else moved.
    expect(defect).toMatchObject({ atom: ATOM, introducedBy: lessonId(0), revisits: 2 });

    // Worth stating plainly, because it is the gate's real behaviour and it
    // surprises people: the atom is STILL a continuity defect in this run. It
    // reappears one lesson after it was taught and then never again, so it
    // misses the R2, R3 and R4 windows and `measureContinuity` reports it. The
    // defect exists in BOTH runs; only `revisits` differs. Criterion 4 is a
    // count of revisits, not a check that the windows were hit — so the gate's
    // verdict flips while the underlying defect does not. That is the threshold
    // this file pins, and the two assertions below are what pin it.
    expect(defect).toBeDefined();
    expect(defect!.missed).toContain("R2");

    expect(track.attained).toBe("pre-A1");
    expect(track.blockers.map((blocker) => blocker.criterion)).not.toContain("reinforcement");

    // `blockers` is NOT empty here, and cannot be. It always reports the FIRST
    // level that failed, and pre-A1 no longer fails — so it now describes A1,
    // which this fixture makes no attempt to satisfy: no A1 spine node is
    // authored, 300 headwords is half of A1's 600, and 5 verbs is an eighth of
    // A1's 40. Spelling those three out by name, rather than asserting a length,
    // is what proves the list moved UP a rung instead of merely shrinking.
    expect(track).toMatchObject({ inProgressAt: "A1" });
    expect(track.blockers.map((blocker) => blocker.criterion)).toEqual([
      "spine-nodes",
      "vocabulary",
      "verb-vocabulary",
    ]);
  });

  it("changes nothing but the one revisit between the two runs", () => {
    // The claim the two cases above rest on, checked rather than asserted in a
    // comment. If a future edit makes `buildLessons` vary anything else by the
    // flag, the transition stops being attributable to reinforcement and this
    // test says so before the other two start lying.
    const blocked = buildLessons(false);
    const repaired = buildLessons(true);

    expect(repaired).toHaveLength(blocked.length);

    const differing = blocked
      .map((lesson, index) => ({ index, lesson, other: repaired[index]! }))
      .filter(({ lesson, other }) => JSON.stringify(lesson) !== JSON.stringify(other));

    expect(differing.map((entry) => entry.index)).toEqual([2]);
    expect(differing[0]!.lesson.frontmatter["practises.knowledge"]).toBeUndefined();
    expect(differing[0]!.other.frontmatter["practises.knowledge"]).toEqual([ATOM]);
  });
});
