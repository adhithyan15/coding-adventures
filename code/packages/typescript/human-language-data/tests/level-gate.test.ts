// HL09 §3.1 — what it takes to CLAIM a level. See src/level-gate.ts for why.

import { describe, expect, it, vi } from "vitest";
import { fileURLToPath } from "node:url";
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
// `vocabularyOf` / `verbVocabularyOf` are module-private in level-gate.ts, so the
// criterion-2b test below mirrors them; CONTENT_TYPES is the filter they share.
import { CONTENT_TYPES } from "../src/constants.js";
import { measureContinuity } from "../src/continuity.js";
import type { ContinuityReport } from "../src/continuity.js";
import type { WritingStageReport } from "../src/writing-stages.js";
import {
  assertLevelProgressOwners,
  loadLevelProgressOwners,
} from "./level-gate-attainment-pins.js";

// Several assertions intentionally rebuild or remeasure the full curriculum.
// The corpus keeps growing, so give this integration-heavy file one explicit
// budget instead of chasing whichever assertion next crosses Vitest's default.
vi.setConfig({ testTimeout: 60_000 });

const ATTAINMENT_DIR = fileURLToPath(new URL("./level-gate-attainment/", import.meta.url));
const REAL_EVERYTHING = loadEverything();
const LEVEL_PROGRESS = loadLevelProgressOwners(
  ATTAINMENT_DIR,
  REAL_EVERYTHING.registry.languages.map((language) => language.id),
);

// Built ONCE for the file, not once per test. The gap report now walks continuity
// (~900ms) and the level gate on top of everything else, so rebuilding an identical
// report five times cost ~18s and timed out the 5s default on CI. Memoising is also
// simply correct: every test wants the same report.
//
// And built at IMPORT, not lazily inside the first test that asks. A lazy memo still
// charges the whole load-and-build to that one test's budget: "separates what a
// track TOUCHES" took 22.8s locally with ~20s of it this build, one heavy CI runner
// away from its 30s limit. Import time carries no per-test budget.
// The parsed corpus itself is kept too, so a test that runs the gate on its own
// inputs reads this parse instead of paying for another one in its body.
const REAL_REPORT = (() => {
  const e = REAL_EVERYTHING;
  return buildCurriculumGapReport({
    registry: e.registry,
    lessons: e.lessons,
    books: e.books,
    curricula: e.curricula,
    spine: e.spine,
    trackChapters: loadTrackChapters(),
    chapterPolicy: loadChapterPolicy(),
  });
})();
function realReport() {
  return REAL_REPORT;
}

// The atom-budget witness, built synthetically and at IMPORT for the same reason
// REAL_REPORT is. The claim it carries is "a track can attain pre-A1 while an A1
// lesson is over budget", and the corpus stopped holding a real example when the
// final witness attained A1. So take a track that attains A1 on the real
// corpus, mark one of its A1 lessons over budget in the ramp report, and run the
// gate again on otherwise identical inputs.
const SYNTHETIC_WITNESS = (() => {
  const e = REAL_EVERYTHING;
  const inputs = {
    lessons: e.lessons,
    levels: summarizeLevels(e.lessons, e.curricula, e.spine),
    curricula: e.curricula,
    spine: e.spine,
    ramp: measureRamp(e.lessons, loadChapterPolicy()),
    continuity: measureContinuity(e.lessons),
  };
  const baseline = runLevelGate(inputs);
  const track = baseline.tracks
    .filter((t) => t.attained === "A1")
    .map((t) => t.language)
    .sort()[0]!;
  const stageOf = new Map(e.spine.nodes.map((node) => [node.id, node.stage]));
  const nodeOf = lessonSpineNodes(e.curricula);
  const lesson = e.lessons
    .filter((l) => l.language === track)
    .map((l) => l.realization.lessonId)
    .sort()
    .find((id) => stageOf.get(nodeOf.get(id) ?? "") === "A1")!;
  const overBudget = { lessonId: lesson, language: track, chapter: null, atoms: 4, budget: 3 };
  const gate = runLevelGate({ ...inputs, ramp: { ...inputs.ramp, lessons: [...inputs.ramp.lessons, overBudget] } });
  return { track, lesson, baseline: baseline.tracks.find((t) => t.language === track)!, gate };
})();

describe("the gate that would have caught the A2 claim", () => {
  // Explicit budget: `realReport()` builds the entire gap report over the whole corpus.
  // At 1,313 lessons that runs past vitest's 5,000 ms default under full-suite parallel
  // load, while passing in isolation — so a per-file run will not reproduce it.
  it("separates what a track TOUCHES from what it has ATTAINED", { timeout: 60_000 }, () => {
    const gate = realReport().levelGate!;
    const separated = gate.tracks.filter(
      (track) =>
        track.touches !== null &&
        (track.attained === null || levelRank(track.touches) > levelRank(track.attained)),
    );
    // Anti-vacuity: the corpus must exercise the distinction, while the direct
    // owners below carry every mutable per-track value.
    expect(separated.length).toBeGreaterThan(0);
    for (const track of separated) {
      if (track.attained !== null) {
        expect(levelRank(track.touches!)).toBeGreaterThan(levelRank(track.attained));
      }
    }
  });

  it("reports every track as overstating, which is the finding", () => {
    // All 23 tracks touch a level ABOVE the one they have attained. That is not 23
    // bugs — it is one measurement having been read as another for the whole project.
    const gate = realReport().levelGate!;
    expect(gate.summary.tracksOverstating).toBe(
      gate.tracks.filter(
        (track) =>
          track.touches !== null &&
          (track.attained === null || levelRank(track.touches) > levelRank(track.attained)),
      ).length,
    );
    // Track-specific verdicts and provenance live in direct owners. This shared
    // test states only cross-track relations and derives every count from those
    // owners, so advancing one language never edits this file.
    const measured = Object.fromEntries(
      gate.tracks.map((track) => [
        track.language,
        { attained: track.attained, reach: track.touches },
      ]),
    );
    assertLevelProgressOwners(LEVEL_PROGRESS, measured);
    expect(gate.summary.tracksWithAnyLevel).toBe(
      Object.values(LEVEL_PROGRESS).filter((owner) => owner.attained !== null).length,
    );
    const expectedByLevel = Object.fromEntries(CEFR_LEVELS.map((level) => [level, 0]));
    for (const owner of Object.values(LEVEL_PROGRESS)) {
      if (owner.attained !== null) {
        expectedByLevel[owner.attained] += 1;
      }
    }
    expect(gate.summary.attainedByLevel).toEqual(expectedByLevel);
  });

  it("names which criterion failed and by how much, not just that one did", () => {
    const gate = realReport().levelGate!;
    const open = gate.tracks.filter((track) => track.inProgressAt !== null);
    expect(open.length).toBeGreaterThan(0);
    for (const track of open) {
      expect(track.blockers.length, track.language).toBeGreaterThan(0);
      for (const blocker of track.blockers) {
        expect(blocker.criterion.trim(), track.language).not.toBe("");
        expect(blocker.detail.trim(), track.language).not.toBe("");
        expect(blocker.shortfall, track.language).toBeGreaterThan(0);
      }
    }
  });

  it("scopes the atom budget to the level, so a high lesson cannot block a low one", () => {
    // Before the criteria were level-scoped, an over-budget lesson above pre-A1
    // blocked pre-A1 anyway. The real corpus eventually stopped carrying a stable
    // witness, so the claim now uses the synthetic fixture built above: one A1
    // lesson of an A1 track marked over budget, everything else the real corpus.
    const { track, lesson, baseline, gate } = SYNTHETIC_WITNESS;
    expect(baseline.attained, `${track} attains A1 before the injection`).toBe("A1");
    const witness = gate.tracks.find((t) => t.language === track)!;
    expect(witness.attained, `${track} with ${lesson} over budget`).toBe("pre-A1");
    expect(witness.inProgressAt).toBe("A1");
    expect(witness.blockers.filter((b) => b.criterion === "atom-budget")).toEqual([
      expect.objectContaining({ shortfall: 1 }),
    ]);
  });

  it("fails an authored-but-unrealized level on a COUNT, not on absence", () => {
    // spine.json used to have zero B1-C2 nodes, and the gate refused those levels on
    // the grounds that "no node is unrealized" is not "every node is realized".
    // The tranche is authored now, so the refusal has a better reason: 17 nodes exist,
    // and a track that has not realized them is refused on that count. The failure
    // names a number instead of a void. (The corpus now exercises the positive case.)
    const e = REAL_EVERYTHING;
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
    // sentence above it and stopped being true when the first track attained pre-A1. pre-A1 is
    // not the rung under test — B1 is, and the reason B1 is refused is that all 17 of
    // its nodes are unrealized, which has nothing to do with what the bottom of the
    // ladder has closed. Scoping the assertion to B1-and-above says exactly the thing
    // the test is named for, and still fails the day a track is handed a rung whose
    // ledger is empty.
    //
    // A track then climbed B1 honestly, so "below B1" stopped
    // being true for every track. The claim the test is named for still holds and is
    // now checked directly rather than by proxy: a track at B1 or above must realize
    // EVERY authored B1 node in its own lessons. A track handed B1 on an empty ledger
    // still fails here, on the first node it does not realize.
    const nodeOf = lessonSpineNodes(e.curricula);
    for (const track of gate.tracks) {
      if (track.attained === null || levelRank(track.attained) < levelRank("B1")) continue;
      const realized = new Set(
        e.lessons
          .filter((l) => l.language === track.language)
          .map((l) => nodeOf.get(l.realization.lessonId))
          .filter((node): node is string => node !== undefined),
      );
      for (const node of authored) expect(realized.has(node.id), `${track.language} ${node.id}`).toBe(true);
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

describe("the rungs tracks have climbed", () => {
  it("closes criterion 4 at the atom for every attained rung", { timeout: 60_000 }, () => {
    // Re-derive criterion 4 from the same two inputs the gate reads. The mutable
    // rung for each track comes from its direct owner; this shared test states only
    // the invariant that every attained rung has no thin non-etymology atoms.
    const e = REAL_EVERYTHING;
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

    let scopedDefects = 0;
    let attainedTracks = 0;
    for (const [language, owner] of Object.entries(LEVEL_PROGRESS)) {
      if (owner.attained === null) continue;
      attainedTracks += 1;
      const scoped = continuity.reinforcement.filter((defect) => {
        const level = levelOf.get(defect.introducedBy);
        return (
          defect.language === language &&
          level !== undefined &&
          levelRank(level) <= levelRank(owner.attained!)
        );
      });
      scopedDefects += scoped.length;
      const thin = scoped.filter((defect) => defect.revisits < 2 && !isEtymologyAtom(defect.atom));
      expect(
        thin.map((defect) => `${defect.atom} (${defect.introducedBy})`),
        language,
      ).toEqual([]);
    }
    expect(attainedTracks).toBeGreaterThan(0);
    expect(scopedDefects).toBeGreaterThan(0);
  });

  it("names every track at every attained rung, because bare counts cannot be checked", () => {
    const rendered = renderCurriculumGapReport(realReport());
    const line = rendered.split(/\r?\n/).find((l) => l.startsWith("levels with STRUCTURAL COVERAGE"))!;
    expect(line).toBeDefined();
    const groups = new Map<CefrLevel, string[]>();
    for (const [language, owner] of Object.entries(LEVEL_PROGRESS)) {
      if (owner.attained === null) continue;
      const peers = groups.get(owner.attained) ?? [];
      peers.push(language);
      groups.set(owner.attained, peers);
    }
    expect(groups.size).toBeGreaterThan(0);
    expect(line).not.toContain("none");
    for (const [level, peers] of groups) {
      peers.sort();
      expect(line).toContain(
        `${peers.length} track${peers.length === 1 ? "" : "s"} at ${level} (${peers.join(", ")})`,
      );
    }
    // And the plural agrees with the count, which nothing could have caught while the
    // populated branch of this line had never once run.
    // Guarded by a no-digit lookbehind: "11 tracks" is correct English and
    // contains the substring "1 tracks", which a plain toContain would flag.
    expect(line).not.toMatch(/(?<!\d)1 tracks/);
  });

  it("prints EVERY blocker of every track, because §3.1 is a conjunction", () => {
    // HL-C427. This block used to render one blocker per track, picked with
    // `sort((a, b) => b.shortfall - a.shortfall)[0]`. Shortfalls are in each
    // criterion's own units and the units differ by two orders of magnitude, so
    // `vocabulary` (headwords, 79-257) beat `verb-vocabulary` (verbs, 1-29) and
    // `atom-budget` (lessons, 1-5) on every track, every time. Those two
    // criteria were not occasionally hidden; they were unprintable, and neither
    // had ever appeared in a plan.
    //
    // The old version of this test asserted criterion groups were contiguous and
    // that shortfall ascended within a group -- both true of a renderer that
    // shows one blocker, and both satisfiable while 41 of the corpus's 64
    // blockers went unmentioned. It even stated the unit rule in its own comment
    // while the code one line from the sort it was guarding broke it.
    //
    // So the assertion is now SET EQUALITY against the gate, per track. A
    // regression to a single "worst" blocker fails on the first track that has
    // two, which today is twenty-one of twenty-three.
    const gate = realReport().levelGate!;
    const rendered = renderCurriculumGapReport(realReport());
    const lines = rendered.split(/\r?\n/);
    const start = lines.findIndex((l) => l.startsWith("per-track ladder"));
    expect(start).toBeGreaterThan(-1);
    const rows = lines.slice(start + 1).filter((l) => l.startsWith("  ") && l.includes("touches "));

    // EVERY track, not a top-5 slice. The vocabulary line two above this one
    // prints only five, which is what made a script necessary to see the rest.
    expect(rows).toHaveLength(gate.tracks.length);

    // Parse each row back out of the RENDERED text rather than re-deriving from
    // the gate, which would pass even if the renderer dropped everything.
    const rowFor = (language: string): string => {
      const row = rows.find((r) => r.trimStart().startsWith(`${language} `));
      expect(row, `no ladder row for ${language}`).toBeDefined();
      return row!;
    };
    const parseBlockers = (row: string): { criterion: string; shortfall: number }[] => {
      const tail = row.slice(row.lastIndexOf("—") + 1).trim();
      if (tail === "no blocker — ladder complete" || tail.startsWith("no blocker")) return [];
      const body = tail.replace(/^\d+ blockers?: /, "");
      expect(body, `unparsed ladder tail: ${tail}`).not.toBe(tail);
      return body.split(", ").map((part) => {
        const m = /^([a-z-]+) (\d+)$/.exec(part);
        expect(m, `unparsed blocker '${part}'`).not.toBeNull();
        return { criterion: m![1]!, shortfall: Number(m![2]) };
      });
    };

    let multiBlockerTracks = 0;
    let renderedBlockers = 0;
    for (const track of gate.tracks) {
      const row = rowFor(track.language);
      const shown = parseBlockers(row);
      renderedBlockers += shown.length;
      if (track.blockers.length > 1) multiBlockerTracks += 1;
      // Set equality, criterion AND shortfall, in both directions.
      expect(
        [...shown].sort((a, b) => a.criterion.localeCompare(b.criterion)),
        `${track.language} ladder row does not list exactly its blockers`,
      ).toEqual(
        [...track.blockers]
          .map((b) => ({ criterion: b.criterion, shortfall: b.shortfall }))
          .sort((a, b) => a.criterion.localeCompare(b.criterion)),
      );
      // The count in the prose has to agree with the list after it, or the row
      // reads as authoritative while being wrong.
      if (shown.length > 0) {
        expect(row, `${track.language} blocker count disagrees with its list`).toContain(
          `${shown.length} blocker${shown.length === 1 ? "" : "s"}: `,
        );
      }
    }

    // ANTI-VACUITY, and it is the whole point: a corpus where every track had
    // exactly one blocker would satisfy every assertion above against the old
    // renderer too.
    expect(multiBlockerTracks).toBeGreaterThan(1);
    expect(renderedBlockers).toBeGreaterThan(gate.tracks.length);

    // The ordering claim: fewest criteria left first. Unit-free, unlike any
    // ordering over the shortfall numbers themselves.
    const counts = rows.map((r) => parseBlockers(r).length);
    expect([...counts].sort((a, b) => a - b)).toEqual(counts);
  });

  it("scopes each row's vocabulary to at-or-below the level in progress", () => {
    // `vocabulary` on a track is the total headword count; the criterion counts
    // only headwords at or below the level being worked on. Choose a witness by
    // measured properties so no advancing track becomes a shared fixture.
    const gate = realReport().levelGate!;
    const witness = gate.tracks.find((track) => {
      const blocker = track.blockers.find((candidate) => candidate.criterion === "vocabulary");
      if (!blocker || track.inProgressAt === null) return false;
      const target = gate.vocabularyTargets[track.inProgressAt];
      return target - blocker.shortfall !== track.vocabulary;
    });
    expect(witness).toBeDefined();
    const blocker = witness!.blockers.find((candidate) => candidate.criterion === "vocabulary")!;
    const target = gate.vocabularyTargets[witness!.inProgressAt!];
    const rendered = renderCurriculumGapReport(realReport());
    const row = rendered
      .split(/\r?\n/)
      .find((line) => line.trimStart().startsWith(`${witness!.language} `) && line.includes("touches "))!;
    expect(row).toBeDefined();
    expect(row).toContain(`vocabulary ${target - blocker.shortfall}/${target}`);
    expect(row).not.toContain(`vocabulary ${witness!.vocabulary}/${target}`);
  });

  it("does not print the word ATTAINED, because §3.1 measures a corpus", () => {
    // The vocabulary-of-attainment fix. Every §3.1 criterion counts something
    // about the BOOK — headwords, verbs, spine nodes, reinforcement windows,
    // writing stages — and none of them scores a learner. "levels ATTAINED"
    // reads as a claim about what a reader can do, and a track was about to
    // print it having never had one exam item marked against it.
    //
    // Asserting the word is ABSENT, not merely that the new one is present:
    // a rename that left the old phrasing on some other line would satisfy the
    // positive assertion above and change nothing a reader sees.
    const rendered = renderCurriculumGapReport(realReport());
    expect(rendered).not.toContain("ATTAINED");
    expect(rendered).toContain("coverage complete, performance UNVERIFIED");
    // And the distinction is spelled out, not left to be inferred from a
    // heading. The disclaimer is the load-bearing half of this change.
    expect(rendered).toContain("None measures a learner.");
    expect(rendered).toContain("no line above claims a reader can pass anything");
  });

  it("re-derives criteria 2b and 4 for every track at or above A1", () => {
    const gate = realReport().levelGate!;
    const e = REAL_EVERYTHING;
    const spineNodes = lessonSpineNodes(e.curricula);
    const stageOf = new Map<string, CefrLevel>();
    for (const node of e.spine.nodes) {
      if ((CEFR_LEVELS as readonly string[]).includes(node.stage)) {
        stageOf.set(node.id, node.stage as CefrLevel);
      }
    }
    const atOrBelowA1 = (lessonId: string): boolean => {
      const node = spineNodes.get(lessonId);
      const stage = node ? stageOf.get(node) : undefined;
      return stage !== undefined && levelRank(stage) <= levelRank("A1");
    };
    const continuity = measureContinuity(e.lessons);
    const witnesses = gate.tracks.filter(
      (track) => track.attained !== null && levelRank(track.attained) >= levelRank("A1"),
    );
    expect(witnesses.length).toBeGreaterThan(0);
    for (const track of witnesses) {
      // Mirrors the two module-private vocabulary counters. The anchored verb tag
      // deliberately excludes concepts that only contain the substring elsewhere.
      const headwords = new Set<string>();
      const verbs = new Set<string>();
      for (const lesson of e.lessons) {
        if (lesson.realization.language !== track.language) continue;
        if (!CONTENT_TYPES.has(lesson.realization.type)) continue;
        if (!atOrBelowA1(lesson.realization.lessonId)) continue;
        const headword = (lesson.realization.headword ?? "").trim().toLowerCase();
        if (!headword) continue;
        headwords.add(headword);
        if (/(^|-)VERB-/.test(lesson.realization.concept ?? "")) verbs.add(headword);
      }
      expect(headwords.size, track.language).toBeGreaterThanOrEqual(LEVEL_VOCABULARY["A1"]);
      expect(verbs.size, track.language).toBeGreaterThanOrEqual(LEVEL_VERB_VOCABULARY["A1"]);
      const thin = continuity.reinforcement.filter(
        (defect) =>
          defect.language === track.language &&
          atOrBelowA1(defect.introducedBy) &&
          defect.revisits < 2 &&
          !isEtymologyAtom(defect.atom),
      );
      expect(thin.map((defect) => `${defect.atom} (${defect.introducedBy})`), track.language).toEqual([]);
    }
  });
});

describe("etymology is a hook, not a skill", () => {
  it("waives etymology atoms from the reinforcement criterion, and says how many", { timeout: 60_000 }, () => {
    // The owner's decision: an etymology is read once, not drilled. Before this the
    // gate demanded every atom be revisited twice, and the only way to satisfy that
    // for an etymon was to re-state it in the Guided Practice and again in the
    // Wrap-up Recall -- so the GATE was manufacturing the repetition.
    //
    // This test once pinned one track's mutable reinforcement shortfall. A shared
    // integer for unfinished work serializes parallel authoring, so the invariant
    // now checks the waiver itself with a property-selected witness.
    const e = REAL_EVERYTHING;
    const base = measureContinuity(e.lessons);
    const gateInputs = {
      lessons: e.lessons,
      levels: summarizeLevels(e.lessons, e.curricula, e.spine),
      curricula: e.curricula,
      spine: e.spine,
      ramp: measureRamp(e.lessons, loadChapterPolicy()),
    };

    // The waiver must be VISIBLE. A silently loosened gate is worse than a strict one.
    //
    // This used to read the waiver off whichever real track still had a reinforcement
    // blocker. The A2 reinforcement tranches closed the last of them, so the corpus no
    // longer holds one, and the witness is built instead: take a track that has
    // waived etymology atoms at or below the level it is working on, add ONE
    // non-etymology atom there that nothing revisits, and read the blocker that
    // produces. The detail must name the level and the number of hooks waived.
    const baseline = runLevelGate({ ...gateInputs, continuity: base });
    const stageOf = new Map(e.spine.nodes.map((node) => [node.id, node.stage]));
    const nodeOf = lessonSpineNodes(e.curricula);
    const atOrBelow = (lessonId: string, level: CefrLevel) => {
      const stage = stageOf.get(nodeOf.get(lessonId) ?? "");
      return stage !== undefined && levelRank(stage as CefrLevel) <= levelRank(level);
    };
    // The witness also needs a lesson AT the level it is working on, to host the
    // synthetic atom below. A track that has just climbed a rung is in progress at
    // the next one (B1 for a new A2 track) before it has any lesson there, and an
    // atom with no host is counted at no level at all.
    const hasLessonAt = (language: string, level: CefrLevel) =>
      e.lessons.some(
        (l) => l.language === language && stageOf.get(nodeOf.get(l.realization.lessonId) ?? "") === level,
      );
    const witness = baseline.tracks
      .filter((t) => t.inProgressAt !== null && hasLessonAt(t.language, t.inProgressAt))
      .sort((a, b) => a.language.localeCompare(b.language))
      .find((t) =>
        base.reinforcement.some(
          (d) => d.language === t.language && isEtymologyAtom(d.atom) && atOrBelow(d.introducedBy, t.inProgressAt!),
        ),
      )!;
    expect(witness).toBeDefined();
    const level = witness.inProgressAt!;
    const host = e.lessons
      .filter((l) => l.language === witness.language)
      .map((l) => l.realization.lessonId)
      .sort()
      .find((id) => stageOf.get(nodeOf.get(id) ?? "") === level)!;
    const thinAtom = {
      atom: "SYNTHETIC-GRAMMAR-UNREVISITED-01",
      language: witness.language,
      introducedBy: host,
      introducedAt: 0,
      missed: ["R1", "R2", "R3", "R4"],
      revisits: 0,
    } as ContinuityReport["reinforcement"][number];
    const withThin = runLevelGate({
      ...gateInputs,
      continuity: { ...base, reinforcement: [...base.reinforcement, thinAtom] },
    }).tracks.find((t) => t.language === witness.language)!;
    const reinforcement = withThin.blockers.find((b) => b.criterion === "reinforcement")!;
    expect(reinforcement).toBeDefined();
    expect(reinforcement.detail).toContain(`atom(s) at or below ${level} are rev`);
    expect(reinforcement.detail).toMatch(/\(\d+ etymology hook\(s\) waived\)/);

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
    // hypothetical: when the waiver changes a track's rung, shortfalls from the two
    // runs are no longer commensurable. The verdict comparison below handles both
    // the same-rung and changed-rung cases.
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
    const e = REAL_EVERYTHING;
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
    const e = REAL_EVERYTHING;
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

  it("still BITES on a track that reaches the total with the wrong parts", () => {
    // This test exists because the corpus stopped providing the red case.
    //
    // Criterion 2b was first proved on mutable corpus data. Once authoring
    // discharged its last real blocker, the suite needed a stable red case: the
    // whole counter could otherwise disappear while every positive corpus test
    // stayed green.
    //
    // So the red case moves from the corpus into a fixture, where authoring can
    // never turn it green. The track below satisfies criterion 2 EXACTLY -- all
    // `LEVEL_VOCABULARY["pre-A1"]` headwords, every one distinct -- and carries
    // one verb fewer than criterion 2b asks for. Nothing else about it is short.
    const verbTarget = LEVEL_VERB_VOCABULARY["pre-A1"];
    const lessons = Array.from({ length: LEVEL_VOCABULARY["pre-A1"] }, (_, index) =>
      parseLesson(`---
id: BB-${String(index + 1).padStart(3, "0")}
chapter: 1
type: word
headword: word-${index + 1}
gloss: word ${index + 1}
concept_tag: ${index < verbTarget - 1 ? "BB-VERB" : "BB-WORD"}-${index + 1}
---

word ${index + 1}
`, "beta"),
    );
    const curricula = [{
      version: 1,
      language: "beta",
      path: [{
        id: "beta-pre-a1",
        spine_node: "PRE",
        lessons: lessons.map((lesson) => lesson.realization.lessonId),
        before: [],
        inline: [],
        after: [],
      }],
      spine: { PRE: { segments: ["beta-pre-a1"], omits: [], relocates: {} } },
      extensions: [],
    }];
    const spine = {
      version: 1,
      stages: ["pre-A1", "A1", "A2", "B1", "B2", "C1", "C2"],
      nodes: [{
        id: "PRE",
        stage: "pre-A1",
        strand: "LEXICON",
        canDo: "name a word",
        prerequisites: [],
        core: true,
        concepts: [],
      }],
    };
    const levels = summarizeLevels(lessons, curricula, spine);
    const gate = runLevelGate({
      lessons,
      levels,
      curricula,
      spine,
      ramp: measureRamp(lessons, loadChapterPolicy()),
      continuity: { reinforcement: [] } as unknown as ContinuityReport,
    });
    const beta = gate.tracks.find((track) => track.language === "beta")!;

    // The total is met, so criterion 2 says nothing.
    expect(beta.blockers.find((b) => b.criterion === "vocabulary")).toBeUndefined();
    // Composition is one short, and criterion 2b is the only thing that can tell.
    expect(beta.blockers).toEqual([
      expect.objectContaining({ criterion: "verb-vocabulary", shortfall: 1 }),
    ]);
    expect(beta.attained).toBeNull();

    // And the counterfactual, because "it failed" is not the same as "it failed
    // for the right reason". Tag the one missing lesson as a verb and the level
    // is attained -- so the blocker above was caused by the composition and by
    // nothing else in the fixture.
    const fixed = lessons.map((lesson, index) =>
      index === verbTarget - 1
        ? parseLesson(`---
id: BB-${String(index + 1).padStart(3, "0")}
chapter: 1
type: word
headword: word-${index + 1}
gloss: word ${index + 1}
concept_tag: BB-VERB-${index + 1}
---

word ${index + 1}
`, "beta")
        : lesson,
    );
    const repaired = runLevelGate({
      lessons: fixed,
      levels: summarizeLevels(fixed, curricula, spine),
      curricula,
      spine,
      ramp: measureRamp(fixed, loadChapterPolicy()),
      continuity: { reinforcement: [] } as unknown as ContinuityReport,
    });
    expect(repaired.tracks.find((track) => track.language === "beta")!.attained).toBe("pre-A1");
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

  it("is absent, not empty, when the caller supplied no policy", { timeout: 60_000 }, () => {
    // "Not measured" and "attained nothing" are opposite facts. A consumer that
    // passes no chapter policy must not see a report claiming zero attainment.
    const e = REAL_EVERYTHING;
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
