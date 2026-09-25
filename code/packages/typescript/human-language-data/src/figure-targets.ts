// figure-targets.ts — which lessons get a stroke-order filmstrip (HL-C443).
//
// ---------------------------------------------------------------------------
// The problem this closes
// ---------------------------------------------------------------------------
//
// The filmstrip renderer shipped (HL-C300) with three proof targets declared by
// hand in `core/figure-generation.json`, and no lesson referenced any of them.
// So no printed book showed a single filmstrip, while about 380 writing lessons
// already had a CITED stroke order in `script-ductus`. The drawing was never the
// gap. Getting a figure into a book cost three manual steps per letter, and
// nobody took them.
//
// This module removes the manual steps for the case that is always the same: a
// `type: writing` lesson whose headword is ONE letter, with a `## Writing:`
// block, on a track that has been switched on below. Such a lesson is a
// candidate. The candidate becomes a figure only if `script-ductus` holds a
// cited ductus for that letter, and the generated filmstrip ledger is the record
// of that. HL11 §5.2 still holds: no citation, no pen path, no figure. Nothing
// here can draw a letter the ductus data does not source.
//
// ---------------------------------------------------------------------------
// Why deriving from the headword is safe here
// ---------------------------------------------------------------------------
//
// `ScriptFilmstripTarget` names its glyph explicitly. The worry was that a
// figure which followed a headword edit would "redraw itself the next time
// somebody fixed a typo". For a derived target that redraw is LOUD rather than
// silent:
//
//   * the filmstrip ledger is byte-checked by `check:filmstrip-ledger`;
//   * every SVG is hashed in `core/generated-figure-hashes.json`;
//   * every book chapter is hashed as well.
//
// So a headword edit fails three checks until someone regenerates, and the
// regenerated figure is the letter the lesson now teaches, which is the right
// one. An explicit target in the config still wins over a derived one for the
// same lesson, for the lesson that deliberately prints a letter it is not
// named after.
//
// ---------------------------------------------------------------------------
// One track at a time
// ---------------------------------------------------------------------------
//
// `DERIVED_FILMSTRIP_SCRIPTS` is an allowlist, not a heuristic. Each track that
// joins adds pages of figures to its book, and only a real XeLaTeX build (CI's
// books gate) can say the pages still lay out. So tracks join one PR at a time.

import { basename } from "node:path";
import type { FigureTarget, ScriptFilmstripTarget } from "./figure.js";
import type { ParsedLesson } from "./parse.js";

/**
 * Track -> the `script-ductus` script id its writing lessons are drawn in.
 *
 * Tamil goes first because it is the only script whose ductus is sharded one
 * cited file per glyph, and its single-letter lessons (`TA-S01` onward) are the
 * clearest one-letter-at-a-time ramp in the corpus.
 */
export const DERIVED_FILMSTRIP_SCRIPTS: Readonly<Record<string, string>> = {
  tamil: "tamil",
  // HL-C443 second rollout: the four Devanagari tracks share one cited ductus.
  hindi: "devanagari",
  marathi: "devanagari",
  sanskrit: "devanagari",
  marwadi: "devanagari",
  // HL-C443 third rollout: every other track with any cited ductus. A track
  // whose letters are mostly uncited (kannada, malayalam, telugu: vowels and
  // chillus only) still gets exactly the letters that ARE cited, no more.
  gujarati: "gujarati",
  arabic: "arabic",
  persian: "perso-arabic",
  urdu: "urdu-nastaliq",
  russian: "cyrillic",
  chinese: "chinese",
  japanese: "japanese",
  kannada: "kannada",
  malayalam: "malayalam",
  telugu: "telugu",
};

const GRAPHEMES = new Intl.Segmenter("und", { granularity: "grapheme" });

/** Does this block hold the lesson's writing instructions? */
function isWritingBlock(title: string): boolean {
  return /^Writing\b/.test(title.trim());
}

/**
 * The one letter a writing lesson teaches, or `undefined` when the lesson is
 * not a single-letter writing lesson with a Writing block. A headword like
 * "வ, க" or "வணக்கம்" teaches more than one letter and is left to an explicit
 * target (and to the combinations work HL-C443 lists next).
 */
export function writingLetterOf(lesson: ParsedLesson): string | undefined {
  if (lesson.realization.type !== "writing") return undefined;
  const headword = (lesson.realization.headword ?? "").trim();
  if (headword === "" || [...GRAPHEMES.segment(headword)].length !== 1) return undefined;
  if (!lesson.blocks.some((block) => isWritingBlock(block.title))) return undefined;
  return headword;
}

/** Every lesson on a switched-on track that COULD carry a filmstrip. */
export function filmstripCandidates(
  lessons: readonly ParsedLesson[],
  scripts: Readonly<Record<string, string>> = DERIVED_FILMSTRIP_SCRIPTS,
): ScriptFilmstripTarget[] {
  const candidates: ScriptFilmstripTarget[] = [];
  for (const lesson of lessons) {
    const script = scripts[lesson.language];
    if (script === undefined) continue;
    const glyph = writingLetterOf(lesson);
    if (glyph === undefined) continue;
    const lessonId = lesson.realization.lessonId;
    candidates.push({
      kind: "script-filmstrip",
      lessonId,
      script,
      glyph,
      output: `${lesson.language}/book/figures/${lessonId}-filmstrip.svg`,
    });
  }
  return candidates.sort((left, right) => left.lessonId.localeCompare(right.lessonId));
}

/**
 * The explicit targets plus every candidate whose letter has a cited ductus.
 *
 * `hasDuctus` is the caller's source of truth for "cited": `script-ductus`
 * asks its own stroke registry, and the figure and book generators ask the
 * generated filmstrip ledger, which `script-ductus` writes from that registry.
 */
export function withDerivedFilmstrips(
  explicit: readonly FigureTarget[],
  candidates: readonly ScriptFilmstripTarget[],
  hasDuctus: (script: string, glyph: string) => boolean,
): FigureTarget[] {
  const declared = new Set(
    explicit
      .filter((target) => target.kind === "script-filmstrip")
      .map((target) => target.lessonId),
  );
  return [
    ...explicit,
    ...candidates.filter(
      (candidate) => !declared.has(candidate.lessonId) && hasDuctus(candidate.script, candidate.glyph),
    ),
  ];
}

/** The Markdown image a book prints for a filmstrip target. */
export function filmstripImageMarkdown(target: ScriptFilmstripTarget): string {
  return `![How ${target.glyph} is written, stroke by stroke](figures/${basename(target.output)})`;
}

/**
 * Lessons as the BOOK sees them: each filmstrip target's image placed at the
 * top of its lesson's first Writing block, above the numbered strokes it draws.
 *
 * Only the book gets this view. Narration and the app keep the authored lesson,
 * because a listener cannot see a figure. A lesson that already references its
 * figure somewhere is left alone, so an author can still place one by hand.
 */
export function withFilmstripImages(
  lessons: readonly ParsedLesson[],
  targets: readonly FigureTarget[],
): ParsedLesson[] {
  const byLesson = new Map<string, ScriptFilmstripTarget>();
  for (const target of targets) {
    if (target.kind === "script-filmstrip") byLesson.set(target.lessonId, target);
  }
  return lessons.map((lesson) => {
    const target = byLesson.get(lesson.realization.lessonId);
    if (target === undefined) return lesson;
    const file = `figures/${basename(target.output)}`;
    if (lesson.blocks.some((block) => block.markdown.includes(file))) return lesson;
    const at = lesson.blocks.findIndex((block) => isWritingBlock(block.title));
    if (at === -1) return lesson;
    const blocks = lesson.blocks.map((block, index) =>
      index === at
        ? { ...block, markdown: `${filmstripImageMarkdown(target)}\n\n${block.markdown.replace(/^\s+/, "")}` }
        : block,
    );
    return { ...lesson, blocks };
  });
}
