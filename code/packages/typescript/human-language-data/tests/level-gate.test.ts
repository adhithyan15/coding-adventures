// HL09 §3.1 — what it takes to CLAIM a level. See src/level-gate.ts for why.

import { describe, expect, it } from "vitest";
import { loadEverything, loadChapterPolicy, loadTrackChapters } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";
import { buildCurriculumGapReport, renderCurriculumGapReport } from "../src/report.js";
import {
  LEVEL_VOCABULARY,
  LEVEL_VERB_VOCABULARY,
  runLevelGate,
  isEtymologyAtom,
} from "../src/level-gate.js";
import { summarizeLevels, levelRank, lessonSpineNodes, CEFR_LEVELS } from "../src/levels.js";
import type { CefrLevel } from "../src/levels.js";
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
    // The number that does not, and the first time it has ever moved. Spanish has
    // now met all five pre-A1 criteria — 304 headwords at or below the level, every
    // pre-A1 spine node realized, no lesson over the atom budget, every pre-A1 atom
    // revisited twice, and the writing stages proved — so `attained` reads pre-A1
    // and the work in progress is A1.
    //
    // The distinction this test is named for is UNCHANGED by that, and the gap is
    // still six rungs wide: `touches` says C2 because three lessons point at a C2
    // node; `attained` says pre-A1 because that is the highest rung whose criteria
    // actually hold. Asserting `attained === null` was only ever a proxy for "the
    // two numbers disagree" — it happened to be the value at the time. What is
    // asserted now is the disagreement itself, which is the claim, plus the floor
    // the track has genuinely reached.
    expect(spanish.attained).toBe("pre-A1");
    expect(spanish.inProgressAt).toBe("A1");
    expect(levelRank(spanish.touches!)).toBeGreaterThan(levelRank(spanish.attained!));
  });

  it("reports every track as overstating, which is the finding", () => {
    // All 23 tracks touch a level ABOVE the one they have attained. That is not 23
    // bugs — it is one measurement having been read as another for the whole project.
    const gate = realReport().levelGate!;
    expect(gate.summary.tracksOverstating).toBe(23);
    // `tracksWithAnyLevel` was pinned at 0, and it stayed 0 for every track from the
    // day this gate was written. It is 1 now: Spanish closed the last of its pre-A1
    // criteria. The finding this test names is unaffected — overstating is a track
    // touching HIGHER than it has attained, not a track having attained nothing —
    // so the 23 stands and the zero does not.
    expect(gate.summary.tracksWithAnyLevel).toBe(1);
    // Which rung, and only that rung. Pinning "pre-A1 is 1" alone would pass on a
    // gate that had also handed out a spurious C2, so every level is still checked;
    // the exception is named rather than the assertion dropped.
    for (const [level, count] of Object.entries(gate.summary.attainedByLevel)) {
      expect(count).toBe(level === "pre-A1" ? 1 : 0);
    }
    // And the count agrees with the tracks it is a count OF — the summary is derived
    // from `tracks`, so a summary that drifts from it is the bug this would catch.
    expect(gate.tracks.filter((t) => t.attained !== null).map((t) => t.language)).toEqual([
      "spanish",
    ]);
  });

  it("names which criterion failed and by how much, not just that one did", () => {
    const gate = realReport().levelGate!;
    const spanish = gate.tracks.find((t) => t.language === "spanish")!;

    // This asserted, in turn, Spanish's `vocabulary` blocker and its shortfall, and
    // then — once chapters 328-334 carried the track past the 300-word floor — the
    // ABSENCE of that blocker from `spanish.blockers`. Both readings were of a list
    // scoped to whatever rung Spanish happened to be working on, and that rung has
    // now moved: `blockers` describes A1, where a 600-word target is open again and
    // `vocabulary` is legitimately back in the list. Asserting its absence would now
    // be asserting something false about a different level.
    //
    // The closure is stated where it cannot drift: `attained` is the gate's own
    // verdict that EVERY pre-A1 criterion holds, which is strictly more than the
    // one criterion the old line reached for.
    expect(levelRank(spanish.attained!)).toBeGreaterThanOrEqual(levelRank("pre-A1"));
    // Anti-vacuity. The loop below asserts over `spanish.blockers`, and a `for` over
    // an empty array passes without checking anything — so a Spanish that had gone
    // clean at every level, or a gate that had stopped populating blockers at all,
    // would turn this test green while measuring nothing. Something must be open for
    // the loop to have a subject.
    expect(spanish.blockers.length).toBeGreaterThan(0);
    expect(spanish.inProgressAt).not.toBeNull();
    // This once asserted a `reinforcement` blocker, then asserted its absence
    // after the reinforcement tranche closed all 24 under-reinforced atoms, and
    // now it would have to assert its presence again — criterion 4 re-opens
    // every time a vocabulary wave lands and closes as the new atoms get their
    // second outing. That churn is the argument: pinning WHICH criteria are open
    // makes this test a ledger of unfinished work that every tranche has to
    // edit. What is asserted instead is the property the test is named for —
    // every blocker names its criterion AND quantifies it.
    for (const blocker of spanish.blockers) {
      expect(blocker.detail.trim()).not.toBe("");
      expect(blocker.shortfall).toBeGreaterThan(0);
    }

    // The criterion counts vocabulary AT OR BELOW the level, not the whole track.
    // Measuring the whole track against a per-level target was the first version
    // of this module committing the very error it exists to catch — a number
    // meaning "everything taught" published against one meaning "by the end of
    // pre-A1". That distinction is what the closure above rests on: the track
    // teaches 482 headwords in total, and it is the 304 taught at or below
    // pre-A1 that cleared the 300-word floor. The shortfall CEILING that stood
    // here is retired along with the blocker it measured — git holds its history.
    // Chapter 16 reclassifies two paradigm bundles as grammar and adds ver;
    // Chapter 17 correctly reclassifies its two tense bundles as grammar.
    // Chapter 18 likewise replaces two word/phrase bundles with typed grammar.
    expect(spanish.vocabulary).toBeGreaterThanOrEqual(347); // spanish pre-A1 survival tranche: +15, and exactly +15 -- an earlier draft moved this only +12 because cansado, mal and como-se-dice already existed elsewhere in the track; they were replaced with tranquilo, triste and que-significa rather than shipped as duplicates // +2 more: the repair kit // -1: HL-C98 retires the bundled AR-PRESENT-SINGULAR headword // HL-C152: +5 lessons, +1 chapter — Spanish realizes SPINE-NEGATE-AND-ASK, completing A2 at 5/5 // HL-C157: ayer + hablare close A2 // HL-C158: +4 -- the B1 travel rung (chapter 268) // HL-C159: +4 -- the B1 describe-experience rung (chapter 269) // HL-C160: +1 -- depende closes SPINE-EXPRESS-CONDITION, and B1 // HL-C172: +4 -- the B2 argue rung (chapter 270) // HL-C173: +2 -- B2 closes (chapter 271) // HL-C175: +5 -- chapter 272, reading between the lines // HL-C177: +5 -- chapter 273, C1 closes // HL-C178: +5 -- chapter 274, C2 opens // HL-C179: +5 -- chapter 275, fine shades // HL-C180: +4 -- chapter 276; ARCHAIC-FORM was already taught at chapter 3 // HL-C181: +5 -- chapter 277, the spine closes at 33/33 // HL-C194: +16 Spanish pre-A1 words // spanish pre-A1 tranche: +35 lessons, +7 chapters (chapters 282-288) // spanish pre-A1 round 2: +35 lessons, +7 chapters (chapters 289-295) // spanish pre-A1 round 3: +35 lessons, +7 chapters (chapters 296-302) // FLOOR — content only grows; exact pins serialize parallel tranches
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
    //
    // This read `every(t => t.attained === null)`, which was a stronger claim than the
    // sentence above it and stopped being true when Spanish attained pre-A1. pre-A1 is
    // not the rung under test — B1 is, and the reason B1 is refused is that all 17 of
    // its nodes are unrealized, which has nothing to do with what the bottom of the
    // ladder has closed. Scoping the assertion to B1-and-above says exactly the thing
    // the test is named for, and still fails the day a track is handed a rung whose
    // ledger is empty.
    for (const track of gate.tracks) {
      if (track.attained !== null) {
        expect(levelRank(track.attained)).toBeLessThan(levelRank("B1"));
      }
    }
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

describe("the first rung anybody actually climbed", () => {
  // Everything below is about ONE track reaching ONE level. It is worth its own
  // describe because for the whole life of this module the answer was "none", and a
  // regression to "none" is the kind of thing a corpus-wide suite reports as a
  // changed integer somewhere rather than as the thing it is.

  it("closes criterion 4 for Spanish at the atom, not just at the summary", () => {
    // `attained === "pre-A1"` is the gate's own verdict, and asserting only the
    // verdict would pass on a gate that had quietly stopped measuring. So this
    // re-derives criterion 4 from the same two inputs the gate reads — the continuity
    // report and the lesson→level map — and names any atom that fails it.
    const e = loadEverything();
    const continuity = measureContinuity(e.lessons);
    const spineNodes = lessonSpineNodes(e.curricula);
    const stageOf = new Map<string, CefrLevel>();
    for (const node of e.spine.nodes) {
      if ((CEFR_LEVELS as readonly string[]).includes(node.stage)) {
        stageOf.set(node.id, node.stage as CefrLevel);
      }
    }
    const levelOf = new Map<string, CefrLevel>();
    for (const l of e.lessons) {
      const node = spineNodes.get(l.realization.lessonId);
      const stage = node ? stageOf.get(node) : undefined;
      if (stage) levelOf.set(l.realization.lessonId, stage);
    }

    const thin = continuity.reinforcement.filter((defect) => {
      if (defect.language !== "spanish") return false;
      const level = levelOf.get(defect.introducedBy);
      if (level === undefined || levelRank(level) > levelRank("pre-A1")) return false;
      return defect.revisits < 2 && !isEtymologyAtom(defect.atom);
    });
    // Named, not counted: if this ever regresses, the failure should say which atoms
    // went thin rather than making the next reader re-run the query by hand.
    expect(thin.map((d) => `${d.atom} (${d.introducedBy})`)).toEqual([]);

    // Anti-vacuity for the filter above. If `levelOf` were empty — a plausible
    // refactoring accident, since it is rebuilt here rather than exported — every
    // defect would be skipped and the assertion would pass having checked nothing.
    const scoped = continuity.reinforcement.filter(
      (d) => d.language === "spanish" && levelRank(levelOf.get(d.introducedBy) ?? "C2") <= levelRank("pre-A1"),
    );
    expect(scoped.length).toBeGreaterThan(0);
  });

  it("says WHICH track reached it, because a bare count cannot be checked", () => {
    // The one line this whole apparatus was built to print. It read
    // "levels ATTAINED (HL09 §3.1): none" from the day it was written; now it names a
    // track. Pinning the sentence rather than the count is deliberate: a count is
    // satisfied by any track at all, and the claim being made is about Spanish.
    const rendered = renderCurriculumGapReport(realReport());
    const line = rendered.split(/\r?\n/).find((l) => l.startsWith("levels ATTAINED"))!;
    expect(line).toBeDefined();
    expect(line).not.toContain("none");
    expect(line).toContain("1 track at pre-A1 (spanish)");
    // And the plural agrees with the count, which nothing could have caught while the
    // populated branch of this line had never once run.
    expect(line).not.toContain("1 tracks");
  });

  it("blocks Spanish at A1 on COMPOSITION, while its total is within sixteen", () => {
    // HL23's finding, made into a gate. Spanish teaches 584 of the 600 headwords
    // A1 asks for — close enough that criterion 2 alone would certify A1 after one
    // more tranche — and SEVEN of those headwords are verbs. Criterion 2b is the
    // one that says so, and it must be visibly RED until the lexicon is authored.
    //
    // The shortfall is pinned rather than merely asserted non-zero, because a
    // criterion that fails is not evidence of anything on its own: it has to fail
    // by the amount the corpus actually justifies. This number must FALL as verbs
    // are authored, and every tranche that moves it updates the pin here.
    const spanish = realReport().levelGate!.tracks.find((t) => t.language === "spanish")!;
    expect(spanish.inProgressAt).toBe("A1");
    const verbs = spanish.blockers.find((b) => b.criterion === "verb-vocabulary");
    expect(verbs).toBeDefined();
    expect(verbs!.shortfall).toBe(32);
    expect(verbs!.detail).toContain("8 distinct verb headwords at or below A1");

    // And the criterion it partitions is still short too, by a different amount —
    // proof the two numbers are measuring different things rather than one being a
    // restatement of the other.
    expect(spanish.blockers.find((b) => b.criterion === "vocabulary")?.shortfall).toBe(15);

    // Critically: pre-A1 is NOT perturbed. Spanish is the only track holding any
    // level at all, so a new criterion that revoked it would be a regression
    // dressed as a gate. Six verb headwords at or below pre-A1 against a floor of
    // five — one of margin, and that margin is why the floor is not higher.
    expect(spanish.attained).toBe("pre-A1");
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
    // Each track's WHOLE verdict, not just a shortfall integer. The previous version
    // kept only the number, and the number alone is not comparable between the two
    // runs: a blocker is scoped to the level the track is IN PROGRESS at, so if the
    // waiver lets a track climb a rung, the two runs report shortfalls for two
    // different levels and "unwaived >= waived" stops meaning anything. That is not
    // hypothetical — it is Spanish today. With the waiver it stands at pre-A1 and is
    // working on A1, carrying 88 under-reinforced A1 atoms; with the etymons renamed
    // it never leaves pre-A1 and carries 27 there. 27 < 88 with no waiver bug in
    // sight, and the old assertion failed on it.
    const verdicts = (report: ContinuityReport) =>
      new Map(
        runLevelGate({ ...gateInputs, continuity: report }).tracks.map((track) => [
          track.language,
          {
            attained: track.attained,
            inProgressAt: track.inProgressAt,
            shortfall: track.blockers.find((b) => b.criterion === "reinforcement")?.shortfall ?? 0,
          },
        ]),
      );
    const waived = verdicts(base);
    const unwaived = verdicts(renamed);
    // `attained` is a level or null; null is "below every rung", hence -1.
    const rank = (level: string | null) => (level === null ? -1 : levelRank(level as CefrLevel));
    let bit = false;
    for (const [language, withWaiver] of waived) {
      const without = unwaived.get(language)!;
      // The general invariant, true at every level: the waiver can only ever REMOVE
      // failures, so a track can stand at the same rung with it as without, or higher
      // — never lower.
      expect(rank(without.attained)).toBeLessThanOrEqual(rank(withWaiver.attained));
      if (without.inProgressAt === withWaiver.inProgressAt) {
        // Same rung in both runs, so the two shortfalls are commensurable and the
        // original comparison is exactly right.
        expect(without.shortfall).toBeGreaterThanOrEqual(withWaiver.shortfall);
        if (without.shortfall > withWaiver.shortfall) bit = true;
      } else {
        // Different rungs, which can only have happened because the waiver carried
        // this track over one. That is the strongest bite available: the rename did
        // not merely raise a count, it cost the track a level.
        bit = true;
      }
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
    // This fixture exists to isolate ONE criterion, so it has to satisfy every
    // other one — otherwise the assertion below passes or fails for a reason the
    // test is not about. The first `LEVEL_VERB_VOCABULARY["pre-A1"]` lessons are
    // therefore verb-tagged, which is what the composition criterion (2b) asks
    // for; the rest stay nouns. Headwords are distinct either way, so the total
    // is still exactly `LEVEL_VOCABULARY["pre-A1"]`.
    const lessons = Array.from({ length: LEVEL_VOCABULARY["pre-A1"] }, (_, index) =>
      parseLesson(`---
id: AA-${String(index + 1).padStart(3, "0")}
chapter: 1
type: word
headword: word-${index + 1}
gloss: word ${index + 1}
concept_tag: ${index < LEVEL_VERB_VOCABULARY["pre-A1"] ? "AA-VERB" : "AA-WORD"}-${index + 1}
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

  it("keeps the verb targets ascending, and below the totals they partition", () => {
    // Two properties, and the second is the one worth having. A verb floor above
    // the vocabulary target it is a subset of would be unsatisfiable by any corpus
    // — the gate would fail every track forever and nobody could tell that from a
    // gate working correctly on a corpus nobody had authored yet.
    const verbs = Object.values(LEVEL_VERB_VOCABULARY);
    for (let i = 1; i < verbs.length; i += 1) {
      expect(verbs[i]!).toBeGreaterThan(verbs[i - 1]!);
    }
    for (const level of CEFR_LEVELS) {
      expect(LEVEL_VERB_VOCABULARY[level]).toBeLessThan(LEVEL_VOCABULARY[level]);
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
