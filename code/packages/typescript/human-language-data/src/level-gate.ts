// What it takes to CLAIM a level — HL09 §3.1.
//
// The gap report said Spanish "reached A2". The project owner, who has sat A2
// examinations, did not believe it, and was right by a factor of about seven:
// 178 distinct words against the ~1,000-1,500 A2 asks for, fourteen lessons all
// realizing a single spine node, and only the present tense taught.
//
// Nothing lied. `TrackLevelCoverage.reach` is documented as "the highest level
// this track has any lesson at", and that was true. The mistake was letting a
// number that means TOUCHES be read as MEANS. One lesson pointed at one A2 node
// is enough to move `reach`; it is nowhere near enough to sit the exam.
//
// So this module computes a second, stricter number and reports both. `touches`
// keeps the old meaning. `attained` is the highest level where every enabled
// criterion holds, at that level AND every level below it:
//
//   1. every spine node at the level is realized by some path segment,
//   2. the track teaches at least the level's cumulative vocabulary,
//  2b. and enough of that vocabulary is VERBS (HL09 §3.1 / HL23 §6 — every count
//      criterion carries a composition criterion, because a total can always be
//      reached by the wrong parts),
//   3. no lesson at or below the level exceeds the new-atom budget,
//   4. every atom at or below the level is revisited at least twice, and
//   5. when HL19 evidence is supplied, every cumulative writing stage is proved.
//
// A track that fails any of them is "in progress at X", never "reached X", and
// the report says WHICH criterion failed and by how much — a bare `false` would
// just move the argument rather than settle it.

import type { CefrLevel, LevelSummary } from "./levels.js";
import { CEFR_LEVELS, levelRank, lessonSpineNodes } from "./levels.js";
import { CONTENT_TYPES } from "./constants.js";
import type { ContinuityReport } from "./continuity.js";
import type { RampReport } from "./ramp.js";
import type { ParsedLesson } from "./parse.js";
import type { CurriculumSpine, LanguageCurriculum } from "./types.js";
import type { WritingStageReport } from "./writing-stages.js";

/**
 * Cumulative vocabulary a level asks for, in distinct taught headwords.
 *
 * EDITORIAL, per HL09 §10 — these are the conventional working figures for CEFR
 * receptive vocabulary size, not a claim about any awarding body's published
 * syllabus. They are recorded here rather than inlined so that a track's failure
 * can name the number it was measured against.
 */
export const LEVEL_VOCABULARY: Record<CefrLevel, number> = {
  "pre-A1": 300,
  A1: 600,
  A2: 1200,
  B1: 2500,
  B2: 4000,
  C1: 8000,
  C2: 16000,
};

/**
 * Cumulative VERB vocabulary a level asks for, in distinct taught headwords.
 *
 * EDITORIAL, and more so than `LEVEL_VOCABULARY` above — which at least restates
 * conventional working figures for CEFR receptive vocabulary size. **No awarding
 * body publishes a verb quota, and this table is not derived from one.** It is a
 * project choice, recorded here so that it can be argued with rather than
 * absorbed. HL23 §6.2 proposed these numbers and HL09 §10 governs how they must
 * be labelled; this comment is that label.
 *
 * The shape, stated so the numbers are not merely asserted: against
 * `LEVEL_VOCABULARY` they are 1.7% at pre-A1, 6.7% at A1, and 10% at every level
 * from A2 up. A beginner level is allowed to be noun-heavy — the first hundred
 * words of any language are greetings, numbers and things — and the share then
 * converges on a tenth. Ten percent is the editorial asymptote, not a measured
 * property of any corpus.
 *
 * The measurement under it fails SAFE: `verbVocabularyOf` identifies a verb by
 * its concept tag, which HL23 §6.1 measured at 96.2% recall corpus-wide, and the
 * residual 4% are verbs the tag misses. So the count runs LOW, and a track is
 * flagged when it should not be far more readily than certified when it should
 * not be. A composition check that erred the other way would be worse than none.
 */
export const LEVEL_VERB_VOCABULARY: Record<CefrLevel, number> = {
  "pre-A1": 5,
  A1: 40,
  A2: 120,
  B1: 250,
  B2: 400,
  C1: 800,
  C2: 1600,
};

/**
 * Is this atom an etymology hook rather than a skill?
 *
 * Every track names them the same way — `ES-ETYMON-CREDERE-02`, `TA-ETYMON-NAL-01` —
 * so the id carries the fact. Note this is a CONVENTION, not an enforced schema: an
 * etymology atom named some other way is not waived, and a census found a handful
 * (`ES-HISTORY-AL-ANDALUS-LOANS`, `SA-SOUND-PIE-KW-OUTCOMES`) that arguably qualify
 * and are not matched. Naming them consistently is the fix, not widening this regex.
 */
export function isEtymologyAtom(atom: string): boolean {
  return /-ETYMON-/.test(atom);
}

/** Which attainment criterion a level failed, and by how much. */
export interface LevelBlocker {
  criterion:
    | "spine-nodes"
    | "vocabulary"
    | "verb-vocabulary"
    | "atom-budget"
    | "reinforcement"
    | "writing-stage";
  detail: string;
  /** How far short the track is, in the criterion's own units. */
  shortfall: number;
}

export interface TrackLevelAttainment {
  language: string;
  /** Highest level any lesson SITS at. The old `reach` — touches, not means. */
  touches: CefrLevel | null;
  /** Highest level meeting every §3.1 criterion, here and below. Usually lower. */
  attained: CefrLevel | null;
  /** The level the track is working on: one above `attained`. */
  inProgressAt: CefrLevel | null;
  /** Why `inProgressAt` is not yet attained. Empty only when the ladder is complete. */
  blockers: LevelBlocker[];
  /** Distinct headwords the track teaches at ANY level — context, not the criterion. */
  vocabulary: number;
}

export interface LevelGateReport {
  vocabularyTargets: Record<CefrLevel, number>;
  /** Reported beside the totals so a shortfall names the number it was measured against. */
  verbVocabularyTargets: Record<CefrLevel, number>;
  tracks: TrackLevelAttainment[];
  summary: {
    /** Tracks whose `touches` overstates their `attained` — the bug this exists for. */
    tracksOverstating: number;
    /** Tracks that have attained anything at all. */
    tracksWithAnyLevel: number;
    /** Per level, how many tracks STOP at it — attaining a level implies those below. */
    attainedByLevel: Record<CefrLevel, number>;
  };
}

export interface LevelGateInput {
  lessons: ParsedLesson[];
  levels: LevelSummary;
  curricula: LanguageCurriculum[];
  spine: CurriculumSpine;
  ramp: RampReport;
  continuity: ContinuityReport;
  /** Optional only for compatibility with callers that have not loaded HL16 policy. */
  writingStages?: WritingStageReport;
}

/**
 * Distinct headwords a set of lessons teaches — criterion 2's measurement.
 *
 * Restricted to CONTENT_TYPES (`word`/`phrase`). Without that filter the count
 * picks up drill titles and grammar labels as though they were vocabulary —
 * `(practice)`, `qu-`, `fact or wish?`, `chapter 18 practice` — which inflated
 * Spanish by 25 of 138. `constants.ts` already says those types carry "a
 * session/orthography label, not a cross-language concept"; counting them toward a
 * vocabulary target is exactly the kind of number-means-something-else this module
 * exists to stop.
 */
function vocabularyOf(lessons: ParsedLesson[]): number {
  const words = new Set<string>();
  for (const lesson of lessons) {
    if (!CONTENT_TYPES.has(lesson.realization.type)) continue;
    const headword = (lesson.realization.headword ?? "").trim().toLowerCase();
    if (headword) words.add(headword);
  }
  return words.size;
}

/**
 * Distinct headwords a set of lessons teaches whose concept names a VERB.
 *
 * `vocabularyOf` above counts vocabulary; this counts what KIND. The two have to
 * be asserted separately, because **a total can always be reached by the wrong
 * parts**: six hundred nouns satisfy criterion 2 exactly as well as six hundred
 * words a learner can build a sentence with, and only one of those is A1.
 *
 * That is not hypothetical. HL23 measured Spanish at 584 headwords at or below
 * A1 of which SEVEN were verb-tagged — five distinct lexemes — while the same
 * levels taught the complete present paradigm of all three conjugations. The
 * learner had the machinery and nothing to run it on, and criterion 2 reported
 * a track sixteen words from certification.
 *
 * The verb signal is the `concept_tag` the validator ALREADY requires every
 * content lesson to carry, so this costs no new frontmatter, no re-authoring and
 * no schema change. `(^|-)VERB-` catches the canonical `VERB-EAT` and the
 * namespaced `ES-VERB-LAVAR` alike, and matches neither `ADVERB-*` nor a
 * hypothetical `PROVERB-*`, because the boundary is anchored.
 */
function verbVocabularyOf(lessons: ParsedLesson[]): number {
  const words = new Set<string>();
  for (const lesson of lessons) {
    if (!CONTENT_TYPES.has(lesson.realization.type)) continue;
    if (!/(^|-)VERB-/.test(lesson.realization.concept ?? "")) continue;
    const headword = (lesson.realization.headword ?? "").trim().toLowerCase();
    if (headword) words.add(headword);
  }
  return words.size;
}

/** Run the HL09 §3.1 gate over every track. */
export function runLevelGate(input: LevelGateInput): LevelGateReport {
  const { lessons, levels, curricula, spine, ramp, continuity, writingStages } = input;

  // Spine nodes per level, and the segments that realize them.
  const nodesByLevel = new Map<CefrLevel, string[]>();
  for (const node of spine.nodes) {
    const level = node.stage as CefrLevel;
    if (!CEFR_LEVELS.includes(level)) continue;
    const list = nodesByLevel.get(level) ?? [];
    list.push(node.id);
    nodesByLevel.set(level, list);
  }

  const realizedByTrack = new Map<string, Set<string>>();
  for (const curriculum of curricula) {
    const realized = new Set<string>();
    for (const segment of curriculum.path ?? []) {
      // A segment with no lessons is a declared gap, not a realization — the
      // ledgers use `segments: []` with `omits` to record exactly that.
      if ((segment.lessons ?? []).length > 0 && segment.spine_node) {
        realized.add(segment.spine_node);
      }
    }
    realizedByTrack.set(curriculum.language, realized);
  }

  const lessonsByTrack = new Map<string, ParsedLesson[]>();
  for (const lesson of lessons) {
    const list = lessonsByTrack.get(lesson.language) ?? [];
    list.push(lesson);
    lessonsByTrack.set(lesson.language, list);
  }

  // Every criterion is scoped "at or below the level", per §3.1. That needs a
  // lesson -> level map, and building it is the difference between a gate that
  // measures attainment and one that measures the whole track against a per-level
  // target — the same touches-vs-means confusion this module exists to end.
  const spineNodes = lessonSpineNodes(curricula);
  const stageOf = new Map<string, CefrLevel>();
  for (const node of spine.nodes) {
    const stage = node.stage as CefrLevel;
    if (CEFR_LEVELS.includes(stage)) stageOf.set(node.id, stage);
  }
  const levelOfLesson = new Map<string, CefrLevel>();
  for (const lesson of lessons) {
    const node = spineNodes.get(lesson.realization.lessonId);
    const stage = node ? stageOf.get(node) : undefined;
    if (stage) levelOfLesson.set(lesson.realization.lessonId, stage);
  }
  /**
   * Is this lesson at or below `ceiling`?
   *
   * A lesson no spine node claims is NOT counted toward any level. `levels.ts` makes
   * the same call for the book filter, and for the same reason: crediting unplaced
   * material toward a level is how a ramp stops being one.
   */
  const atOrBelow = (lessonId: string, ceiling: CefrLevel): boolean => {
    const level = levelOfLesson.get(lessonId);
    return level !== undefined && levelRank(level) <= levelRank(ceiling);
  };

  const attainedByLevel = Object.fromEntries(
    CEFR_LEVELS.map((level) => [level, 0]),
  ) as Record<CefrLevel, number>;
  const tracks: TrackLevelAttainment[] = [];

  for (const coverage of levels.tracks) {
    const language = coverage.language;
    const trackLessons = lessonsByTrack.get(language) ?? [];
    const realized = realizedByTrack.get(language) ?? new Set<string>();
    // Reported for context: everything the track teaches, at any level.
    const vocabulary = vocabularyOf(trackLessons);
    const rampViolations = ramp.lessons.filter((v) => v.language === language);
    const underReinforced = continuity.reinforcement.filter((d) => d.language === language);
    const writingCoverage = writingStages?.tracks.find((track) => track.language === language);

    let attained: CefrLevel | null = null;
    let inProgressAt: CefrLevel | null = null;
    let blockers: LevelBlocker[] = [];

    for (const level of CEFR_LEVELS) {
      const failures: LevelBlocker[] = [];

      if (writingStages) {
        const stageCoverage = writingCoverage?.levels.find((entry) => entry.level === level);
        const missing = stageCoverage?.missingStages ?? writingStages.stages
          .filter((stage) => levelRank(stage.firstRequiredAt) <= levelRank(level))
          .map((stage) => stage.id);
        if (missing.length > 0) {
          failures.push({
            criterion: "writing-stage",
            detail: `${missing.length} cumulative writing stage(s) unproved at ${level}: ${missing.join(", ")}`,
            shortfall: missing.length,
          });
        }
      }

      const nodes = nodesByLevel.get(level) ?? [];
      if (nodes.length === 0) {
        // "No node is unrealized" is not "every node is realized". spine.json has
        // zero B1-C2 nodes, so without this those levels would pass criterion 1 on
        // no evidence whatsoever — the touches-vs-means error, one level up.
        failures.push({
          criterion: "spine-nodes",
          detail: `no ${level} spine nodes are authored, so ${level} cannot be attained`,
          shortfall: 1,
        });
      } else {
        const missing = nodes.filter((id) => !realized.has(id));
        if (missing.length > 0) {
          failures.push({
            criterion: "spine-nodes",
            detail: `${missing.length} of ${nodes.length} ${level} spine node(s) unrealized: ${missing.slice(0, 4).join(", ")}${missing.length > 4 ? "…" : ""}`,
            shortfall: missing.length,
          });
        }
      }

      const target = LEVEL_VOCABULARY[level];
      const atLevel = vocabularyOf(
        trackLessons.filter((l) => atOrBelow(l.realization.lessonId, level)),
      );
      if (atLevel < target) {
        failures.push({
          criterion: "vocabulary",
          detail: `teaches ${atLevel} distinct headwords at or below ${level}, against ${target}`,
          shortfall: target - atLevel,
        });
      }

      // Criterion 2b — HL09 §3.1's composition companion to the count above.
      // Deliberately computed from the SAME `atLevel` lesson slice, so the two
      // numbers can never disagree about which lessons they were measuring.
      const verbTarget = LEVEL_VERB_VOCABULARY[level];
      const verbsAtLevel = verbVocabularyOf(
        trackLessons.filter((l) => atOrBelow(l.realization.lessonId, level)),
      );
      if (verbsAtLevel < verbTarget) {
        failures.push({
          criterion: "verb-vocabulary",
          detail: `teaches ${verbsAtLevel} distinct verb headwords at or below ${level}, against ${verbTarget}`,
          shortfall: verbTarget - verbsAtLevel,
        });
      }

      const overBudget = rampViolations.filter((v) => atOrBelow(v.lessonId, level));
      if (overBudget.length > 0) {
        failures.push({
          criterion: "atom-budget",
          detail: `${overBudget.length} lesson(s) at or below ${level} exceed the new-atom budget`,
          shortfall: overBudget.length,
        });
      }

      // §3.1 asks for TWO revisits, so `revisits < 2` — not `=== 0`. Measuring
      // zero-revisit atoms only would have hidden 51 of Spanish's 141 failures.
      //
      // Etymology atoms are WAIVED here, by the project owner's decision: an etymology
      // is a memory hook, read once, not a skill to be drilled. Before this, the gate
      // demanded every atom be revisited twice, and the only way to satisfy that for an
      // etymon was to re-state it in the Guided Practice and again in the Wrap-up
      // Recall — so the gate was manufacturing the repetition the owner asked to
      // remove.
      //
      // The waiver lives HERE and not in `continuity.ts` on purpose. `measureContinuity`
      // goes on reporting every atom truthfully, so `atomsTaught`,
      // `atomsNeverRevisited` and the R-window counts keep meaning what they say and
      // the gap report stays honest. Only the LEVEL CLAIM ignores them, which is the
      // one place the decision actually applies — and it is visible in
      // `waivedEtymologyAtoms` rather than silently absent.
      const relevant = underReinforced.filter((d) => atOrBelow(d.introducedBy, level));
      const waived = relevant.filter((d) => isEtymologyAtom(d.atom));
      const thin = relevant.filter((d) => d.revisits < 2 && !isEtymologyAtom(d.atom));
      if (thin.length > 0) {
        failures.push({
          criterion: "reinforcement",
          detail:
            `${thin.length} atom(s) at or below ${level} are revisited fewer than twice` +
            (waived.length > 0 ? ` (${waived.length} etymology hook(s) waived)` : ""),
          shortfall: thin.length,
        });
      }

      if (failures.length === 0) {
        attained = level;
        continue;
      }
      // The first level that fails is the one in progress; everything above it is
      // unreachable by definition, since the criteria are cumulative.
      inProgressAt = level;
      blockers = failures;
      break;
    }

    if (attained) attainedByLevel[attained] += 1;

    tracks.push({
      language,
      touches: coverage.reach,
      attained,
      inProgressAt,
      blockers,
      vocabulary,
    });
  }

  tracks.sort((a, b) => a.language.localeCompare(b.language));
  return {
    vocabularyTargets: { ...LEVEL_VOCABULARY },
    verbVocabularyTargets: { ...LEVEL_VERB_VOCABULARY },
    tracks,
    summary: {
      tracksOverstating: tracks.filter(
        (t) =>
          t.touches !== null &&
          (t.attained === null || levelRank(t.touches) > levelRank(t.attained)),
      ).length,
      tracksWithAnyLevel: tracks.filter((t) => t.attained !== null).length,
      attainedByLevel,
    },
  };
}
