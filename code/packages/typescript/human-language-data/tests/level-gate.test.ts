// HL09 §3.1 — what it takes to CLAIM a level. See src/level-gate.ts for why.

import { describe, expect, it } from "vitest";
import { loadEverything, loadChapterPolicy, loadTrackChapters } from "../src/loader.js";
import { buildCurriculumGapReport } from "../src/report.js";
import { LEVEL_VOCABULARY, runLevelGate, isEtymologyAtom } from "../src/level-gate.js";
import { summarizeLevels } from "../src/levels.js";
import { measureRamp } from "../src/ramp.js";
import { measureContinuity } from "../src/continuity.js";

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
    expect(spanish.touches).toBe("C1");
    // The number that does not: Spanish has not met even pre-A1's criteria.
    expect(spanish.attained).toBeNull();
    expect(spanish.inProgressAt).toBe("pre-A1");
  });

  it("reports every track as overstating, which is the finding", () => {
    // All 22 tracks touch a level none of them has attained. That is not 22 bugs —
    // it is one measurement having been read as another for the whole project.
    const gate = realReport().levelGate!;
    expect(gate.summary.tracksOverstating).toBe(22);
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
    expect(spanish.blockers.map((b) => b.criterion)).toContain("reinforcement");

    // The criterion counts vocabulary AT OR BELOW the level, not the whole track.
    // Spanish teaches 135 headwords in total but only 46 at or below pre-A1, so the
    // shortfall is 254. Measuring the whole track against a per-level target
    // was the first version of this module committing the very error it exists to
    // catch — a number meaning "everything taught" published against one meaning
    // "by the end of pre-A1".
    expect(vocab.shortfall).toBe(252); // -2 more: the repair kit adds two pre-A1 headwords // +1: HL-C98
    // Chapter 16 reclassifies two paradigm bundles as grammar and adds ver;
    // Chapter 17 correctly reclassifies its two tense bundles as grammar.
    // Chapter 18 likewise replaces two word/phrase bundles with typed grammar.
    expect(spanish.vocabulary).toBe(193); // +2 more: the repair kit // -1: HL-C98 retires the bundled AR-PRESENT-SINGULAR headword // HL-C152: +5 lessons, +1 chapter — Spanish realizes SPINE-NEGATE-AND-ASK, completing A2 at 5/5 // HL-C157: ayer + hablare close A2 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C175: +5 -- chapter 272, reading between the lines
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
    const gate = realReport().levelGate!;
    const spanish = gate.tracks.find((t) => t.language === "spanish")!;
    const reinforcement = spanish.blockers.find((b) => b.criterion === "reinforcement")!;

    // The waiver must be VISIBLE. A silently loosened gate is worse than a strict one.
    expect(reinforcement.detail).toMatch(/etymology hook\(s\) waived/);
    // And it must BITE, which needs a number the waiver actually changes. Two earlier
    // versions of this assertion did not: `shortfall < shortfall + waived` is true of
    // any number, and comparing against a whole-track count could not fail because the
    // gate scopes to pre-A1. Both passed with the waiver deleted. So pin the figure:
    // Chapter 9 revisits its origin atoms in both the location contrast and terminal
    // checkpoint, so the pre-A1 shortfall falls from 49 to 48. Chapter 10 revisits
    // another older atom and lowers the shortfall again. Its etymon atoms sit above
    // pre-A1, so this level-specific waiver count correctly stays unchanged.
    expect(reinforcement.shortfall).toBe(24); // -1: HL-C98 closes a reinforcement gap // -1: HL-C113 -- ES-C55-si revisits ES-LEX-SI-01 and ES-GRAMMAR-DIACRITIC-ACCENT from pre-A1
    expect(reinforcement.detail).toContain("atom(s) at or below pre-A1 are rev"); // +2: the repair kit's two etymons // -1: HL-C98
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
