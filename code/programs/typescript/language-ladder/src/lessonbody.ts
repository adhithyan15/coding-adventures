// A deliberately small Markdown-to-view-model parser for authored lesson bodies.
// We preserve the source in `Lesson.body`; this layer only extracts readable
// sections for the DOM without injecting HTML.

import { parseBodyBlocks } from "@coding-adventures/human-language-data/src/parse.ts";
import { filmstripBlockIndex } from "@coding-adventures/human-language-data/src/strip-placement.ts";

export interface LessonSection {
  title: string;
  blocks: LessonViewBlock[];
  /**
   * The section's writing stage, from its `<!-- hl-writing-stage: … -->`
   * directive (HL19), when it declares one. The directive itself is metadata,
   * never learner copy.
   */
  writingStage?: string;
  /**
   * Which of the book's blocks this section is: the 0-based count of the
   * `## ` heading that opened it, the index `parseBodyBlocks` gives the same
   * block. Absent for the preamble, the text before the first heading, which
   * the book does not count as a block. Used to place the filmstrip where the
   * book does (`filmstripSectionIndex`).
   */
  blockIndex?: number;
}

export type LessonViewBlock =
  | { kind: "text"; text: string }
  | { kind: "image"; alt: string; source: string };

// Each bracket class also excludes its own opener: link text stops at the next
// "[" and a target stops at the next "(". Without that, a paragraph of unclosed
// "[" (or of "[a](" with no ")") made every opener scan to the end of the text,
// O(n²) -- about two seconds on 50,000 characters. With it, each character is
// scanned from at most one opener, so the pass is linear. The cost is that a
// nested "[" inside link text, or a "(" inside a target, leaves that link
// unstripped; no lesson writes either.
function plainInline(markdown: string): string {
  return markdown
    .replace(/!\[([^\][]*)\]\([^()]*\)/g, "$1")
    .replace(/\[([^\][]+)\]\([^()]*\)/g, "$1")
    .replace(/[*_`]/g, "")
    .replace(/^>\s?/, "")
    .trim();
}

export function lessonSections(markdown: string): LessonSection[] {
  const sections: LessonSection[] = [];
  let current: LessonSection = { title: "Lesson", blocks: [] };
  let paragraph: string[] = [];
  // How many of the BOOK's blocks have opened so far. `parseBodyBlocks` opens
  // one at every line that starts (after indentation) with "## ", and that
  // is what is counted here, rather than this view's own, looser heading
  // pattern below: a "##<tab>Title" line is a section here but not a book
  // block, and must not shift every later section's `blockIndex` by one.
  let bookBlocks = 0;

  const flushParagraph = () => {
    if (paragraph.length === 0) return;
    current.blocks.push({ kind: "text", text: plainInline(paragraph.join(" ")) });
    paragraph = [];
  };
  const flushSection = () => {
    flushParagraph();
    if (current.blocks.length > 0) sections.push(current);
  };

  for (const raw of markdown.replace(/\r\n/g, "\n").split("\n")) {
    const line = raw.trim();
    const opensBookBlock = raw.trimStart().startsWith("## ");
    if (opensBookBlock) bookBlocks += 1;
    if (/^#\s+/.test(line)) continue; // card already displays the lesson title
    if (/^<!--\s*hl-(?:knowledge|activity):/.test(line)) continue; // canonical AST metadata, not learner copy
    const stage = /^<!--\s*hl-writing-stage:\s*([a-z][a-z0-9-]*)\s*-->$/.exec(line);
    if (stage) {
      current.writingStage = stage[1]!;
      continue;
    }
    if (/^<!--\s*hl-writing-stage:/.test(line)) continue; // malformed: the validator reports it
    const heading = /^##\s+(.+)$/.exec(line);
    const image = /^!\[([^\]]+)\]\(([^)]+)\)$/.exec(line);
    if (heading) {
      flushSection();
      current = { title: plainInline(heading[1]!), blocks: [] };
      if (opensBookBlock) current.blockIndex = bookBlocks - 1;
    } else if (image) {
      flushParagraph();
      current.blocks.push({ kind: "image", alt: plainInline(image[1]!), source: image[2]! });
    } else if (line === "") {
      flushParagraph();
    } else if (/^[-*]\s+/.test(line)) {
      flushParagraph();
      current.blocks.push({
        kind: "text",
        text: `• ${plainInline(line.replace(/^[-*]\s+/, ""))}`,
      });
    } else {
      paragraph.push(line);
    }
  }
  flushSection();
  return sections;
}

// ---------------------------------------------------------------------------
// Which section a lesson's stroke-order filmstrip goes in
// ---------------------------------------------------------------------------
//
// Wherever the BOOK puts it. The rule is not restated here: the book's own
// `filmstripBlockIndex` (human-language-data's strip-placement.ts) is run on
// the book's own parse of this lesson's body (`parseBodyBlocks`), and its
// answer is translated into one of this view's sections. The rule, in short:
//
//   1. the first Writing block, else
//   2. the first Script block, else
//   3. the first block whose writing stage SHOWS a model (trace, guided
//      copy, delayed copy), else
//   4. the first block that introduces a script atom (the book's one
//      declared strip on a word lesson, FA-C03-chist, which main.ts does
//      not load: see the note there),
//
// skipping, in 1 to 3, any block whose declared stage shows NO model: a
// dictation, a composition, a timed task, where a strip would hand over the
// answer the lesson is testing.
//
// Steps 1 and 2 are where the app used to differ. It took the first Writing
// OR Script section, whichever came first, and many letter lessons put their
// Script section above their Writing one (AR-W46-dal: "Script you'll notice:
// د", then "Writing: د"; most are in the Indic tracks): 435 strips sat
// higher in the app than on the printed page. Running the book's code, not a
// copy of it, is what stops the two drifting apart again.
//
// Translating a block index into a section index needs `blockIndex` above: a
// lesson's preamble (text before its first heading) is a section here but not
// a book block, and a heading with nothing under it is a book block but no
// section here. A cross-check over the whole corpus (tests/lessonbody.test.ts)
// holds the translation to the book for every lesson that has a strip.

/**
 * The index of the section a filmstrip belongs in, or -1 when the book would
 * place none. `body` is the authored lesson Markdown that `sections` was built
 * from (`lessonSections(body)`).
 */
export function filmstripSectionIndex(body: string, sections: readonly LessonSection[]): number {
  const at = filmstripBlockIndex({ blocks: parseBodyBlocks(body).blocks });
  if (at === -1) return -1;
  return sections.findIndex((section) => section.blockIndex === at);
}
