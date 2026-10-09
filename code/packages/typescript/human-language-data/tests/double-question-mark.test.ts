// No doubled question mark in learner-facing prose.
//
// ---------------------------------------------------------------------------
// Where "??" came from
// ---------------------------------------------------------------------------
//
// Review lessons quiz a run of glosses as spoken prompts: "To dare?
// (**audeō**.)". The prompt is the gloss plus a question mark. When the gloss
// is itself a question word — "Why?", "Which?", "How much?" — the template
// adds a second one, and the learner reads "Why?? (**quārē**.)".
//
// Five lessons carried it (LA-R119, FA-R88, FA-R98, FA-R139, SA-R190). Each
// now names what is being asked for ("Asking why?") so the prompt reads as a
// question about a question word, with one mark.
//
// ---------------------------------------------------------------------------
// Why this is an assertion of zero
// ---------------------------------------------------------------------------
//
// Unlike the banned-word ceiling, the corpus is already clean, so `toEqual([])`
// costs nothing today and catches the template the next time it meets a
// question-word gloss. "??" is never correct in any of the 23 tracks:
//
//   input                         counts?   why
//   ----------------------------  -------   ----------------------------------
//   "Why?? (**quārē**.)"          yes       doubled mark, the bug above
//   "¿Dónde? (**where**)"         no        one mark; Spanish's opener is ¿
//   "Comment ? (**how**)"         no        French spacing puts one mark apart
//   "?" standing in for script    yes       mojibake: lost non-Latin text was
//   ("?? ??")                               once authored this way; it must
//                                           never come back silently
//
// Frontmatter and `hl-*` directives are not learner-facing and are excluded:
// `block.markdown` already has the directive removed, and frontmatter never
// reaches it.

import { describe, it, expect } from "vitest";
import { loadEverything } from "../src/loader.js";

const { lessons } = loadEverything();

/** The prose a learner sees: every block body, with fenced code removed. */
function proseOf(lesson: (typeof lessons)[number]): string {
  return [lesson.preamble, ...lesson.blocks.map((block) => block.markdown ?? "")]
    .join("\n")
    .replace(/^```[\s\S]*?^```/gm, "");
}

/**
 * Every doubled question mark in `prose`, each with a little context either
 * side so a failure names the sentence, not just the lesson.
 */
function doubledQuestionMarks(prose: string): string[] {
  const hits: string[] = [];
  for (const match of prose.matchAll(/\?{2,}/g)) {
    const at = match.index ?? 0;
    hits.push(prose.slice(Math.max(0, at - 40), at + 20).replace(/\s+/g, " "));
  }
  return hits;
}

describe("learner-facing prose never doubles a question mark", () => {
  it("no lesson prints '??'", () => {
    const problems: string[] = [];
    for (const lesson of lessons) {
      for (const hit of doubledQuestionMarks(proseOf(lesson))) {
        problems.push(`${lesson.realization.lessonId}: "${hit}"`);
      }
    }
    expect(problems, problems.join("\n")).toEqual([]);
  });

  it("fires on the shapes it was written for, and not on single marks", () => {
    // Anti-vacuity: the check above passes on a clean corpus, which is also
    // what a detector that never looked would do. Prove it fires on the exact
    // shapes it replaced, and stays quiet on the legitimate ones in the table.
    expect(doubledQuestionMarks("To dare? (**audeō**.) Why?? (**quārē**.)")).toHaveLength(1);
    expect(doubledQuestionMarks("?? ??")).toHaveLength(2);
    expect(doubledQuestionMarks("Asking why? (**quārē**.)")).toEqual([]);
    expect(doubledQuestionMarks("¿Dónde? Comment ? Why?")).toEqual([]);
  });
});
