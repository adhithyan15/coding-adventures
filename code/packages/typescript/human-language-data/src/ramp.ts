// Is the ramp actually gentle? — HL08's budgets, finally measured.
//
// `core/chapter-policy.json` has declared `maxNewAtomsPerLesson: 3` and
// `maxNewAtomsPerChapter: 12` since HL08, with a rationale saying they sit at the corpus's
// own p90 so "only genuine spikes are flagged". **Nothing read either number.** They were
// policy in the sense that a sign is policy: written down, and enforced by nobody.
//
// That is not a small gap. "A very gentle ramp" is the project's founding promise, and
// HL-C18 exists to burn down the lessons that break it — but the figure everyone quoted
// ("52 over-budget lessons") came from an ad-hoc count that no test reproduces, and a fresh
// count returns something different because most of the corpus is schema-v1 and carries no
// machine-readable atoms at all. You cannot burn down a list you cannot recompute.
//
// This module recomputes it. Report-only, like the HL05 chapter gates: the debt predates
// the measurement, so it is measured and made visible rather than turned into a build
// failure on a corpus nobody regressed.
//
// THE HONEST LIMIT, stated because it changes how the number should be read: a lesson only
// counts atoms it declares. Schema-v1 lessons declare none, so they are reported separately
// as `unmeasurable` rather than silently counted as compliant. An explicit schema-v2
// retrieval contract can prove that zero introductions is intentional by naming an empty
// introduction list and the atoms being practised. Writing retrieval has one more burden:
// an aligned, explicit stage claim that distinguishes copying and transcription from new
// composition. A track with a low violation count and a high unmeasurable count has not
// proved it is gentle; it has proved its frontier is still unknown.

// ─────────────────────────────────────────────────────────────────────────────
// THE SECOND RAMP: script.
//
// Everything above counts *atoms* — units of meaning. That is one of the two
// burdens a lesson imposes, and the corpus had no measurement of the other.
// Before a learner can mean anything by नमस्ते they have to decode it, and
// decoding is a separate skill on a separate curve.
//
// The gap was not subtle. `HI-W01-shirorekha-na-ma` declares exactly ONE atom
// and puts TWELVE new Devanagari glyphs on the page. It passes
// `maxNewAtomsPerLesson: 3` comfortably. Sixty-one lessons are in that position;
// thirty-eight of them declare zero atoms and so read as maximally gentle while
// teaching up to a dozen new shapes.
//
// TARGET vs FOREIGN — the distinction that makes the number honest.
//
// A Kannada learner must eventually read every Kannada glyph. The Devanagari,
// Tamil, Telugu and Malayalam in a cousin table is *context* — it says "your
// language's word for thanks is the same word Hindi uses" to a reader who
// happens to know Hindi, and it is meant to be skippable by everyone else.
// Charging both to one budget is what makes `KA-C01-dhanyavada` look like a
// 34-glyph cliff in Chapter 1. Its actual Kannada load is 7.
//
// So foreign glyphs are counted, reported, and never charged against the
// budget. They are the cousin layer's footprint, useful to see and wrong to
// penalise. What they DO justify is keeping that layer visually separable, so a
// reader who does not know Hindi can skip it without skipping the lesson.
//
// ORDER MATTERS, so this walks lessons in reading order (chapter, then
// sequence) per track and counts only what is genuinely new at that point. A
// glyph taught in Chapter 1 is free in Chapter 30. That is what makes it a ramp
// measurement rather than a density measurement.

import type { ChapterPolicy, Script } from "./types.js";
import type { ParsedLesson } from "./parse.js";
import { hasOwn } from "./constants.js";

/**
 * Which Unicode scripts make up each of the curriculum's script ids.
 *
 * Nearly all are one-to-one. `japanese` is the exception that motivates the
 * whole `maxNewScriptSystemsPerLesson` rule: three writing systems behind one
 * id, which is also why HL-C42 wants a track to be able to declare several.
 * `perso-arabic` and `urdu-nastaliq` are Arabic script in different hands, so
 * they resolve to the same Unicode script.
 */
const SCRIPT_SYSTEMS_MUTABLE: Record<string, string[]> = {
  latin: ["Latin"],
  devanagari: ["Devanagari"],
  bengali: ["Bengali"],
  gurmukhi: ["Gurmukhi"],
  gujarati: ["Gujarati"],
  tamil: ["Tamil"],
  telugu: ["Telugu"],
  kannada: ["Kannada"],
  malayalam: ["Malayalam"],
  arabic: ["Arabic"],
  "perso-arabic": ["Arabic"],
  "urdu-nastaliq": ["Arabic"],
  cyrillic: ["Cyrillic"],
  hebrew: ["Hebrew"],
  chinese: ["Han"],
  japanese: ["Hiragana", "Katakana", "Han"],
};

/**
 * The track-to-scripts map, frozen.
 *
 * `ALL_SYSTEMS` and `SYSTEM_MATCHERS` below are computed ONCE at module load
 * from this object. Exporting it mutable would let a consumer add a script
 * afterwards: membership tests would see the new script, `systemOf` would never
 * learn it, and every glyph of it would classify as null -- so the track would
 * report ZERO debt while appearing measured. That is this module's own failure
 * mode, reachable from outside the package, so the map and the tables derived
 * from it are pinned together.
 */
export const SCRIPT_SYSTEMS: Readonly<Record<string, readonly string[]>> = Object.freeze(
  Object.fromEntries(
    Object.entries(SCRIPT_SYSTEMS_MUTABLE).map(([k, v]) => [k, Object.freeze(v)]),
  ),
);

/** Every Unicode script this curriculum can name, for classifying a stray glyph. */
const ALL_SYSTEMS = [
  ...new Set(Object.values(SCRIPT_SYSTEMS).flat()),
].filter((system) => system !== "Latin");

/**
 * `Script_Extensions`, not `Script`, and the difference is load-bearing.
 *
 * Several characters a learner must decode are formally `Script=Common` because
 * more than one script borrows them — Japanese's prolonged-sound mark ー
 * (U+30FC) is the clearest case, shared by hiragana and katakana and therefore
 * belonging to neither under the narrow property. `Script=Katakana` misses it;
 * `Script_Extensions=Katakana` catches it. Matching on the narrow property
 * undercounted コーヒー by exactly the mark that makes it a long vowel.
 */
const SYSTEM_MATCHERS: ReadonlyArray<readonly [string, RegExp]> = ALL_SYSTEMS.map((system) => {
  try {
    return [system, new RegExp(`\\p{Script_Extensions=${system}}`, "u")] as const;
  } catch {
    // These regexes are built at module load, and `index.ts` re-exports this file, so a
    // bad name takes down every CLI in the package — book generation, narration,
    // modality, validate — with a SyntaxError naming a regex rather than the map that
    // caused it. The names that throw are exactly the plausible extensions: "Kanji",
    // "Nastaliq", or a typo like "Devangari". Say which one and where.
    throw new Error(
      `SCRIPT_SYSTEMS names "${system}", which is not a Unicode script. ` +
        `Use the script's Unicode name (e.g. "Han", not "Kanji"; "Arabic", not "Nastaliq").`,
    );
  }
});

/**
 * The Unicode script a character belongs to, or null when it is not script at all.
 *
 * Punctuation, spaces, format characters and Latin are all "not script" here. Latin
 * is excluded deliberately: romanization (`namaskāram`) rides alongside every
 * non-Latin headword, and counting `ā` as a glyph to be learned would swamp the
 * signal with the very thing that exists to make the script approachable.
 *
 * Two things are deliberately NOT excluded. Combining marks: a Devanagari mātrā is
 * a shape the learner must read, and dropping it would undercount every abugida in
 * the corpus. And digits: ०१२ and ۱۲۳ are `\p{N}`, but they are also glyphs nobody
 * born to ASCII can already read, so a numbers lesson genuinely does teach script.
 * Latin digits never reach the matchers, so no exclusion is needed for them.
 */
export function systemOf(ch: string): string | null {
  if (/[\s\p{P}\p{C}]/u.test(ch)) return null;
  for (const [system, matcher] of SYSTEM_MATCHERS) {
    if (matcher.test(ch)) return system;
  }
  return null;
}

/**
 * Does this character belong to ANY of these scripts?
 *
 * `systemOf` returns the FIRST match in map order, which is the right answer for
 * "what is this glyph" and the wrong one for "is this glyph mine".
 * `Script_Extensions` is set-valued, and several marks a learner must read belong
 * to many scripts at once. U+0951 and U+0952, the Vedic tone marks, carry
 * Devanagari, Bengali, Gujarati, Gurmukhi, Kannada, Malayalam, Oriya, Tamil,
 * Telugu and Grantha between them; U+1CD0 carries Devanagari, Bengali, Kannada
 * and Grantha. `systemOf` attributes every one of them to Devanagari, because
 * that is simply where the map happens to start.
 *
 * (Not every mark in that block is shared -- U+A8E0 is Devanagari alone. The
 * membership is per-character, which is the reason to ask the regex rather than
 * to keep a list.)
 *
 * In a non-Devanagari Indic track those marks would then be neither shown nor
 * load-bearing -- silently dropped, an undercount in exactly the abugidas the
 * script measurements exist to serve. So a caller asking about a specific target
 * asks this instead, and gets an answer that does not depend on iteration order.
 */
export function belongsToAny(ch: string, systems: ReadonlySet<string>): boolean {
  if (/[\s\p{P}\p{C}]/u.test(ch)) return false;
  for (const [system, matcher] of SYSTEM_MATCHERS) {
    if (systems.has(system) && matcher.test(ch)) return true;
  }
  return false;
}

/**
 * Reading order within a track: chapter, then sequence, then id for stability.
 *
 * Exported because `continuity.ts` measures the same walk. Two independent
 * orderings that drift apart would make the two reports disagree about which
 * lesson comes first, and the disagreement would be silent.
 */
export function readingOrder(a: ParsedLesson, b: ParsedLesson): number {
  const chapter = (lesson: ParsedLesson) =>
    typeof lesson.realization.chapter === "number" && Number.isFinite(lesson.realization.chapter)
      ? lesson.realization.chapter
      : Number.MAX_SAFE_INTEGER;
  const sequence = (lesson: ParsedLesson) => {
    const raw = lesson.frontmatter.sequence;
    const value = typeof raw === "number" ? raw : Number(raw);
    return Number.isFinite(value) ? value : Number.MAX_SAFE_INTEGER;
  };
  return (
    chapter(a) - chapter(b) ||
    sequence(a) - sequence(b) ||
    a.realization.lessonId.localeCompare(b.realization.lessonId)
  );
}

/** One lesson that introduces more than the budget allows. */
export interface RampViolation {
  lessonId: string;
  language: string;
  chapter: number | null;
  /** Atoms this lesson introduces. */
  atoms: number;
  /** The budget it exceeded. */
  budget: number;
}

/** One chapter that introduces more than the chapter budget allows. */
export interface ChapterRampViolation {
  language: string;
  chapter: number;
  atoms: number;
  budget: number;
  /** Lessons in the chapter, so a splitter knows what it is working with. */
  lessonCount: number;
}

export interface TrackRampCoverage {
  language: string;
  lessonCount: number;
  /** Lessons declaring at least one atom — the ones this can actually judge. */
  measurable: number;
  /** Lessons declaring none, almost always schema-v1. NOT evidence of gentleness. */
  unmeasurable: number;
  lessonViolations: number;
  chapterViolations: number;
}

/** One lesson putting more new target-script glyphs on the page than the budget allows. */
export interface ScriptRampViolation {
  lessonId: string;
  language: string;
  chapter: number | null;
  /** New glyphs of the track's OWN script, first seen in this lesson. */
  glyphs: number;
  /** The glyphs themselves, so a splitter can see what it is dividing. */
  sample: string;
  /** Writing systems opened here — >1 is its own violation. */
  systems: string[];
  budget: number;
}

/** One lesson opening more writing systems at once than the budget allows. */
export interface ScriptSystemViolation {
  lessonId: string;
  language: string;
  chapter: number | null;
  systems: string[];
  budget: number;
}

export interface TrackScriptRamp {
  language: string;
  /** The track's declared script id. */
  script: Script;
  /** Latin-script tracks carry no decoding burden and are measured but never flagged. */
  latinScript: boolean;
  lessonCount: number;
  /** Distinct target-script glyphs the whole track ever shows. */
  totalGlyphs: number;
  lessonViolations: number;
  systemViolations: number;
  /** Lessons showing at least one glyph from a DIFFERENT script (cousin tables). */
  lessonsWithForeignScript: number;
}

export interface ScriptRampReport {
  policy: { maxNewGlyphsPerLesson: number; maxNewScriptSystemsPerLesson: number };
  lessons: ScriptRampViolation[];
  systems: ScriptSystemViolation[];
  tracks: TrackScriptRamp[];
  summary: {
    /** Lessons above `maxNewGlyphsPerLesson` — the script burn-down list. */
    lessonViolations: number;
    /** Lessons opening more than one writing system at once. */
    systemViolations: number;
    /** Lessons carrying cousin-table glyphs. Context, never a violation. */
    lessonsWithForeignScript: number;
    /** Most foreign glyphs any one lesson shows, so the cousin layer's cost is visible. */
    maxForeignGlyphsInALesson: number;
    /** The steepest single lesson, where a burn-down starts. */
    steepestLesson: ScriptRampViolation | null;
  };
}

export interface RampReport {
  policy: { maxNewAtomsPerLesson: number; maxNewAtomsPerChapter: number };
  lessons: RampViolation[];
  chapters: ChapterRampViolation[];
  tracks: TrackRampCoverage[];
  /** The script ramp — a second, independent curve. See the note at the top of this file. */
  script: ScriptRampReport;
  summary: {
    /** Lessons above `maxNewAtomsPerLesson`. The HL-C18 burn-down list. */
    lessonViolations: number;
    /** Chapters above `maxNewAtomsPerChapter`, so splitting cannot game the lesson rule. */
    chapterViolations: number;
    /** Lessons whose introduced-atom frontier is unknown — the measurement's blind spot, named. */
    unmeasurableLessons: number;
    /** Share of the corpus this measurement can actually see. */
    measurablePercent: number;
    /** The steepest single lesson, which is where a burn-down starts. */
    steepestLesson: RampViolation | null;
  };
}

export function frontmatterList(lesson: ParsedLesson, key: string): string[] {
  const value = lesson.frontmatter[key];
  if (Array.isArray(value)) return value.filter((item): item is string => typeof item === "string");
  return typeof value === "string" && value.trim() ? [value.trim()] : [];
}

/**
 * Atoms a lesson introduces — frontmatter and block directives unioned.
 *
 * The frontmatter key is FLAT and dotted (`introduces.knowledge`); reading it as a nested
 * object returns undefined for every lesson in the corpus. That mistake once made the
 * chapter gates report all 279 authored chapters as broken, so it is worth restating
 * wherever atoms are counted.
 */
export function introducedAtoms(lesson: ParsedLesson): string[] {
  const atoms = new Set(frontmatterList(lesson, "introduces.knowledge"));
  for (const block of lesson.blocks ?? []) {
    for (const atom of block.knowledge?.introduces ?? []) atoms.add(atom);
  }
  return [...atoms];
}

/**
 * Whether a zero-introduction lesson proves that zero is intentional.
 *
 * An empty atom set is ambiguous in legacy prose: it can mean "this lesson
 * introduces nothing" or "nobody has migrated its knowledge contract yet."
 * Schema-v2 review and practice lessons can remove that ambiguity by explicitly
 * declaring an empty introduction list and a non-empty practice list. Writing
 * retrieval may do the same only when its parsed stage evidence is narrow and
 * atom-aligned: guided copy, delayed copy, or dictation/transcription, never
 * composition presented as retrieval. Missing/malformed fields and other lesson
 * types remain fail-closed as measurement-blind.
 */
const WRITING_RETRIEVAL_STAGES = new Set([
  "guided-copy",
  "delayed-copy",
  "dictation-transcription",
]);

function hasExplicitWritingRetrievalContract(
  lesson: ParsedLesson,
  practises: readonly string[],
): boolean {
  if (lesson.frontmatter.type !== "writing") return false;
  if (lesson.blocks.some((block) => block.writingStageDirectiveError !== undefined)) return false;

  const stagedBlocks = lesson.blocks.filter((block) => block.writingStage !== undefined);
  if (stagedBlocks.length === 0) return false;
  if (stagedBlocks.some((block) => !WRITING_RETRIEVAL_STAGES.has(block.writingStage!))) return false;

  const assessed = new Set<string>();
  for (const block of stagedBlocks) {
    if (
      !block.knowledge ||
      block.knowledge.introduces.length > 0 ||
      block.knowledge.assesses.length === 0
    ) {
      return false;
    }
    for (const atom of block.knowledge.assesses) assessed.add(atom);
  }

  const practised = new Set(practises);
  return assessed.size === practised.size && [...assessed].every((atom) => practised.has(atom));
}

function isExplicitRetrievalOnlyLesson(lesson: ParsedLesson): boolean {
  const type = lesson.frontmatter.type;
  const introduces = lesson.frontmatter["introduces.knowledge"];
  const practises = frontmatterList(lesson, "practises.knowledge");
  return (
    lesson.frontmatter.schema_version === "2" &&
    Array.isArray(introduces) &&
    introduces.length === 0 &&
    practises.length > 0 &&
    (
      type === "review" ||
      type === "practice" ||
      type === "practice-mix" ||
      hasExplicitWritingRetrievalContract(lesson, practises)
    )
  );
}

/**
 * Measure the script ramp: new target-script glyphs per lesson, in reading order.
 *
 * Defaults mirror `chapter-policy.json` so a policy file written before this
 * existed still measures something rather than silently reporting zero — the
 * failure mode that let the atom budgets sit unread for a whole release.
 */
export function measureScriptRamp(
  lessons: ParsedLesson[],
  policy: ChapterPolicy,
): ScriptRampReport {
  const perLesson = policy.maxNewGlyphsPerLesson ?? 3;
  const perSystems = policy.maxNewScriptSystemsPerLesson ?? 1;

  const violations: ScriptRampViolation[] = [];
  const systemViolations: ScriptSystemViolation[] = [];
  const tracks: TrackScriptRamp[] = [];
  let maxForeign = 0;
  let foreignLessons = 0;

  const byTrack = new Map<string, ParsedLesson[]>();
  for (const lesson of lessons) {
    let group = byTrack.get(lesson.language);
    if (!group) byTrack.set(lesson.language, (group = []));
    group.push(lesson);
  }

  for (const [language, group] of [...byTrack].sort((a, b) => a[0].localeCompare(b[0]))) {
    const script = group[0]!.script;
    // `hasOwn`, not `?? ["Latin"]`. A track.json may declare any string as its script,
    // and `SCRIPT_SYSTEMS["__proto__"]` returns Object.prototype — which is not nullish,
    // so `??` never fires and `new Set(Object.prototype)` throws, taking the whole gap
    // report down with it. Same for `constructor`, `toString`, `valueOf`. The package
    // already exports `hasOwn` for exactly this; this lookup is the one that skipped it.
    const target = new Set(hasOwn(SCRIPT_SYSTEMS, script) ? SCRIPT_SYSTEMS[script]! : ["Latin"]);
    const latinScript = target.has("Latin");

    const seenTarget = new Set<string>();
    const seenForeign = new Set<string>();
    const track: TrackScriptRamp = {
      language,
      script,
      latinScript,
      lessonCount: group.length,
      totalGlyphs: 0,
      lessonViolations: 0,
      systemViolations: 0,
      lessonsWithForeignScript: 0,
    };

    for (const lesson of [...group].sort(readingOrder)) {
      const newTarget = new Set<string>();
      const newForeign = new Set<string>();
      const systems = new Set<string>();

      for (const ch of new Set(lesson.body)) {
        const system = systemOf(ch);
        if (system === null) continue;
        if (target.has(system)) {
          if (latinScript || seenTarget.has(ch)) continue;
          seenTarget.add(ch);
          newTarget.add(ch);
          systems.add(system);
        } else if (!seenForeign.has(ch)) {
          seenForeign.add(ch);
          newForeign.add(ch);
        }
      }

      if (newForeign.size > 0) {
        foreignLessons += 1;
        track.lessonsWithForeignScript += 1;
        maxForeign = Math.max(maxForeign, newForeign.size);
      }
      track.totalGlyphs += newTarget.size;

      const chapter =
        typeof lesson.realization.chapter === "number" && Number.isFinite(lesson.realization.chapter)
          ? lesson.realization.chapter
          : null;
      const orderedSystems = [...systems].sort();

      if (newTarget.size > perLesson) {
        violations.push({
          lessonId: lesson.realization.lessonId,
          language,
          chapter,
          glyphs: newTarget.size,
          sample: [...newTarget].sort().join(""),
          systems: orderedSystems,
          budget: perLesson,
        });
        track.lessonViolations += 1;
      }

      if (orderedSystems.length > perSystems) {
        systemViolations.push({
          lessonId: lesson.realization.lessonId,
          language,
          chapter,
          systems: orderedSystems,
          budget: perSystems,
        });
        track.systemViolations += 1;
      }
    }

    tracks.push(track);
  }

  // Steepest first, then by id, so the list is a stable work queue rather than a set.
  violations.sort((a, b) => b.glyphs - a.glyphs || a.lessonId.localeCompare(b.lessonId));
  systemViolations.sort(
    (a, b) => b.systems.length - a.systems.length || a.lessonId.localeCompare(b.lessonId),
  );

  return {
    policy: { maxNewGlyphsPerLesson: perLesson, maxNewScriptSystemsPerLesson: perSystems },
    lessons: violations,
    systems: systemViolations,
    tracks,
    summary: {
      lessonViolations: violations.length,
      systemViolations: systemViolations.length,
      lessonsWithForeignScript: foreignLessons,
      maxForeignGlyphsInALesson: maxForeign,
      steepestLesson: violations[0] ?? null,
    },
  };
}

/** Measure the gentle-ramp budgets across the corpus. */
export function measureRamp(lessons: ParsedLesson[], policy: ChapterPolicy): RampReport {
  const perLesson = policy.maxNewAtomsPerLesson;
  const perChapter = policy.maxNewAtomsPerChapter;

  const violations: RampViolation[] = [];
  const chapterAtoms = new Map<string, Set<string>>();
  const chapterLessons = new Map<string, number>();
  const tracks = new Map<string, TrackRampCoverage>();

  for (const lesson of lessons) {
    const language = lesson.language;
    const chapter = typeof lesson.realization?.chapter === "number" ? lesson.realization.chapter : null;
    const atoms = introducedAtoms(lesson);

    let track = tracks.get(language);
    if (!track) {
      track = {
        language,
        lessonCount: 0,
        measurable: 0,
        unmeasurable: 0,
        lessonViolations: 0,
        chapterViolations: 0,
      };
      tracks.set(language, track);
    }
    track.lessonCount += 1;
    if (atoms.length === 0 && !isExplicitRetrievalOnlyLesson(lesson)) track.unmeasurable += 1;
    else track.measurable += 1;

    if (atoms.length > perLesson) {
      violations.push({
        lessonId: lesson.realization.lessonId,
        language,
        chapter,
        atoms: atoms.length,
        budget: perLesson,
      });
      track.lessonViolations += 1;
    }

    if (chapter !== null) {
      const key = `${language}:${chapter}`;
      let set = chapterAtoms.get(key);
      if (!set) chapterAtoms.set(key, (set = new Set()));
      for (const atom of atoms) set.add(atom);
      chapterLessons.set(key, (chapterLessons.get(key) ?? 0) + 1);
    }
  }

  const chapters: ChapterRampViolation[] = [];
  for (const [key, atoms] of chapterAtoms) {
    if (atoms.size <= perChapter) continue;
    const [language, chapterText] = key.split(":");
    chapters.push({
      language: language!,
      chapter: Number(chapterText),
      atoms: atoms.size,
      budget: perChapter,
      lessonCount: chapterLessons.get(key) ?? 0,
    });
    const track = tracks.get(language!);
    if (track) track.chapterViolations += 1;
  }

  // Steepest first, then by id, so the list is a stable work queue rather than a set.
  violations.sort((a, b) => b.atoms - a.atoms || a.lessonId.localeCompare(b.lessonId));
  chapters.sort(
    (a, b) => b.atoms - a.atoms || a.language.localeCompare(b.language) || a.chapter - b.chapter,
  );

  const unmeasurable = [...tracks.values()].reduce((sum, track) => sum + track.unmeasurable, 0);
  return {
    policy: { maxNewAtomsPerLesson: perLesson, maxNewAtomsPerChapter: perChapter },
    lessons: violations,
    chapters,
    tracks: [...tracks.values()].sort((a, b) => a.language.localeCompare(b.language)),
    script: measureScriptRamp(lessons, policy),
    summary: {
      lessonViolations: violations.length,
      chapterViolations: chapters.length,
      unmeasurableLessons: unmeasurable,
      measurablePercent:
        lessons.length === 0 ? 0 : Math.round(((lessons.length - unmeasurable) / lessons.length) * 100),
      steepestLesson: violations[0] ?? null,
    },
  };
}
