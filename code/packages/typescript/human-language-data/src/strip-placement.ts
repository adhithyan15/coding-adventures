// strip-placement.ts — which block of a lesson page a stroke-order filmstrip
// goes in (HL-C443).
//
// ---------------------------------------------------------------------------
// Why this is a module of its own
// ---------------------------------------------------------------------------
//
// Two programs draw a lesson with its filmstrip: the printed book (through
// `withFilmstripImages` in figure-targets.ts) and the language-ladder app,
// which renders the same authored Markdown in a browser. They used to carry
// two copies of the placement rule, and the copies drifted. The book took the
// first Writing block and only then looked for a Script block; the app took
// whichever of the two came FIRST. An Arabic letter lesson is laid out
//
//   ## Warm-up
//   ## Script you'll notice: د      <- the app put the strip here
//   ## Writing: د                   <- the book put it here
//
// and 435 strip lessons showed their strip in a different section in the two.
//
// So the rule lives here, once, and both import it. The module has NO
// imports: figure-targets.ts reaches for `node:path`, which a browser bundle
// cannot load, and the app's first-paint chunk has a size budget. Everything
// below is a pure function of a lesson's level-two blocks, and of only the
// three things placement reads from each block: its heading, its declared
// writing stage, and the knowledge atoms it introduces.
//
// The functions take any `{ blocks }` of that shape. A `ParsedLesson` is one
// (figure-targets.ts passes the whole lesson), and so is
// `{ blocks: parseBodyBlocks(body).blocks }`, which is what the app builds
// from the authored body it already holds.

/** The parts of one level-two lesson block that decide where a strip goes. */
export interface StripPlacementBlock {
  /** The block's `## ` heading as authored, e.g. "Writing: د". */
  readonly title: string;
  /** The block's `<!-- hl-writing-stage: … -->` id (HL19), when it declares one. */
  readonly writingStage?: string;
  /** The block's `<!-- hl-knowledge: … -->` closure, when it declares one. */
  readonly knowledge?: { readonly introduces: readonly string[] };
}

/** A lesson as placement sees it: its level-two blocks, in authored order. */
export interface StripPlacementLesson {
  readonly blocks: readonly StripPlacementBlock[];
}

/** Does this block hold the lesson's writing instructions? */
function isWritingBlock(title: string): boolean {
  return /^Writing\b/.test(title.trim());
}

/** Does this block present the letter itself (the tracks that teach a letter under a `## Script` heading)? */
function isScriptBlock(title: string): boolean {
  return /^Script\b/.test(title.trim());
}

/**
 * Where a lesson's filmstrip belongs: its first Writing block, or, for the
 * tracks that present a letter under `## Script` instead (chinese, japanese,
 * urdu, persian and most russian letter lessons), its first Script block, or,
 * in a lesson with neither, its first practice block that shows the learner a
 * model (`modelledPracticeBlockIndex`); a block that declares a no-model stage
 * (a dictation, a composition) is never one of them (`stripBlockIndex`).
 * `-1` when the lesson has none of them.
 */
export function filmstripBlockIndex(lesson: StripPlacementLesson): number {
  const home = stripBlockIndex(lesson);
  if (home !== -1) return home;
  // A DECLARED target may sit on a lesson that teaches its letter inside a word
  // lesson (FA-C03-chist introduces چ in its first explanation block). There the
  // figure goes on the first block that introduces a script atom. Derived
  // candidates never reach this branch: every candidate needs a strip block.
  return lesson.blocks.findIndex((block) =>
    (block.knowledge?.introduces ?? []).some((atom) => /-SCRIPT-/.test(atom)),
  );
}

// ---------------------------------------------------------------------------
// A lesson with no Writing or Script block: the strip goes where the model is
// ---------------------------------------------------------------------------
//
// Many writing lessons are built from Warm-up / Guided Practice / Wrap-up
// Recall alone. The Chinese copy pair for each character (ZH-W16-han-guided,
// then ZH-W16-han-delayed) and the Gujarati place words (GU-C20-ghar) are
// examples: their headword is a letter or word whose every piece has a cited
// ductus, but with no Writing or Script block they printed no strip.
//
// They cannot all have one. The same block shape carries lessons whose whole
// point is that the learner sees NO model: a dictation ("hear woman and write
// 女 without a model"), a timed paper, a "select, do not copy" form card. A
// strip printed there would hand over the answer the lesson is testing.
//
// The corpus already says which is which, in data rather than prose. Every
// practice block that asks for writing carries a writing-stage directive
// (`<!-- hl-writing-stage: … -->`, HL19), whose ids are defined, each with a
// one-line description, in `core/assessment-policy.json`:
//
//   stage                         the learner …                      strip?
//   ----------------------------  ---------------------------------  ------
//   observe-trace                 traces with the model visible      yes
//   guided-copy                   copies beside the model            yes
//   delayed-copy                  looks at the model, hides it,      yes
//                                 writes, then uncovers and repairs
//   dictation-transcription       writes from sound, no model        no
//   controlled-composition        chooses and orders known language  no
//   connected-composition         writes connected sentences         no
//   timed-assessment-production   writes under exam timing           no
//
// Delayed copy is on the "yes" side on purpose. Its model is shown first and
// compared against last ("Study 汉 for five seconds. Cover it. … Reveal the
// model and repair"), so a book has to print one for the learner to cover.
// The strip is that model, with the route drawn in.
//
// A block with NO stage directive is never chosen, whatever its title says.
// That excludes, correctly, the Punjabi selector cards ("Guided Practice —
// decide before writing") and the Hindi concept lessons whose Guided
// Practice is a list of cues rather than a copy (HI-W02-abugida-ka-ta's
// headword is अ, a letter it never asks the learner to write).
//
// This is a FALLBACK. A lesson with a Writing or Script block keeps its strip
// there, and the fallback only ever adds lessons; it never moves a strip. (The
// same stage test does keep a strip out of a Writing or Script block that is
// itself a dictation or a composition: see the next section.)

/** Writing stages whose block shows the learner a model of what to write. */
export const MODELLED_WRITING_STAGES: ReadonlySet<string> = new Set([
  "observe-trace",
  "guided-copy",
  "delayed-copy",
]);

/** The first block whose writing stage shows a model (above); `-1` when none does. */
export function modelledPracticeBlockIndex(lesson: StripPlacementLesson): number {
  return lesson.blocks.findIndex(
    (block) => block.writingStage !== undefined && MODELLED_WRITING_STAGES.has(block.writingStage),
  );
}

/**
 * The block that TEACHES a lesson's letter: its first Writing block, else its
 * first Script block; `-1` when neither. `letter-anchoring.ts` counts exactly
 * these lessons as letter lessons, whether or not the block prints a strip
 * (a single-letter dictation's does not; see `stripBlockIndex`). A copy or
 * delayed-copy lesson that gets its strip from the fallback above practises a
 * letter some earlier lesson taught, and is not counted there.
 */
export function letterBlockIndex(lesson: StripPlacementLesson): number {
  const writing = lesson.blocks.findIndex((block) => isWritingBlock(block.title));
  if (writing !== -1) return writing;
  return lesson.blocks.findIndex((block) => isScriptBlock(block.title));
}

// ---------------------------------------------------------------------------
// A Writing block that is a dictation is not a home for the strip either
// ---------------------------------------------------------------------------
//
// The stage rule above was first applied only to the fallback. A Writing or
// Script block took the strip whatever stage it declared, and 37 lessons
// declare a no-model stage on their Writing block: the short dictations that
// close each track's first writing runway (ES-W00-hola-dictation,
// SA-S02-dictation, GU-W01-haa-dictation …), the Gujarati spaced-return
// lessons' "Writing — from sound", the Marathi "Writing — heard cue", and the
// four-line connected compositions. The book printed the strip at the TOP of
// that block, under its heading and above the cue:
//
//   Writing — short dictation
//   [strip: How hola is written, letter by letter]
//   Hear: OH-la
//   Write the Spanish greeting from that sound alone.
//
// That is the answer, drawn large, above the question. SA-S02-dictation's
// Warm-up even says "no stroke order in front of you" a few lines above it.
// The check these lessons do have ("Now uncover and compare: hola") is a line
// of the same block, after the attempt, not a block of its own, so there is no
// later block the strip could move to.
//
// So a block whose declared stage shows no model is skipped wherever the strip
// is looked for. A Marathi letter lesson (MR-W03-ba) then finds its Script
// block, which prints the letter and describes its strokes, and which the
// dictation tells the learner to cover: the strip moves up to the model it
// belongs to. A lesson with no other home prints no strip; its earlier copy
// lessons already printed one (ES-W00-hola-delayed-copy, SA-S02-letter-na).

/** Does this block declare a writing stage in which the learner sees no model? */
function declaresNoModel(block: StripPlacementBlock): boolean {
  return block.writingStage !== undefined && !MODELLED_WRITING_STAGES.has(block.writingStage);
}

/**
 * The block a writing lesson's strip lands in: its first Writing block, else
 * its first Script block, else its first modelled practice block, skipping
 * any block whose declared stage shows no model (above); `-1` when none is
 * left, and then the lesson is not a filmstrip candidate.
 */
export function stripBlockIndex(lesson: StripPlacementLesson): number {
  const writing = lesson.blocks.findIndex((block) => isWritingBlock(block.title) && !declaresNoModel(block));
  if (writing !== -1) return writing;
  const script = lesson.blocks.findIndex((block) => isScriptBlock(block.title) && !declaresNoModel(block));
  if (script !== -1) return script;
  return modelledPracticeBlockIndex(lesson);
}
