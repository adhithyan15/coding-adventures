// HL09 §3.1 — what it takes to CLAIM a level. See src/level-gate.ts for why.

import { describe, expect, it } from "vitest";
import { loadEverything, loadChapterPolicy, loadTrackChapters } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";
import { buildCurriculumGapReport } from "../src/report.js";
import { LEVEL_VOCABULARY, runLevelGate, isEtymologyAtom } from "../src/level-gate.js";
import { summarizeLevels } from "../src/levels.js";
import { measureRamp } from "../src/ramp.js";
import { measureContinuity } from "../src/continuity.js";
import type { ContinuityReport } from "../src/continuity.js";
import type { WritingStageReport } from "../src/writing-stages.js";

// Built ONCE for the file, not once per test. The gap report now walks continuity
// (~900ms) and the level gate on top of everything else, so rebuilding an identical
// report five times cost ~18s and timed out the 5s default on CI. Memoising is also
// simply correct: every test wants the same report.
let cached: ReturnType<typeof buildCurriculumGapReport> | undefined;
function realReport() {
  if (cached) return cached;
  const e = loadEverything();
  cached = buildCurriculumGapReport({
    registry: e.registry,
    lessons: e.lessons,
    books: e.books,
    curricula: e.curricula,
    spine: e.spine,
    trackChapters: loadTrackChapters(),
    chapterPolicy: loadChapterPolicy(),
  });
  return cached;
}

describe("the gate that would have caught the A2 claim", () => {
  // Explicit budget: `realReport()` builds the entire gap report over the whole corpus.
  // At 1,313 lessons that runs past vitest's 5,000 ms default under full-suite parallel
  // load, while passing in isolation — so a per-file run will not reproduce it.
  it("separates what a track TOUCHES from what it has ATTAINED", { timeout: 30_000 }, () => {
    const gate = realReport().levelGate!;
    const spanish = gate.tracks.find((t) => t.language === "spanish")!;

    // The number that misled: one lesson pointing at one node moves `touches`, and
    // three lessons on `SPINE-REPORT-WHAT-OTHERS-SAID` have now moved it to B2 --
    // out of a track whose vocabulary has not cleared pre-A1. `attained` is unmoved
    // at null, which is exactly the distinction this module exists for, and the gap
    // between the two numbers widens every time the ladder is climbed.
    // HL-C175: chapter 272 touches SPINE-INFER-IMPLICIT-MEANING, so `touches`
    // rises to C1 while `attained` stays null. That widening gap is not a
    // regression -- it is the entire point of this module: five lessons on a C1
    // node do not make a C1 reader, and the gate says so.
    // HL-C178: chapter 274 touches SPINE-READ-CULTURAL-WEIGHT, so `touches`
    // reaches the top of the ladder while `attained` is STILL null. That is the
    // widest this gap has ever been, and it is the clearest statement of what
    // this module is for: a track can touch C2 and have attained nothing.
    expect(spanish.touches).toBe("C2");
    // The number that does not: Spanish has not met even pre-A1's criteria.
    expect(spanish.attained).toBeNull();
    expect(spanish.inProgressAt).toBe("pre-A1");
  });

  it("reports every track as overstating, which is the finding", () => {
    // All 23 tracks touch a level none of them has attained. That is not 23 bugs —
    // it is one measurement having been read as another for the whole project.
    const gate = realReport().levelGate!;
    expect(gate.summary.tracksOverstating).toBe(23);
    expect(gate.summary.tracksWithAnyLevel).toBe(0);
    for (const level of Object.values(gate.summary.attainedByLevel)) {
      expect(level).toBe(0);
    }
  });

  it("names which criterion failed and by how much, not just that one did", () => {
    const gate = realReport().levelGate!;
    const spanish = gate.tracks.find((t) => t.language === "spanish")!;
    const vocab = spanish.blockers.find((b) => b.criterion === "vocabulary")!;

    // A bare `false` would move the argument rather than settle it.
    expect(vocab.detail).toContain(String(LEVEL_VOCABULARY["pre-A1"]));
    // This used to assert Spanish also carried a `reinforcement` blocker. It no
    // longer does: the pre-A1 reinforcement tranche closed all 24 of its
    // under-reinforced atoms, and Spanish is now the only track in the corpus with
    // criterion 4 clean. Pinning WHICH criteria are open would make this test a
    // ledger of unfinished work that every closing tranche has to edit, so what is
    // asserted instead is the property the test is named for: every blocker names
    // its criterion AND quantifies it.
    for (const blocker of spanish.blockers) {
      expect(blocker.detail.trim()).not.toBe("");
      expect(blocker.shortfall).toBeGreaterThan(0);
    }

    // The criterion counts vocabulary AT OR BELOW the level, not the whole track.
    // Spanish teaches 135 headwords in total but only 46 at or below pre-A1, so the
    // shortfall is 254. Measuring the whole track against a per-level target
    // was the first version of this module committing the very error it exists to
    // catch — a number meaning "everything taught" published against one meaning
    // "by the end of pre-A1".
    expect(vocab.shortfall).toBeLessThanOrEqual(131); // spanish pre-A1 survival tranche: +15 lessons, +3 chapters (chapters 303-305) -- 154/300 to 169/300, a drop of exactly the lesson count // -1: quien now has its own pre-A1 voice-first lesson // -2 more: the repair kit adds two pre-A1 headwords // +1: HL-C98 // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295) // spanish pre-A1 round 3: +35 lessons, +7 chapters (chapters 296-302) -- 118/300 to 153/300, a drop of exactly the lesson count // FLOOR — content only grows; exact pins serialize parallel tranches
    // Chapter 16 reclassifies two paradigm bundles as grammar and adds ver;
    // Chapter 17 correctly reclassifies its two tense bundles as grammar.
    // Chapter 18 likewise replaces two word/phrase bundles with typed grammar.
    expect(spanish.vocabulary).toBeGreaterThanOrEqual(347); // spanish pre-A1 survival tranche: +15, and exactly +15 -- an earlier draft moved this only +12 because cansado, mal and como-se-dice already existed elsewhere in the track; they were replaced with tranquilo, triste and que-significa rather than shipped as duplicates // +2 more: the repair kit // -1: HL-C98 retires the bundled AR-PRESENT-SINGULAR headword // HL-C152: +5 lessons, +1 chapter — Spanish realizes SPINE-NEGATE-AND-ASK, completing A2 at 5/5 // HL-C157: ayer + hablare close A2 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C175: +5 -- chapter 272, reading between the lines // HL-C177: +5 -- chapter 273, C1 closes // HL-C178: +5 -- chapter 274, C2 opens // HL-C179: +5 -- chapter 275, fine shades // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C194: +16 Spanish pre-A1 words // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295) // spanish pre-A1 round 3: +35 lessons, +7 chapters (chapters 296-302) // FLOOR — content only grows; exact pins serialize parallel tranches
    expect(vocab.shortfall).toBeGreaterThan(LEVEL_VOCABULARY["pre-A1"] - spanish.vocabulary);
  });

  it("scopes the atom budget to the level, so a high lesson cannot block a low one", () => {
    // Hindi has one over-budget lesson, and it sits ABOVE pre-A1. Before the criteria
    // were level-scoped it blocked pre-A1 anyway, which made criterion 3 unfalsifiable
    // at the bottom of the ladder for every track.
    const gate = realReport().levelGate!;
    const hindi = gate.tracks.find((t) => t.language === "hindi")!;
    expect(hindi.inProgressAt).toBe("pre-A1");
    expect(hindi.blockers.map((b) => b.criterion)).not.toContain("atom-budget");
  });

  it("fails an authored-but-unrealized level on a COUNT, not on absence", () => {
    // spine.json used to have zero B1-C2 nodes, and the gate refused those levels on
    // the grounds that "no node is unrealized" is not "every node is realized".
    // The tranche is authored now, so the refusal has a better reason: 17 nodes exist
    // and none is realized by any track. The failure names a number instead of a void.
    const e = loadEverything();
    const gate = runLevelGate({
      lessons: e.lessons,
      levels: summarizeLevels(e.lessons, e.curricula, e.spine),
      curricula: e.curricula,
      spine: e.spine,
      ramp: measureRamp(e.lessons, loadChapterPolicy()),
      continuity: measureContinuity(e.lessons),
    });
    const authored = e.spine.nodes.filter((n) => n.stage === "B1");
    expect(authored.length).toBeGreaterThan(0);
    // Authoring a rung does not climb it. No track may attain B1 on an empty ledger.
    expect(gate.tracks.every((t) => t.attained === null)).toBe(true);
    // And the whole ladder is now reachable in principle: every CEFR level has nodes.
    const stages = new Set(e.spine.nodes.map((n) => n.stage));
    for (const level of ["B1", "B2", "C1", "C2"]) expect(stages.has(level)).toBe(true);
  });

  it("stops at the FIRST failing level, because the criteria are cumulative", () => {
    const gate = realReport().levelGate!;
    for (const track of gate.tracks) {
      // You cannot be in progress at two levels at once, and a level above a failing
      // one is unreachable by definition.
      if (track.inProgressAt !== null) expect(track.blockers.length).toBeGreaterThan(0);
      if (track.attained === null) expect(track.inProgressAt).toBe("pre-A1");
    }
  });
});

describe("etymology is a hook, not a skill", () => {
  it("waives etymology atoms from the reinforcement criterion, and says how many", () => {
    // The owner's decision: an etymology is read once, not drilled. Before this the
    // gate demanded every atom be revisited twice, and the only way to satisfy that
    // for an etymon was to re-state it in the Guided Practice and again in the
    // Wrap-up Recall -- so the GATE was manufacturing the repetition.
    //
    // This test used to read Spanish and pin `reinforcement.shortfall` to 24. Spanish
    // no longer HAS a reinforcement blocker -- closing all 24 is what the pre-A1
    // reinforcement tranche did -- so the subject moved, and with it the shape of the
    // assertion. A pinned integer on one track's unfinished work serialises every
    // parallel authoring tranche behind this file, and it names a number that is
    // supposed to fall to zero. What is checked now is the waiver itself.
    const gate = realReport().levelGate!;
    const waiving = gate.tracks.filter((track) =>
      track.blockers.some(
        (b) => b.criterion === "reinforcement" && /etymology hook\(s\) waived/.test(b.detail),
      ),
    );
    // The waiver must be VISIBLE. A silently loosened gate is worse than a strict one.
    expect(waiving.length).toBeGreaterThan(0);
    for (const track of waiving) {
      const reinforcement = track.blockers.find((b) => b.criterion === "reinforcement")!;
      expect(reinforcement.detail).toContain(`atom(s) at or below ${track.inProgressAt} are rev`);
    }

    // And it must BITE. Two earlier versions of this assertion did not: `shortfall <
    // shortfall + waived` is true of any number, and comparing against a whole-track
    // count could not fail because the gate scopes to a level. Both passed with the
    // waiver deleted.
    //
    // So the bite is a COUNTERFACTUAL rather than a constant. Run the same gate over a
    // continuity report whose etymology atoms have been renamed out of the convention
    // `isEtymologyAtom` matches. With the waiver in place those atoms stop being
    // waived and some track's reinforcement shortfall must rise; with the waiver
    // deleted the rename changes nothing at all and this fails. No corpus figure is
    // pinned, so no authoring tranche has to edit it.
    const e = loadEverything();
    const base = measureContinuity(e.lessons);
    const gateInputs = {
      lessons: e.lessons,
      levels: summarizeLevels(e.lessons, e.curricula, e.spine),
      curricula: e.curricula,
      spine: e.spine,
      ramp: measureRamp(e.lessons, loadChapterPolicy()),
    };
    const renamed: ContinuityReport = {
      ...base,
      reinforcement: base.reinforcement.map((defect) =>
        isEtymologyAtom(defect.atom)
          ? { ...defect, atom: defect.atom.replace("-ETYMON-", "-HOOK-") }
          : defect,
      ),
    };
    const shortfalls = (report: ContinuityReport) =>
      new Map(
        runLevelGate({ ...gateInputs, continuity: report }).tracks.map((track) => [
          track.language,
          track.blockers.find((b) => b.criterion === "reinforcement")?.shortfall ?? 0,
        ]),
      );
    const waived = shortfalls(base);
    const unwaived = shortfalls(renamed);
    let bit = false;
    for (const [language, shortfall] of waived) {
      // The waiver can only ever REMOVE failures, never add them.
      expect(unwaived.get(language)!).toBeGreaterThanOrEqual(shortfall);
      if (unwaived.get(language)! > shortfall) bit = true;
    }
    expect(bit).toBe(true);
  });

  it("leaves continuity's own numbers alone", () => {
    // The waiver lives at the GATE on purpose. `measureContinuity` still reports every
    // atom, so the gap report and every pinned corpus figure keep meaning what they
    // say. If this ever fails, the waiver has leaked into the measurement.
    const e = loadEverything();
    const continuity = measureContinuity(e.lessons);
    const etymons = continuity.reinforcement.filter((d) => isEtymologyAtom(d.atom));
    expect(etymons.length).toBeGreaterThan(0);
  });

  it("matches the id convention and nothing else", () => {
    expect(isEtymologyAtom("ES-ETYMON-CREDERE-02")).toBe(true);
    expect(isEtymologyAtom("TA-ETYMON-NAL-01")).toBe(true);
    // A skill atom that merely mentions a language or a sound is NOT waived.
    expect(isEtymologyAtom("ES-LEX-DE-PIE-11")).toBe(false);
    expect(isEtymologyAtom("SA-SOUND-PIE-KW-OUTCOMES")).toBe(false);
    expect(isEtymologyAtom("ES-GRAMMAR-AGREEMENT-MASCULINE-PLURAL")).toBe(false);
  });
});

describe("the criteria themselves", () => {
  it("passes a level only when all four hold", () => {
    // A synthetic track that satisfies nothing must attain nothing, and must name
    // every criterion it failed rather than the first one.
    const e = loadEverything();
    const levels = summarizeLevels(e.lessons, e.curricula, e.spine);
    const ramp = measureRamp(e.lessons, loadChapterPolicy());
    const continuity = measureContinuity(e.lessons);
    const gate = runLevelGate({
      lessons: e.lessons,
      levels,
      curricula: e.curricula,
      spine: e.spine,
      ramp,
      continuity,
    });
    const worst = gate.tracks.find((t) => t.blockers.length >= 3);
    expect(worst).toBeDefined();
    expect(new Set(worst!.blockers.map((b) => b.criterion)).size).toBe(
      worst!.blockers.length,
    );
  });

  it("cannot attain an otherwise complete level while its required writing stage is unproved", () => {
    const lessons = Array.from({ length: LEVEL_VOCABULARY["pre-A1"] }, (_, index) =>
      parseLesson(`---
id: AA-${String(index + 1).padStart(3, "0")}
chapter: 1
type: word
headword: word-${index + 1}
gloss: word ${index + 1}
concept_tag: AA-WORD-${index + 1}
---

word ${index + 1}
`, "alpha"),
    );
    const curriculum = [{
      version: 1,
      language: "alpha",
      path: [{
        id: "alpha-pre-a1",
        spine_node: "PRE",
        lessons: lessons.map((lesson) => lesson.realization.lessonId),
        before: [],
        inline: [],
        after: [],
      }],
      spine: { PRE: { segments: ["alpha-pre-a1"], omits: [], relocates: {} } },
      extensions: [],
    }];
    const spine = {
      version: 1,
      stages: ["pre-A1", "A1", "A2", "B1", "B2", "C1", "C2"],
      nodes: [{
        id: "PRE",
        stage: "pre-A1",
        strand: "FUNCTION",
        canDo: "write a word",
        prerequisites: [],
        core: true,
        concepts: [],
      }],
    };
    const levels = summarizeLevels(lessons, curriculum, spine);
    const ramp = measureRamp(lessons, loadChapterPolicy());
    const continuity = { reinforcement: [] } as unknown as ContinuityReport;
    const withoutWritingGate = runLevelGate({ lessons, levels, curricula: curriculum, spine, ramp, continuity });
    expect(withoutWritingGate.tracks[0]?.attained).toBe("pre-A1");

    const writingStages = {
      stages: [{ id: "observe-trace", firstRequiredAt: "pre-A1", prerequisites: [] }],
      tracks: [{
        language: "alpha",
        evidence: [],
        validEvidence: [],
        defects: [],
        levels: [{
          level: "pre-A1",
          requiredStages: ["observe-trace"],
          evidencedStages: [],
          missingStages: ["observe-trace"],
          complete: false,
        }],
      }],
      summary: {
        tracks: 1,
        tracksWithAnyEvidence: 0,
        tracksCompleteAtPreA1: 0,
        evidenceBlocks: 0,
        invalidEvidenceBlocks: 0,
        missingTrackLevelStages: 1,
      },
    } satisfies WritingStageReport;
    const gated = runLevelGate({
      lessons,
      levels,
      curricula: curriculum,
      spine,
      ramp,
      continuity,
      writingStages,
    });
    expect(gated.tracks[0]).toMatchObject({
      attained: null,
      inProgressAt: "pre-A1",
      blockers: [{ criterion: "writing-stage", shortfall: 1 }],
    });
  });

  it("keeps the vocabulary targets ascending, since they are cumulative", () => {
    const values = Object.values(LEVEL_VOCABULARY);
    for (let i = 1; i < values.length; i += 1) {
      expect(values[i]!).toBeGreaterThan(values[i - 1]!);
    }
  });

  it("is absent, not empty, when the caller supplied no policy", () => {
    // "Not measured" and "attained nothing" are opposite facts. A consumer that
    // passes no chapter policy must not see a report claiming zero attainment.
    const e = loadEverything();
    const report = buildCurriculumGapReport({
      registry: e.registry,
      lessons: e.lessons,
      books: e.books,
      curricula: e.curricula,
      spine: e.spine,
    });
    expect(report.levelGate).toBeUndefined();
  });
});
