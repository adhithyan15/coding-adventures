import { canonicalChapterHash } from "./hash.js";
import { CONTENT_TYPES, hasOwn } from "./constants.js";
import { compileLessonActivities } from "./activity.js";
import type {
  LessonBodyBlock,
  ChapterCapability,
  CompiledLessonActivity,
} from "./types.js";
import { stripHtmlComments } from "./literal-markup.js";
import {
  closingBracket,
  joinQualifier,
  MAX_CUE_LENGTH,
  opensDeliveryCue,
  parseDeliveryCue,
  type PromptCue,
} from "./delivery-cue.js";
import type { ParsedLesson } from "./parse.js";
import type { ChapterModality } from "./modality.js";

export interface BookGenerationTarget {
  language: string;
  chapter: number;
  title: string;
  label: string;
  output: string;
  /** Unicode Script property whose runs need the book's dedicated font command. */
  unicodeScript?: string;
  /** LaTeX command name, without the leading backslash, used for those runs. */
  scriptCommand?: string;
  /** Multiple script/font mappings for lessons that compare writing systems inline. */
  inlineScripts?: InlineRenderOptions[];
}

const BOOK_MODALITY_SIGNS: Readonly<Record<"voice" | "sight" | "pen", string>> = {
  voice: "\\hlvoicesign{}",
  sight: "\\hlsightsign{}",
  pen: "\\hlpensign{}",
};

/**
 * The words after "Hands-free start:" in a chapter's modality line.
 *
 * A phrase built for many lessons ("first 1 of 2 lessons", "all 1 lesson",
 * "none of the 1 lesson") reads like a form letter at one and two, so the small
 * counts get their own words:
 *
 *   drivable / lessons   prints
 *   ------------------   -------------------------
 *   1 / 1                its only lesson
 *   0 / 1                none (1 lesson)
 *   2 / 2                both lessons
 *   0 / 2                neither of the 2 lessons
 *   n / n   (n > 2)      all n lessons
 *   0 / n   (n > 2)      none of the n lessons
 *   1 / n   (n > 1)      the first of n lessons
 *   k / n   (1 < k < n)  the first k of n lessons
 *
 * Exported for the test that walks every row of that table.
 */
export function handsFreeStart(drivablePrefix: number, lessonCount: number): string {
  if (lessonCount === 1) return drivablePrefix === 1 ? "its only lesson" : "none (1 lesson)";
  if (drivablePrefix === lessonCount) {
    return lessonCount === 2 ? "both lessons" : `all ${lessonCount} lessons`;
  }
  if (drivablePrefix === 0) {
    return lessonCount === 2 ? "neither of the 2 lessons" : `none of the ${lessonCount} lessons`;
  }
  if (drivablePrefix === 1) return `the first of ${lessonCount} lessons`;
  return `the first ${drivablePrefix} of ${lessonCount} lessons`;
}

/**
 * Render one track's chapter-modality projection.
 *
 * The file is loaded once, immediately after `\\mainmatter`, and every numbered
 * chapter calls `\\hlchaptermodality` immediately after its title and label. Generated
 * chapters receive that call from this renderer; protected handwritten chapters keep
 * the same one-line projection marker beside their authored opening. This avoids
 * patching LaTeX's heavily wrapped `\\chapter` command or moving learner content.
 * The tiny signs are TikZ paths instead of Unicode emoji: all 22 preambles already load
 * tcolorbox/TikZ, while the repository deliberately carries no emoji font and the book
 * warning gate treats missing glyphs as a regression. The adjacent words remain the
 * accessible, unambiguous label.
 */
export function renderBookChapterModalities(
  language: string,
  chapters: readonly ChapterModality[],
): string {
  if (chapters.length === 0) throw new Error(`${language}: no chapter modality data`);
  const seen = new Set<number>();
  const definitions: string[] = [];
  for (const chapter of [...chapters].sort((left, right) => left.chapter - right.chapter)) {
    if (chapter.language !== language) {
      throw new Error(
        `${language}: chapter modality belongs to ${chapter.language} chapter ${chapter.chapter}`,
      );
    }
    if (!Number.isInteger(chapter.chapter) || chapter.chapter <= 0 || seen.has(chapter.chapter)) {
      throw new Error(`${language}: duplicate or invalid chapter modality ${chapter.chapter}`);
    }
    seen.add(chapter.chapter);
    if (chapter.lessonCount <= 0 || chapter.drivablePrefix > chapter.lessonCount) {
      throw new Error(`${language} chapter ${chapter.chapter}: invalid modality counts`);
    }
    const modes = (["voice", "sight", "pen"] as const)
      .filter((mode) => chapter[mode] > 0)
      .map((mode) => {
        const label = mode === "sight" ? "eyes" : mode;
        return `${BOOK_MODALITY_SIGNS[mode]}~\\textbf{${label}} (${chapter[mode]})`;
      })
      .join(" \\quad ");
    const prefix = handsFreeStart(chapter.drivablePrefix, chapter.lessonCount);
    definitions.push(
      `\\expandafter\\def\\csname hlchaptermodality${chapter.chapter}\\endcsname{%`,
      "  {\\small\\noindent",
      `    \\textbf{Modes:} ${modes}\\quad`,
      `    ${BOOK_MODALITY_SIGNS.voice}~\\textbf{Hands-free start:} ${prefix}.%`,
      "    \\par}\\medskip",
      "}",
      "",
    );
  }
  return [
    "% GENERATED FILE. Edit canonical lessons, then run npm run generate:books.",
    "% Full modes use the whole printed lesson; the hands-free prefix uses its voice core.",
    "",
    "% Font-independent modality signs, drawn from paths so every PDF gets the same glyphs.",
    "\\newcommand{\\hlvoicesign}{%",
    "  \\tikz[baseline=-0.55ex,x=0.11em,y=0.11em,line width=0.45pt]{%",
    "    \\draw[rounded corners=0.5] (0,1) rectangle (8,4);%",
    "    \\draw (1.5,4) -- (2.7,6) -- (5.8,6) -- (7,4);%",
    "    \\fill (2,0.7) circle (0.8); \\fill (6,0.7) circle (0.8);%",
    "  }%",
    "}",
    "\\newcommand{\\hlsightsign}{%",
    "  \\tikz[baseline=-0.55ex,x=0.11em,y=0.11em,line width=0.45pt]{%",
    "    \\draw (0,3) .. controls (2,6) and (6,6) .. (8,3)%",
    "      .. controls (6,0) and (2,0) .. cycle;%",
    "    \\fill (4,3) circle (1.05);%",
    "  }%",
    "}",
    "\\newcommand{\\hlpensign}{%",
    "  \\tikz[baseline=-0.55ex,x=0.11em,y=0.11em,line width=0.7pt]{%",
    "    \\draw (1,1) -- (7,7); \\draw (0.5,0.5) -- (2.4,1.1) -- (1.1,2.4) -- cycle;%",
    "  }%",
    "}",
    "",
    "\\newcommand{\\hlchaptermodality}[1]{%",
    "  \\ifcsname hlchaptermodality#1\\endcsname",
    "    \\csname hlchaptermodality#1\\endcsname",
    "  \\fi",
    "}",
    "",
    ...definitions,
  ].join("\n");
}

/** A generated back-matter reference sourced from canonical Markdown. */
export interface BookReferenceAppendixTarget {
  language: string;
  title: string;
  source: string;
  output: string;
  /** Table-of-contents line, when the chapter title is too long to sit on one. */
  shortTitle?: string;
  /** Running head, when `Pronunciation` misdescribes what the reference is about. */
  runningHead?: string;
  /** Unicode Script property whose runs need the book's dedicated font command. */
  unicodeScript?: string;
  /** LaTeX command name, without the leading backslash, used for those runs. */
  scriptCommand?: string;
  /** Multiple script/font mappings for references that compare writing systems inline. */
  inlineScripts?: InlineRenderOptions[];
}

/** A generated book glossary derived from canonical word and phrase lessons. */
export interface BookGlossaryTarget {
  language: string;
  output: string;
  /** Unicode Script property whose runs need the book's dedicated font command. */
  unicodeScript?: string;
  /** LaTeX command name, without the leading backslash, used for those runs. */
  scriptCommand?: string;
  /** Multiple script/font mappings for entries that compare writing systems inline. */
  inlineScripts?: InlineRenderOptions[];
}

/** Generated review questions and answers sourced from executable lesson activities. */
export interface BookAnswerKeyTarget {
  language: string;
  output: string;
  /** Unicode Script property whose runs need the book's dedicated font command. */
  unicodeScript?: string;
  /** LaTeX command name, without the leading backslash, used for those runs. */
  scriptCommand?: string;
  /** Multiple script/font mappings for entries that compare writing systems inline. */
  inlineScripts?: InlineRenderOptions[];
}

/** A generated subject index derived from canonical lessons and chapter capabilities. */
export interface BookIndexTarget {
  language: string;
  output: string;
  /** Unicode Script property whose runs need the book's dedicated font command. */
  unicodeScript?: string;
  /** LaTeX command name, without the leading backslash, used for those runs. */
  scriptCommand?: string;
  /** Multiple script/font mappings for entries that compare writing systems inline. */
  inlineScripts?: InlineRenderOptions[];
}

/** The stable title/label subset shared by generated and handwritten chapters. */
export interface BookIndexChapter {
  chapter: number;
  title: string;
  label: string;
}

export interface InlineRenderOptions {
  unicodeScript: string;
  scriptCommand: string;
}

type InlineRenderOptionsInput = InlineRenderOptions | readonly InlineRenderOptions[];

export interface GeneratedBookChapter {
  tex: string;
  sourceHash: string;
  lessonIds: string[];
}

function stringValue(value: ParsedLesson["frontmatter"][string] | undefined): string {
  return typeof value === "string" ? value : "";
}

function escapeLatexCharacter(character: string): string {
  const escaped: Record<string, string> = {
    "\\": "\\textbackslash{}",
    "{": "\\{",
    "}": "\\}",
    "%": "\\%",
    "$": "\\$",
    "#": "\\#",
    "_": "\\_",
    "&": "\\&",
    "~": "\\textasciitilde{}",
    "^": "\\textasciicircum{}",
    "[": "{[}",
    "]": "{]}",
    "←": "$\\leftarrow$",
    "→": "$\\to$",
    "↔": "$\\leftrightarrow$",
    "≈": "$\\approx$",
    "≠": "$\\neq$",
    "ṓ": "\\'{\\={o}}",
    "ḗ": "\\'{\\={e}}",
    "ḯ": "\\'{\\={\\i}}",
    "ḱ": "\\'{k}",
    "₁": "\\textsubscript{1}",
    "₂": "\\textsubscript{2}",
    "₃": "\\textsubscript{3}",
    "ʰ": "\\textsuperscript{h}",
    "ʷ": "\\textsuperscript{w}",
    "ⁿ": "\\textsuperscript{n}",
  };
  return escaped[character] ?? character;
}

/**
 * Decide what a Markdown link destination means to a *printed* book.
 *
 * The book is a standalone artefact. Somebody who downloads the PDF has the
 * PDF and nothing else — no checkout, no app, no network guarantee — so the
 * only links worth printing are ones that still work from an armchair:
 *
 *   destination                                what the book does
 *   ---------------------------------------    -----------------------------
 *   https://sites.la.utexas.edu/...            keep it. A dictionary or a
 *   https://en.wiktionary.org/...              reference grammar is ordinary
 *                                              scholarly apparatus, and a
 *                                              real book cites its sources.
 *
 *   ./ES-C01-bien.md                           relative — it points at a file
 *   ../pronunciation-reference.md              in the curriculum repository.
 *                                              Return `undefined`: the caller
 *                                              prints the label as plain text
 *                                              and drops the link. "bien /
 *                                              bueno --- the adjective bueno"
 *                                              still reads correctly; a URL
 *                                              the reader cannot follow does
 *                                              not.
 *
 *   mailto:...                                 throw. Neither of the above,
 *                                              so somebody authored something
 *                                              the book has no policy for.
 *
 * Returns the absolute URL to link, or `undefined` when the destination is
 * relative and the label should be printed unlinked.
 */
function absoluteBookLink(destination: string): string | undefined {
  const trimmed = destination.trim();
  if (trimmed === "") throw new Error("Markdown link destination must not be empty");
  let resolved: URL;
  try {
    resolved = new URL(trimmed);
  } catch {
    return undefined;
  }
  if (resolved.protocol !== "https:" && resolved.protocol !== "http:") {
    throw new Error(`unsupported Markdown link protocol '${resolved.protocol}'`);
  }
  return resolved.href;
}

/** Escape URL characters that TeX would otherwise interpret inside `\href`'s first argument. */
function escapeLatexLinkDestination(destination: string): string {
  const escaped: Record<string, string> = {
    "\\": "\\%5C",
    "{": "\\%7B",
    "}": "\\%7D",
    "$": "\\%24",
    "^": "\\%5E",
    "~": "\\%7E",
    "%": "\\%",
    "#": "\\#",
    "_": "\\_",
    "&": "\\&",
  };
  return destination.replace(/[\\{}$^~%#_&]/g, (character) => escaped[character] ?? character);
}

/**
 * Validate a local image path for a standalone book, then point SVG sources at
 * the PDF produced by the build's deterministic rsvg-convert step.
 */
export function bookImageDestination(destination: string): string {
  const trimmed = destination.trim().replaceAll("\\", "/");
  if (
    trimmed === "" ||
    trimmed.startsWith("/") ||
    /^[A-Za-z][A-Za-z0-9+.-]*:/.test(trimmed) ||
    trimmed.split("/").includes("..") ||
    !/^[A-Za-z0-9._/-]+\.(?:svg|pdf|png|jpe?g)$/.test(trimmed)
  ) {
    throw new Error(`unsafe Markdown image destination '${destination}'`);
  }
  return trimmed.endsWith(".svg") ? `${trimmed.slice(0, -4)}.pdf` : trimmed;
}

/** Image paths are allowlisted above, so detokenize is safe and keeps `_` literal. */
function escapeLatexImageDestination(destination: string): string {
  return `\\detokenize{${bookImageDestination(destination)}}`;
}

function markdownImageAt(markdown: string, cursor: number): {
  alt: string;
  destination: string;
  end: number;
} | undefined {
  if (!markdown.startsWith("![", cursor)) return undefined;
  const altEnd = markdown.indexOf("](", cursor + 2);
  const destinationEnd = altEnd === -1 ? -1 : markdown.indexOf(")", altEnd + 2);
  if (altEnd === -1 || destinationEnd === -1) return undefined;
  const alt = markdown.slice(cursor + 2, altEnd).trim();
  if (alt === "") throw new Error("Markdown images require non-empty alt text");
  return {
    alt,
    destination: markdown.slice(altEnd + 2, destinationEnd),
    end: destinationEnd + 1,
  };
}

/**
 * U+25CC DOTTED CIRCLE — the placeholder base a combining mark is shown on when
 * it is presented by itself. Written as an escape because the Write tool has
 * stripped non-ASCII literals from TypeScript sources before, and a silently
 * emptied string here would put the mark back on nothing.
 */
const DOTTED_CIRCLE = "\u25CC";

function scriptMatchers(options: InlineRenderOptionsInput | undefined): Array<{
  matcher: RegExp;
  scriptCommand: string;
}> {
  if (!options) return [];
  const mappings = Array.isArray(options) ? options : [options];
  const seen = new Set<string>();
  return mappings.map((mapping) => {
    if (!/^[A-Za-z_]+$/.test(mapping.unicodeScript)) {
      throw new Error(`invalid Unicode script '${mapping.unicodeScript}'`);
    }
    if (!/^[A-Za-z@]+$/.test(mapping.scriptCommand)) {
      throw new Error(`invalid LaTeX script command '${mapping.scriptCommand}'`);
    }
    if (seen.has(mapping.unicodeScript)) {
      throw new Error(`duplicate Unicode script '${mapping.unicodeScript}'`);
    }
    seen.add(mapping.unicodeScript);
    return {
      matcher: new RegExp(`^\\p{Script_Extensions=${mapping.unicodeScript}}$`, "u"),
      scriptCommand: mapping.scriptCommand,
    };
  });
}

type StraightQuoteRole = "opening" | "closing";

/**
 * Pair authored ASCII double quotes that belong to prose. Code spans, escaped
 * literals, and link destinations are intentionally outside this typography
 * pass so the generated PDF never changes their contents.
 */
function pairedStraightQuoteRoles(markdown: string): Map<number, StraightQuoteRole> {
  const positions: number[] = [];
  let cursor = 0;
  while (cursor < markdown.length) {
    const character = markdown[cursor] ?? "";
    if (character === "\\" && cursor + 1 < markdown.length) {
      const escaped = markdown[cursor + 1] ?? "";
      if (`!"#$%&'()*+,-./:;<=>?@[\\]^_\`{|}~`.includes(escaped)) {
        cursor += 2;
        continue;
      }
    }
    if (character === "`") {
      const end = markdown.indexOf("`", cursor + 1);
      if (end !== -1) {
        cursor = end + 1;
        continue;
      }
    }
    if (character === "[") {
      const labelEnd = markdown.indexOf("](", cursor + 1);
      const destinationEnd = labelEnd === -1 ? -1 : markdown.indexOf(")", labelEnd + 2);
      if (labelEnd !== -1 && destinationEnd !== -1) {
        cursor = destinationEnd + 1;
        continue;
      }
    }
    if (character === '"') positions.push(cursor);
    cursor += 1;
  }

  const roles = new Map<number, StraightQuoteRole>();
  const openings: number[] = [];
  for (const position of positions) {
    let previousIndex = position - 1;
    while (markdown[previousIndex] === "*") {
      previousIndex -= 1;
    }
    const previous = markdown[previousIndex];
    const next = markdown[position + 1];
    const hasOpeningContext =
      (previous === undefined || /\s/.test(previous) || "([{<—–-/:;=".includes(previous)) &&
      next !== undefined &&
      !/\s/.test(next);
    if (hasOpeningContext) {
      openings.push(position);
      continue;
    }
    const opening = openings.pop();
    if (opening === undefined) continue;
    roles.set(opening, "opening");
    roles.set(position, "closing");
  }
  return roles;
}

// ===========================================================================
// The book voice
// ===========================================================================
//
// A lesson is written once and read twice.
//
//     code/learning/human-languages/<track>/lessons/*.md   (canonical, HL04)
//                    |
//         +----------+-----------+
//         |                      |
//    NARRATION view          BOOK view  <-- this file
//    read out loud           read on a page
//         |                      |
//    keeps every cue        removes every cue
//    verbatim: "[PAUSE      because a reader sets their own
//    2s]" is an instruc-    pace, and a printed "[PAUSE 2s]"
//    tion to the voice.     is a stage direction that has
//                           wandered onto the stage.
//
// HL00 asks lessons to be authored as audio scripts, with bracketed delivery
// cues, so a track can be recorded. That is right for the source. It was
// never right for the page: a reader who opens the Spanish PDF should see
// a language book, not a shooting script.
//
// EVERY transformation below is BOOK-VIEW ONLY.
//   * It never edits a lesson file. `block.markdown` still holds the cues.
//   * No other consumer of `ParsedLesson` goes through this code path.
//   * There is no narration exporter in this package yet. When one is added
//     it must read `block.markdown` directly and MUST NOT reuse `bookVoice`,
//     or it will silently record lessons with the timing stripped out.
//
// ---------------------------------------------------------------------------
// 1. Printed section titles
// ---------------------------------------------------------------------------
//
// `classifyBlock` in parse.ts sorts each `## ` heading into a block type by
// looking at its words. A handful of those headings are really internal
// labels that got printed by accident. A language book does not head a
// section "Wrap-up recall".
//
// This table is the ONLY place the printed wording lives, so all twenty books
// say the same thing.

const BOOK_BLOCK_TITLES = {
  /**
   * The warm-up is the paragraph that says "you already know X --- here is the
   * new thing". That is simply how a section opens. It gets no printed label
   * at all: several lessons share a chapter, and a bold "Warm-up." five times
   * on one spread reads like a worksheet. Set as an indented lead-in instead,
   * which is what the typography was always doing anyway.
   */
  warmup: "",
  /** Was "Guided Practice" --- the phrase every language textbook actually uses. */
  guidedPractice: "Your turn",
  /** Was "Wrap-up recall" --- names the moment rather than the pedagogy. */
  recall: "Before you move on",
  /** Was "You'll want to know first" --- shorter, and reads as a heading. */
  input: "What to know first",
  /**
   * A connected passage. Titled rather than bare because it is the one block a
   * reader is meant to stop at and re-read, and an untitled box on the page
   * looks like an aside to be skimmed.
   */
  comprehension: "Read this",
} as const;

/**
 * Authored headings that are really block-type labels, lower-cased.
 *
 * Only *bare* headings are rewritten. Most "You'll want to know" headings
 * carry a descriptive tail --- "You'll want to know --- The famous mātrā" ---
 * and those are authorial prose that already reads like a book, so they pass
 * through untouched rather than being machine-mangled into something worse.
 */
const AUTHORED_TITLE_REWRITES = new Map<string, string>([
  ["guided practice", BOOK_BLOCK_TITLES.guidedPractice],
  ["you'll want to know first", BOOK_BLOCK_TITLES.input],
  ["you'll want to know", BOOK_BLOCK_TITLES.input],
]);

/**
 * Block labels that a few lessons extend with a qualifier --- "Guided
 * Practice: conjugate on command". The label still has to go; the qualifier is
 * the author's and stays, so the prefix alone is swapped.
 */
// The matcher is written out rather than built from the label at call time: a
// RegExp assembled from a string would silently reinterpret any metacharacter
// a future label happened to contain. Each entry carries its own literal
// pattern, and `length` is how much of the authored heading it replaces.
const AUTHORED_TITLE_PREFIXES: Array<{ matcher: RegExp; length: number; printed: string }> = [
  // The trailing separator is required, so "Guided Practicing" is never touched.
  { matcher: /^guided practice\s*[:—–-]/i, length: "guided practice".length,
    printed: BOOK_BLOCK_TITLES.guidedPractice },
];

/** The heading a reader sees, given the heading an author wrote. */
export function bookBlockTitle(authored: string): string {
  const trimmed = authored.trim();
  const exact = AUTHORED_TITLE_REWRITES.get(trimmed.toLowerCase());
  if (exact !== undefined) return exact;
  for (const { matcher, length, printed } of AUTHORED_TITLE_PREFIXES) {
    if (matcher.test(trimmed)) return printed + trimmed.slice(length);
  }
  return authored;
}

// ---------------------------------------------------------------------------
// 2. Delivery cues
// ---------------------------------------------------------------------------
//
// Three cue shapes exist across the lesson files. Their grammar --- where a cue's
// bracket closes, and what its words mean --- lives in `delivery-cue.ts`, shared
// with the narration script, so the page and the voice can never disagree about
// what is a cue. This section only decides what each one LOOKS like in print:
//
//   [PAUSE 2s]      "hold here" --- meaningless in print, so it is deleted.
//   [REPEAT x2]     "say that twice" --- said in prose instead.
//   [YOU SAY: ...]  "the learner's turn" --- typeset as a practice prompt.
//
// A cue can sit anywhere: heading a line, filling a bullet, mid-sentence, or
// wrapped by the author's editor across two or three source lines. The book used
// to find cues one source line at a time with anchored regular expressions, so
// every one of those positions except the first two leaked: 60 generated chapters
// in 12 books printed `{[}YOU SAY: ...{]}`, `{[}PAUSE 4s{]}` or `{[}REPEAT x2{]}`
// as literal text. Now each paragraph and each list item is scanned as the one
// unit Markdown will typeset it as, and `check:books` refuses any chapter that
// still prints a cue's brackets.
//
// `[YOU <VERB>: ...]` uses twenty-eight different verbs. Each needs an English
// imperative a book can print. Two shapes are possible:
//
//   item  a label on one bullet, used when a list mixes cue kinds:
//             *Say it:* "siento" --- I feel
//   lead  one lead-in above a run of bullets that all share a verb, so the
//         instruction is given once instead of three times:
//             Say these aloud:
//               - "siento" --- I feel
//               - the cousin word --- "perdón"
//
// `lead` is deliberately absent for most verbs: "Choose each of these:" is
// worse English than repeating "*Choose:*" per bullet, so those fall back to
// the item form. Absent `lead` therefore means "this verb does not pluralise
// into a natural instruction", not "not filled in yet".

interface CueVoice {
  item: string;
  lead?: string;
}

const CUE_VOICES: Record<string, CueVoice> = {
  SAY: { item: "Say it", lead: "Say these aloud" },
  WRITE: { item: "Write it", lead: "Write these out" },
  READ: { item: "Read it", lead: "Read these aloud" },
  TRACE: { item: "Trace it", lead: "Trace these" },
  POINT: { item: "Point to" },
  CHOOSE: { item: "Choose" },
  CONTRAST: { item: "Contrast" },
  CONNECT: { item: "Connect" },
  QUALIFY: { item: "Qualify" },
  BUILD: { item: "Build" },
  USE: { item: "Use" },
  SEGMENT: { item: "Break apart" },
  RUN: { item: "Run through" },
  IDENTIFY: { item: "Identify" },
  ANSWER: { item: "Answer" },
  SWITCH: { item: "Switch" },
  REBUILD: { item: "Rebuild" },
  PARAPHRASE: { item: "Put it another way" },
  LABEL: { item: "Label" },
  KEEP: { item: "Keep" },
  HUM: { item: "Hum" },
  GESTURE: { item: "Gesture" },
  FRAME: { item: "Frame" },
  FEEL: { item: "Feel" },
  COMPARE: { item: "Compare" },
  CLASSIFY: { item: "Sort" },
  ASK: { item: "Ask" },
  ADD: { item: "Add" },
};

/**
 * A verb nobody has given a voice to yet still has to print as English --- and
 * one lesson writes a whole phrase, `[YOU CHOOSE BY CONTEXT: ...]`, so the
 * fallback sentence-cases the whole thing: "Choose by context".
 *
 * The lookup goes through `hasOwn` rather than a bare index, so a verb that
 * happened to spell an inherited member could never resolve to `Object`'s
 * prototype. `parseDeliveryCue` already restricts the verb to A--Z and spaces, which
 * rules that out today; the guard means it stays ruled out if the cue grammar
 * is ever widened.
 */
function cueVoice(verb: string): CueVoice {
  const known = hasOwn(CUE_VOICES, verb) ? CUE_VOICES[verb] : undefined;
  return known ?? { item: verb.charAt(0) + verb.slice(1).toLowerCase() };
}

/** A Markdown bullet, matching renderMarkdown's own column-zero rule. */
const LIST_ITEM = /^- /;

/** How a prompt cue's verb prints, qualifier included: "Say it (m.)". */
function promptLabel(cue: PromptCue): string {
  return joinQualifier(cueVoice(cue.action).item, cue.qualifier);
}

/** `[REPEAT x2]` in words: "Twice through", "3 times through". */
function repeatLead(times: number): string {
  return times === 2 ? "Twice through" : `${times} times through`;
}

/**
 * The book-voice output of {@link voiceCues}, built as a list of pieces.
 *
 * Why not one growing string: `output += piece` makes a rope, and the two
 * questions this builder has to answer --- "is the output at the head of a
 * line?" and "strip the blank before this deleted pause" --- would read the
 * rope's last characters, which flattens it. Once per cue, that is quadratic:
 * a line of 40,000 `[REPEAT x2]` cues took fourteen seconds. Looking only at
 * the last few pieces keeps every question O(1) amortised.
 */
class VoicedText {
  private readonly pieces: string[] = [];

  push(piece: string): void {
    if (piece !== "") this.pieces.push(piece);
  }

  /** True when everything after the last newline written so far is blank. */
  atLineHead(): boolean {
    for (let piece = this.pieces.length - 1; piece >= 0; piece -= 1) {
      const text = this.pieces[piece] ?? "";
      for (let index = text.length - 1; index >= 0; index -= 1) {
        const character = text[index];
        if (character === "\n") return true;
        if (character !== " " && character !== "\t") return false;
      }
    }
    return true;
  }

  /** Drop trailing spaces and tabs (never newlines). Each removed blank is gone for good. */
  trimTrailingBlanks(): void {
    while (this.pieces.length > 0) {
      const last = this.pieces.length - 1;
      const kept = trimTrailingBlanks(this.pieces[last] ?? "");
      if (kept !== "") {
        this.pieces[last] = kept;
        return;
      }
      this.pieces.pop();
    }
  }

  toString(): string {
    return this.pieces.join("");
  }
}

/** `text` without its trailing spaces and tabs (newlines are kept). */
function trimTrailingBlanks(text: string): string {
  let end = text.length;
  while (end > 0 && (text[end - 1] === " " || text[end - 1] === "\t")) end -= 1;
  return end === text.length ? text : text.slice(0, end);
}

/**
 * Put every delivery cue in `text` into book voice, wherever it sits.
 *
 * `text` is one unit Markdown will typeset as a single run --- a paragraph's
 * lines, or one list item --- with its source newlines still in it. A cue may
 * cross those newlines; that is the case this function exists for:
 *
 *     source       Cover the word and write it. [YOU WRITE: the word, then
 *                  its meaning] Check the spelling.
 *
 *     book voice   Cover the word and write it. *Write it:* the word, then its meaning Check the spelling.
 *
 * The cue's content is re-joined onto one line exactly as renderMarkdown would
 * join it (lines trimmed, one space between), so the LaTeX is identical to the
 * same cue authored on one line. Lines the cue did not cross keep their breaks.
 *
 *   pause    deleted, along with the blank it leaves, so "a. [PAUSE 2s] b"
 *            prints "a. b" and a line that was only a pause prints nothing.
 *   repeat   at the head of a line it introduces the copy that follows
 *            ("*Twice through:* ..."); after prose it refers back to it
 *            ("... let it creak. *Twice through.*").
 *   prompt   "*Say it:* content", the qualifier carried into the label.
 *
 * An index scan, not a regular expression. A `[` that does not open with a cue
 * keyword costs one comparison (`opensDeliveryCue`); one that does costs at most
 * `MAX_CUE_LENGTH` steps (`closingBracket`). Either way the walk is linear.
 * A bracket that is not a cue is copied through and scanning resumes just
 * inside it, so a gloss like `[I am your friend]` is untouched.
 */
function voiceCues(text: string): string {
  const output = new VoicedText();
  let index = 0;
  // Plain runs are copied as slices, not character by character.
  let copiedTo = 0;
  while (index < text.length) {
    const character = text[index];
    if (character === "\\") {
      index += 2;
      continue;
    }
    if (character !== "[") {
      index += 1;
      continue;
    }
    const close = opensDeliveryCue(text, index) ? closingBracket(text, index) : -1;
    const cue = close === -1 ? null : parseDeliveryCue(text.slice(index + 1, close));
    if (!cue) {
      index += 1;
      continue;
    }
    output.push(text.slice(copiedTo, index));
    let next = close + 1;
    if (cue.kind === "pause") {
      // Swallow the blank after the cue too, then decide whether the blank
      // BEFORE it is still needed: not at the end of a line, and not before
      // punctuation ("word [PAUSE 2s]." must print "word.").
      while (text[next] === " " || text[next] === "\t") next += 1;
      const following = text[next];
      if (following === undefined || following === "\n" || ".,;:!?)".includes(following)) {
        output.trimTrailingBlanks();
      }
    } else if (cue.kind === "repeat") {
      if (output.atLineHead()) {
        while (text[next] === " " || text[next] === "\t") next += 1;
        output.push(`*${repeatLead(cue.times)}:* `);
      } else {
        output.push(`*${repeatLead(cue.times)}.*`);
      }
    } else {
      // A cue inside a cue's content ("[YOU SAY: a [PAUSE 1s] b]") is voiced
      // too. The content is strictly shorter than the text, so this recursion
      // always ends.
      output.push(`*${promptLabel(cue)}:* ${voiceCues(cue.content)}`);
    }
    index = next;
    copiedTo = next;
  }
  output.push(text.slice(copiedTo));
  return output.toString();
}

/**
 * A prompt cue that fills a whole list item, or undefined.
 *
 * "Whole" is decided by the bracket scan, not by an anchored pattern: the old
 * `^\[YOU ...:(.*)\]$` read `[YOU HEAR: a] [YOU ANSWER: b]` as ONE cue whose
 * content was `a] [YOU ANSWER: b`, and printed the inner brackets raw.
 */
function wholePromptCue(item: string): PromptCue | undefined {
  const trimmed = item.trim();
  if (!trimmed.startsWith("[")) return undefined;
  if (closingBracket(trimmed, 0) !== trimmed.length - 1) return undefined;
  const cue = parseDeliveryCue(trimmed.slice(1, -1));
  return cue?.kind === "prompt" ? cue : undefined;
}

/**
 * Gather one Markdown list into logical items, keeping each item's wrapped
 * continuation lines (trimmed) on their own lines inside it. `voiceCues` joins
 * them only where a cue crosses them, and renderMarkdown joins the rest with a
 * single space, so the item typesets exactly as before.
 *
 * Each item is gathered as an ARRAY of lines with an {@link OpenCueTracker}
 * beside it, and joined once at the end. An earlier draft re-joined the item
 * into one string per line and re-scanned its tail to ask "is a cue still
 * open?" --- which is quadratic, because reading the tail of a growing
 * concatenated string flattens it: a bullet followed by 80,000 flush-left
 * `[YOU a` lines took 28 seconds. Now each line is read exactly once.
 */
function collectListItems(lines: string[], start: number): { items: string[]; next: number } {
  const items: string[][] = [];
  let tracker = new OpenCueTracker();
  let cursor = start;
  const extend = (line: string): void => {
    items.at(-1)?.push(line);
    tracker.feed(line);
  };
  while (cursor < lines.length) {
    const line = (lines[cursor] ?? "").trimEnd();
    if (LIST_ITEM.test(line)) {
      items.push([]);
      tracker = new OpenCueTracker();
      extend(line.slice(2));
      cursor += 1;
      continue;
    }
    if (items.length > 0 && /^\s+\S/.test(line)) {
      extend(line.trim());
      cursor += 1;
      continue;
    }
    // A Markdown "lazy" continuation: an UNindented line that still belongs to
    // the item because the cue it is finishing never closed. Tamil chapter 29
    // wraps a cue's last word flush left ---
    //
    //     - [YOU SAY: the honest structural surprise --- no ... at
    //       all, unlike Tamil's
    //     neighbours]
    //
    // --- and without this the bullet ended at "Tamil's", leaving both halves
    // printed raw. Only an OPEN cue licenses it; any other flush-left line
    // still ends the list, exactly as renderMarkdown has always typeset it.
    if (items.length > 0 && line.trim() !== "" && tracker.hasOpenCue()) {
      extend(line.trim());
      cursor += 1;
      continue;
    }
    break;
  }
  return { items: items.map((item) => item.join("\n")), next: cursor };
}

/**
 * Answers "has a delivery cue (`[YOU `, `[PAUSE `, `[REPEAT `) opened in this
 * item and not yet closed?" while the item's lines are fed in one at a time.
 *
 * A running bracket stack: `[` pushes, `]` pops, and a `[` still on the stack
 * is one no `]` has closed --- exactly when the depth scan in `closingBracket`
 * would fail. Alongside it, the offsets of the stack entries that open a cue,
 * in stack order, so the answer is the topmost of them. A cue that opened more
 * than `MAX_CUE_LENGTH` characters ago can never close within the bound, so it
 * no longer counts as open; that stops a hostile item from absorbing every
 * flush-left line after it on the strength of one ancient `[YOU`.
 *
 * Each character is read once, so feeding a whole item is linear in its length.
 */
class OpenCueTracker {
  /** For each unclosed `[`, whether it opens a cue. */
  private readonly stack: boolean[] = [];
  /** Offsets of the unclosed `[` that open a cue, oldest first. */
  private readonly cueOffsets: number[] = [];
  /** Characters fed so far, the newlines that will join the lines included. */
  private length = 0;

  feed(line: string): void {
    if (this.length > 0) this.length += 1; // the "\n" that will join this line on
    // The newline after the line counts as the blank after a keyword: `[YOU`
    // ending one line with `SAY: …` on the next is a wrapped cue.
    const text = `${line}\n`;
    let index = 0;
    while (index < line.length) {
      const character = line[index];
      if (character === "\\") {
        index += 2;
        continue;
      }
      if (character === "[") {
        const opensCue = opensDeliveryCue(text, index);
        this.stack.push(opensCue);
        if (opensCue) this.cueOffsets.push(this.length + index);
      } else if (character === "]" && this.stack.length > 0) {
        if (this.stack.pop() === true) this.cueOffsets.pop();
      }
      index += 1;
    }
    this.length += line.length;
  }

  hasOpenCue(): boolean {
    const newest = this.cueOffsets.at(-1);
    return newest !== undefined && this.length - newest < MAX_CUE_LENGTH;
  }
}

/**
 * Re-join a list item's lines the way the old collector did and renderMarkdown
 * still does: continuation lines trimmed, one space between, the first line as
 * written. (Not `joinWrappedLines`, which also trims the item's outer edges ---
 * that would be a silent change to bullets that hold no cue at all.)
 */
function joinItemLines(item: string): string {
  if (!item.includes("\n")) return item;
  return item
    .split("\n")
    .map((line, index) => (index === 0 ? line : line.trim()))
    .join(" ");
}

/**
 * Turn a collected list back into Markdown, in book voice.
 *
 * Uniform-verb lists of two or more whole-cue bullets get the instruction once,
 * above the list. Anything else --- a lone cue, a mixed list, a qualified cue
 * whose "(m.)" a shared lead-in would lose, a cue that shares its bullet with
 * prose --- keeps the per-bullet form, so no information is lost. A bullet that
 * was nothing but a pause has nothing left to print and is dropped.
 */
function renderCueList(items: string[]): string[] {
  const cues = items.map(wholePromptCue);
  const verbs = new Set(cues.map((cue) => cue?.action));
  const uniformVerb = verbs.size === 1 ? [...verbs][0] : undefined;
  const leadable = cues.every((cue) => cue !== undefined && cue.qualifier === "");
  if (uniformVerb !== undefined && leadable && items.length > 1) {
    const lead = cueVoice(uniformVerb).lead;
    if (lead !== undefined) {
      return [`${lead}:`, "", ...cues.map((cue) => `- ${voiceCues(cue?.content ?? "")}`)];
    }
  }
  const rendered: string[] = [];
  for (const item of items) {
    const voiced = joinItemLines(voiceCues(item));
    if (voiced === "" && item.trim() !== "") continue;
    rendered.push(`- ${voiced}`);
  }
  return rendered;
}

/**
 * True when a source line starts a new typeset unit in renderMarkdown rather
 * than continuing the paragraph above it: a quote, a numbered item, a table row,
 * a heading, or a block image. A cue is never allowed to reach across one of
 * these --- its two halves would land in different LaTeX environments.
 */
function startsTypesetUnit(line: string): boolean {
  const trimmed = line.trim();
  if (trimmed.startsWith(">") || trimmed.startsWith("|") || trimmed.startsWith("#")) return true;
  if (trimmed.startsWith("![")) return true;
  let digits = 0;
  while (trimmed[digits] !== undefined && trimmed[digits]! >= "0" && trimmed[digits]! <= "9") {
    digits += 1;
  }
  return digits > 0 && trimmed[digits] === "." && /\s/.test(trimmed[digits + 1] ?? "");
}

/** A `> ` blockquote line --- renderMarkdown joins a run of them into one quote. */
function isQuoteLine(line: string): boolean {
  return line.startsWith("> ");
}

/**
 * Voice one run of consecutive non-blank, non-bullet lines.
 *
 * The run is cut into typeset units (see {@link startsTypesetUnit}); each unit
 * is voiced as one string so a wrapped cue is seen whole. One exception to the
 * cut: consecutive `> ` lines are ONE unit, because renderMarkdown typesets
 * them as one quote and the narration speaks them as one utterance --- so a cue
 * wrapped inside a blockquote is a cue to both. Their `> ` markers come off
 * before the scan (or they would land inside the cue's content) and go back on
 * every line after it.
 *
 * A line the cues left empty --- a bare `[PAUSE 1s]` --- is dropped: the right
 * print treatment of an instruction to wait is no ink at all.
 */
function voiceProseRun(run: string[]): string[] {
  const units: string[][] = [];
  for (const line of run) {
    const current = units.at(-1);
    const continuesQuote =
      current !== undefined && isQuoteLine(line) && isQuoteLine(current.at(-1) ?? "");
    if (current === undefined || (startsTypesetUnit(line) && !continuesQuote)) units.push([line]);
    else current.push(line);
  }
  const output: string[] = [];
  for (const unit of units) {
    const quoted = isQuoteLine(unit[0] ?? "");
    const prefixes = unit.map((line) => (quoted && isQuoteLine(line) ? "> " : ""));
    const text = unit.map((line, index) => line.slice(prefixes[index]?.length ?? 0)).join("\n");
    const voiced = voiceCues(text).split("\n");
    // If no cue crossed a line, every line is still where it was and gets its
    // own marker back --- an indented continuation keeps its indentation and no
    // marker, byte for byte as authored. If one did, the lines no longer line
    // up with their markers; a quote unit then marks every line, trimmed, which
    // renderMarkdown joins into the same single quote.
    const aligned = voiced.length === unit.length;
    voiced.forEach((line, index) => {
      const kept = trimTrailingBlanks(line);
      if (kept.trim() === "") return;
      if (!quoted) output.push(kept);
      else if (aligned) output.push(`${prefixes[index] ?? ""}${kept}`);
      else output.push(`> ${kept.trimStart()}`);
    });
  }
  return output;
}

// ---------------------------------------------------------------------------
// 3. The gate: a printed page never shows a cue's brackets
// ---------------------------------------------------------------------------
//
// Every earlier leak of this kind was found by a person reading a PDF, never by
// a check: `check:books` compares the generator with its own committed output,
// and `{[}YOU SAY: ...{]}` is perfectly valid LaTeX that compiles without a
// warning. So the generator's output is now inspected for the one thing it must
// never contain. The escaped form `{[}` is what renderInlineMarkdown makes of a
// literal `[`; the bare form is listed too so a future raw-LaTeX path cannot
// smuggle one past.

/**
 * What a delivery cue looks like once it has reached a `.tex` file: an opening
 * bracket, escaped (`{[}`) or bare, then a cue keyword.
 */
const PRINTED_CUE_BRACKETS = ["{[}", "["] as const;
const PRINTED_CUE_KEYWORDS = ["YOU", "PAUSE", "REPEAT"] as const;

/** One raw cue found in generated LaTeX: 1-based line, and the opener it used. */
export interface PrintedCue {
  line: number;
  /** The opener, normalised to one trailing space: `{[}YOU `, `[PAUSE `. */
  opener: string;
}

/**
 * Every place `tex` prints a delivery cue's brackets instead of book voice.
 * An empty list is the only acceptable answer for a generated chapter.
 *
 * The keyword counts when it is followed by a space, a tab, or the end of the
 * line --- the same "any blank" rule the cue parser uses (`afterKeyword` in
 * delivery-cue.ts). A gate that wanted exactly one space would have waved
 * `{[}YOU\tSAY: hi{]}` through, and that is exactly what it once did. `[YOUR`
 * and `[PAUSED` are words, not cues, and are not flagged.
 *
 * Plain `indexOf` scans: linear in the text, no pattern to tune. The escaped
 * opener cannot also match its bare twin --- `{[}YOU` has a `}` between the `[`
 * and the word --- so one leak is reported once.
 */
export function findPrintedDeliveryCues(tex: string): PrintedCue[] {
  const found: PrintedCue[] = [];
  const lines = tex.split("\n");
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index] ?? "";
    for (const bracket of PRINTED_CUE_BRACKETS) {
      for (const keyword of PRINTED_CUE_KEYWORDS) {
        const opener = `${bracket}${keyword}`;
        let at = line.indexOf(opener);
        while (at !== -1) {
          const after = line[at + opener.length];
          if (after === undefined || after === " " || after === "\t") {
            found.push({ line: index + 1, opener: `${opener} ` });
            break;
          }
          at = line.indexOf(opener, at + 1);
        }
      }
    }
  }
  return found;
}

/**
 * Rewrite one block's authored Markdown into the version the book prints.
 * Book view only --- see the banner at the top of this section.
 */
export function bookVoice(markdown: string): string {
  const lines = markdown.split("\n");
  const output: string[] = [];
  let cursor = 0;
  while (cursor < lines.length) {
    const line = (lines[cursor] ?? "").trimEnd();
    if (LIST_ITEM.test(line)) {
      const { items, next } = collectListItems(lines, cursor);
      const rendered = renderCueList(items);
      // A lead-in is a paragraph. If the previous line is still prose, keep the
      // blank line that stops Markdown from welding the two together.
      const needsBlank =
        rendered[0] !== undefined &&
        !LIST_ITEM.test(rendered[0]) &&
        output.length > 0 &&
        (output.at(-1) ?? "").trim() !== "";
      if (needsBlank) output.push("");
      output.push(...rendered);
      cursor = next;
      continue;
    }
    if (line.trim() === "") {
      output.push(line);
      cursor += 1;
      continue;
    }
    const run: string[] = [];
    while (cursor < lines.length) {
      const candidate = (lines[cursor] ?? "").trimEnd();
      if (candidate.trim() === "" || LIST_ITEM.test(candidate)) break;
      run.push(candidate);
      cursor += 1;
    }
    output.push(...voiceProseRun(run));
  }
  return output.join("\n");
}

// ===========================================================================

/** Render the deliberately small inline subset used by schema-v2 lessons. */
export function renderInlineMarkdown(
  markdown: string,
  options?: InlineRenderOptionsInput,
): string {
  markdown = markdown.normalize("NFC");
  const output: string[] = [];
  const emphasis: Array<"italic" | "bold"> = [];
  const scripts = scriptMatchers(options);
  const straightQuotes = pairedStraightQuoteRoles(markdown);
  let cursor = 0;
  const open = (kind: "italic" | "bold"): void => {
    output.push(kind === "italic" ? "\\emph{" : "\\textbf{");
    emphasis.push(kind);
  };
  const close = (): void => {
    output.push("}");
    emphasis.pop();
  };
  const nextEmphasisDelimiterWidth = (from: number): number | undefined => {
    for (let index = from; index < markdown.length; index += 1) {
      if (markdown[index] !== "*" || markdown[index - 1] === "\\") continue;
      let width = 1;
      while (width < 3 && markdown[index + width] === "*") width += 1;
      return width;
    }
    return undefined;
  };
  while (cursor < markdown.length) {
    const codePoint = markdown.codePointAt(cursor);
    const character = codePoint === undefined ? "" : String.fromCodePoint(codePoint);
    if (markdown.startsWith("ī\u0301", cursor)) {
      output.push("\\'{\\={\\i}}");
      cursor += 2;
      continue;
    }
    if (markdown.startsWith("ā\u0301", cursor)) {
      output.push("\\'{\\={a}}");
      cursor += 2;
      continue;
    }
    if (character === "\\" && cursor + 1 < markdown.length) {
      const escaped = markdown[cursor + 1] ?? "";
      if (`!"#$%&'()*+,-./:;<=>?@[\\]^_\`{|}~`.includes(escaped)) {
        output.push(escapeLatexCharacter(escaped));
        cursor += 2;
        continue;
      }
    }
    const image = markdownImageAt(markdown, cursor);
    if (image) {
      output.push(`\\hlinlinefigure{${escapeLatexImageDestination(image.destination)}}`);
      cursor = image.end;
      continue;
    }
    // U+25CC DOTTED CIRCLE has Script_Extensions=Common, so on its own it joins
    // no script run — and printed outside one it is handed to the Latin body
    // font, which has no such glyph. That is not hypothetical: the first build of
    // the Indic recognition segments logged 184 "Missing character" warnings,
    // every one of them this character, leaving a hole in the PDF exactly where
    // the mark being taught should have been.
    //
    // The dotted circle exists for one purpose: to be the base a combining mark
    // sits on when the mark is shown by itself. So when it is followed by a
    // character that DOES belong to a run, it belongs to that run — that is what
    // it is for, and the script's own font is the one that has the glyph.
    const runOpener =
      character === DOTTED_CIRCLE
        ? String.fromCodePoint(markdown.codePointAt(cursor + character.length) ?? 0)
        : character;
    const script = scripts.find((candidate) => candidate.matcher.test(runOpener));
    if (script) {
      const run: string[] = [];
      while (cursor < markdown.length) {
        const nextCodePoint = markdown.codePointAt(cursor);
        const next = nextCodePoint === undefined ? "" : String.fromCodePoint(nextCodePoint);
        const carriesAMark =
          next === DOTTED_CIRCLE &&
          script.matcher.test(String.fromCodePoint(markdown.codePointAt(cursor + next.length) ?? 0));
        if (!script.matcher.test(next) && !carriesAMark) break;
        run.push(escapeLatexCharacter(next));
        cursor += next.length;
      }
      output.push(`\\${script.scriptCommand}{${run.join("")}}`);
      continue;
    }
    if (markdown[cursor] === "`") {
      const end = markdown.indexOf("`", cursor + 1);
      if (end !== -1) {
        const literal = markdown
          .slice(cursor + 1, end)
          .split("")
          .map(escapeLatexCharacter)
          .join("");
        output.push(`\\texttt{${literal}}`);
        cursor = end + 1;
        continue;
      }
    }
    if (markdown[cursor] === "[") {
      const labelEnd = markdown.indexOf("](", cursor + 1);
      const destinationEnd = labelEnd === -1 ? -1 : markdown.indexOf(")", labelEnd + 2);
      if (labelEnd !== -1 && destinationEnd !== -1) {
        const destination = absoluteBookLink(markdown.slice(labelEnd + 2, destinationEnd));
        const label = renderInlineMarkdown(markdown.slice(cursor + 1, labelEnd), options);
        output.push(
          destination === undefined
            ? label
            : `\\href{${escapeLatexLinkDestination(destination)}}{${label}}`,
        );
        cursor = destinationEnd + 1;
        continue;
      }
    }
    const straightQuote = straightQuotes.get(cursor);
    if (straightQuote) {
      output.push(
        straightQuote === "opening" ? "\\textquotedblleft{}" : "\\textquotedblright{}",
      );
      cursor += 1;
      continue;
    }
    if (markdown.startsWith("***", cursor)) {
      const top = emphasis.at(-1);
      const below = emphasis.at(-2);
      if (top && below && top !== below) {
        close();
        close();
      } else if (!top && nextEmphasisDelimiterWidth(cursor + 3) === 1) {
        // A triple opener is ambiguous: `***x** y.*` is italic around bold,
        // while `***x*, *y***` is bold around two italic runs. The first
        // delimiter inside the run tells us which layer must be on top.
        open("bold");
        open("italic");
      } else {
        open("italic");
        open("bold");
      }
      cursor += 3;
      continue;
    }
    if (markdown.startsWith("**", cursor)) {
      if (emphasis.at(-1) === "bold") close();
      else open("bold");
      cursor += 2;
      continue;
    }
    if (markdown[cursor] === "*") {
      if (emphasis.at(-1) === "italic") close();
      else open("italic");
      cursor += 1;
      continue;
    }
    output.push(escapeLatexCharacter(markdown[cursor] ?? ""));
    cursor += 1;
  }
  while (emphasis.length > 0) close();
  return output.join("");
}

function renderMarkdown(
  markdown: string,
  options?: InlineRenderOptionsInput,
  tableLayout: "grid" | "records" = "grid",
): string {
  // An HTML comment is BY DEFINITION not reader-facing, and this renderer used
  // to pass any non-directive one straight into the book as body text. One had
  // been typesetting into the shipped Spanish PDF -- a note from an author to
  // future authors, printed inside a coloured culture box for the reader.
  //
  // `parse.ts` strips the `hl-knowledge` and `hl-activity` directives because it
  // consumes them; everything else arrived here untouched. Stripping is done on
  // the whole string rather than per line so a comment spanning several lines --
  // which the live instance did -- goes as one unit.
  markdown = stripHtmlComments(markdown, "remove");
  const output: string[] = [];
  const paragraph: string[] = [];
  const quote: string[] = [];
  let listOpen = false;
  // Which environment the open list is in. Markdown's `- ` and `1. ` are
  // different lists and must not be merged into one.
  let listEnv: "itemize" | "enumerate" = "itemize";
  let listItem: string[] = [];
  const tableRows: string[][] = [];

  const flushParagraph = (): void => {
    if (paragraph.length === 0) return;
    output.push(renderInlineMarkdown(paragraph.join(" "), options), "");
    paragraph.length = 0;
  };
  const flushQuote = (): void => {
    if (quote.length === 0) return;
    output.push(
      "\\begin{quote}",
      renderInlineMarkdown(quote.join(" "), options),
      "\\end{quote}",
      "",
    );
    quote.length = 0;
  };
  const flushListItem = (): void => {
    if (listItem.length === 0) return;
    output.push(`  \\item ${renderInlineMarkdown(listItem.join(" "), options)}`);
    listItem = [];
  };
  const closeList = (): void => {
    if (!listOpen) return;
    flushListItem();
    output.push(`\\end{${listEnv}}`, "");
    listOpen = false;
  };

  const flushTable = (): void => {
    if (tableRows.length === 0) return;
    const [header = [], separator = [], ...body] = tableRows;
    const isSeparator =
      separator.length === header.length &&
      separator.every((cell) => /^:?-{3,}:?$/.test(cell));
    if (!isSeparator || header.length === 0) {
      output.push(
        ...tableRows.flatMap((row) => [
          renderInlineMarkdown(`| ${row.join(" | ")} |`, options),
          "",
        ]),
      );
      tableRows.length = 0;
      return;
    }
    if (tableLayout === "records") {
      const renderedHeader = header.map((cell) => renderInlineMarkdown(cell, options));
      output.push(
        "\\begin{itemize}",
        "\\raggedright",
        "\\setlength{\\itemsep}{0.35em}",
        ...body.flatMap((row) => {
          const rendered = Array.from({ length: header.length }, (_, index) =>
            renderInlineMarkdown(row[index] ?? "", options),
          );
          return [
            "  \\item \\begin{minipage}[t]{\\linewidth}",
            `  \\textbf{${renderedHeader[0] ?? ""}:} ${rendered[0] ?? ""}`,
            ...rendered.slice(1).map(
              (cell, index) =>
                `  \\par \\textbf{${renderedHeader[index + 1] ?? ""}:} ${cell}`,
            ),
            "  \\end{minipage}",
          ];
        }),
        "\\end{itemize}",
        "",
      );
      tableRows.length = 0;
      return;
    }
    const columns = Array.from(
      { length: header.length },
      () => ">{\\raggedright\\arraybackslash}X",
    ).join("");
    const cells = (row: string[]): string[] =>
      Array.from({ length: header.length }, (_, index) =>
        renderInlineMarkdown(row[index] ?? "", options),
      );
    // A row's first cell is whatever follows the previous row's `\\`, and `\\*`
    // is a LaTeX command in its own right -- the starred line break, "no page
    // break here". So a table row beginning with an asterisk has that asterisk
    // SWALLOWED by the `\\` above it: the row still typesets, one character
    // short, and nothing complains. The .tex on disk contains the asterisk, so
    // every text assertion and every hash check passes; only the printed page is
    // wrong, and only the second row onward, because the first body row follows
    // `\midrule` instead.
    //
    // This is not hypothetical padding against a rule nobody breaks. Every
    // reconstructed proto-form is written with a leading asterisk -- `*ph2ter`,
    // `*swesor` -- and the moment a lesson puts a column of them in a table, the
    // convention that MARKS them as reconstructed is the thing that disappears.
    // French chapter 12 did exactly that and lost the star from `*meh2ter`.
    //
    // Bracing hides the `*` from `\\`'s lookahead and typesets identically.
    // `[` needs no such guard: it is already escaped to `{[}` upstream, which
    // would otherwise be read as `\\[<length>]`.
    const protectLeadingStar = (cell: string): string =>
      cell.startsWith("*") ? `{*}${cell.slice(1)}` : cell;
    const bodyRow = (row: string[]): string => {
      const rendered = cells(row);
      const [first, ...rest] = rendered;
      return [protectLeadingStar(first ?? ""), ...rest].join(" & ");
    };
    output.push(
      "\\noindent",
      `\\begin{tabularx}{\\linewidth}{@{}${columns}@{}}`,
      "\\toprule",
      `${cells(header)
        .map((cell) => `\\textbf{${cell}}`)
        .join(" & ")} \\\\`,
      "\\midrule",
      ...body.map((row) => `${bodyRow(row)} \\\\`),
      "\\bottomrule",
      "\\end{tabularx}",
      "",
    );
    tableRows.length = 0;
  };

  for (const rawLine of markdown.split(/\r?\n/)) {
    const line = rawLine.trimEnd();
    if (line.trim() === "") {
      flushParagraph();
      flushQuote();
      closeList();
      flushTable();
      continue;
    }
    const image = markdownImageAt(line.trim(), 0);
    if (image && image.end === line.trim().length) {
      flushParagraph();
      flushQuote();
      closeList();
      flushTable();
      output.push(
        `\\hlblockfigure{${escapeLatexImageDestination(image.destination)}}{${renderInlineMarkdown(image.alt, options)}}`,
        "",
      );
      continue;
    }
    if (/^\s*\|.*\|\s*$/.test(line)) {
      flushParagraph();
      flushQuote();
      closeList();
      tableRows.push(
        line
          .trim()
          .replace(/^\|/, "")
          .replace(/\|$/, "")
          .split("|")
          .map((cell) => cell.trim()),
      );
      continue;
    }
    flushTable();
    if (line.startsWith("> ")) {
      flushParagraph();
      closeList();
      quote.push(line.slice(2));
      continue;
    }
    // A line that is just `>` is markdown's blank line INSIDE a blockquote: it
    // separates two quoted paragraphs. It does not start with "> ", so without
    // this branch it fell through to the paragraph path and was emitted as a
    // literal `>` between two `quote` environments -- a stray character that
    // reached readers in 67 generated chapters across 13 books before anyone
    // read a compiled page and saw it. Treat it as the separator it is.
    if (line.trim() === ">") {
      flushParagraph();
      closeList();
      flushQuote();
      continue;
    }
    if (quote.length > 0 && /^\s+/.test(line)) {
      quote.push(line.trim());
      continue;
    }
    if (line.startsWith("- ")) {
      flushParagraph();
      flushQuote();
      if (listOpen && listEnv !== "itemize") closeList();
      if (!listOpen) {
        listEnv = "itemize";
        output.push("\\begin{itemize}", "\\raggedright");
        if (tableLayout === "records") output.push("\\setlength{\\itemsep}{0.35em}");
        listOpen = true;
      }
      flushListItem();
      listItem = [line.slice(2)];
      continue;
    }
    // `1. ` — an ORDERED list. There was no branch for this at all, so numbered
    // steps fell through to the paragraph path and were joined into a single
    // run-on line. 72 generated chapters shipped that way, and most of them are
    // writing instructions, where the steps are the whole point: Hindi chapter 1
    // printed "1. a left bowl -- a rounded scoop on the left. 2. a right spine
    // -- a vertical downstroke..." as one paragraph. No text assertion could see
    // it; only the compiled page shows it.
    const ordered = /^(\d+)\.\s+(.*)$/.exec(line);
    if (ordered) {
      flushParagraph();
      flushQuote();
      if (listOpen && listEnv !== "enumerate") closeList();
      if (!listOpen) {
        listEnv = "enumerate";
        output.push("\\begin{enumerate}", "\\raggedright");
        if (tableLayout === "records") output.push("\\setlength{\\itemsep}{0.35em}");
        listOpen = true;
      }
      flushListItem();
      listItem = [ordered[2]];
      continue;
    }
    if (listOpen && /^\s+/.test(line)) {
      listItem.push(line.trim());
      continue;
    }
    if (quote.length > 0) flushQuote();
    if (listOpen) closeList();
    paragraph.push(line.trim());
  }
  flushParagraph();
  flushQuote();
  closeList();
  flushTable();
  return output.join("\n").trimEnd();
}

/** Render reference prose, adding ordered-list support without changing lesson rendering. */
function renderReferenceMarkdown(
  markdown: string,
  options?: InlineRenderOptionsInput,
): string {
  const lines = markdown.split(/\r?\n/);
  const output: string[] = [];
  const ordinary: string[] = [];
  const flushOrdinary = (): void => {
    if (ordinary.length === 0) return;
    const rendered = renderMarkdown(ordinary.join("\n"), options, "records");
    if (rendered !== "") output.push(rendered, "");
    ordinary.length = 0;
  };

  let cursor = 0;
  while (cursor < lines.length) {
    const first = /^(\d+)[.)]\s+(.+)$/.exec((lines[cursor] ?? "").trimEnd());
    if (!first) {
      ordinary.push(lines[cursor] ?? "");
      cursor += 1;
      continue;
    }

    flushOrdinary();
    output.push(
      "\\begin{enumerate}",
      "\\raggedright",
      "\\setlength{\\itemsep}{0.35em}",
    );
    while (cursor < lines.length) {
      const item = /^(\d+)[.)]\s+(.+)$/.exec((lines[cursor] ?? "").trimEnd());
      if (!item) break;
      const content = [item[2] ?? ""];
      cursor += 1;
      while (cursor < lines.length && /^\s+\S/.test(lines[cursor] ?? "")) {
        content.push((lines[cursor] ?? "").trim());
        cursor += 1;
      }
      output.push(`  \\item ${renderInlineMarkdown(content.join(" "), options)}`);
    }
    output.push("\\end{enumerate}", "");
  }
  flushOrdinary();
  return output.join("\n").trimEnd();
}

function renderBlock(block: LessonBodyBlock, options?: InlineRenderOptionsInput): string {
  const content = renderMarkdown(bookVoice(block.markdown), options);
  const title = renderInlineMarkdown(bookBlockTitle(block.title), options);
  if (block.type === "pronunciation") return `\\begin{sounds}\n${content}\n\\end{sounds}`;
  if (block.type === "etymology") return `\\begin{cousinweb}\n${content}\n\\end{cousinweb}`;
  if (block.type === "cognates") {
    // Deliberately an INLINE tcolorbox rather than `\\begin{cognates}`. The four
    // Dravidian preambles define a `cognates` environment and the other nineteen
    // do not, so emitting the named environment would compile here and break the
    // moment any other track authored the heading. `tcolorbox` is loaded by every
    // preamble, so the colours are carried in the emitted LaTeX instead -- and
    // they are the same violet the hand-written Kannada chapters used, so the
    // page is unchanged.
    return [
      "\\begin{tcolorbox}[breakable,skin=enhanced,colback=violet!6,colframe=violet!45!black," +
        `boxrule=0.5pt,arc=1mm,left=6pt,right=6pt,top=4pt,bottom=4pt,fonttitle=\\bfseries,title={${title}}]`,
      content,
      "\\end{tcolorbox}",
    ].join("\n");
  }
  if (block.type === "grammar" || block.type === "notice") {
    return `\\begin{grammarlens}[title={${title}}]\n${content}\n\\end{grammarlens}`;
  }
  if (block.type === "comprehension") {
    // Rendered as an inline tcolorbox rather than a named environment on
    // purpose: every named environment (culture, grammarlens) is defined in a
    // per-track preamble.tex, and there are 23 of those, all hand-written --
    // exactly the files this programme is retiring. An inline box needs no
    // preamble edit, so a reading passage can appear in any track's book the
    // day that track authors one.
    const readingTitle = BOOK_BLOCK_TITLES.comprehension;
    return [
      "\\begin{tcolorbox}[breakable,skin=enhanced,colback=orange!6,colframe=orange!45!black," +
        `boxrule=0.5pt,arc=1mm,left=6pt,right=6pt,top=4pt,bottom=4pt,fonttitle=\\bfseries,title={${readingTitle}}]`,
      content,
      "\\end{tcolorbox}",
    ].join("\n");
  }
  if (block.type === "culture-pragmatics") return `\\begin{culture}\n${content}\n\\end{culture}`;
  if (block.type === "warmup") {
    // BOOK_BLOCK_TITLES.warmup is deliberately empty: the indented lead-in is
    // the label. Guard anyway so a future non-empty value cannot be dropped.
    const label = BOOK_BLOCK_TITLES.warmup === "" ? "" : `\\textbf{${BOOK_BLOCK_TITLES.warmup}.} `;
    return `\\begin{quote}\n${label}${content}\n\\end{quote}`;
  }
  if (block.type === "recall") {
    return [
      "\\begin{tcolorbox}[breakable,colback=teal!4,colframe=teal!35!black," +
        `title={${BOOK_BLOCK_TITLES.recall}}]`,
      content,
      "\\end{tcolorbox}",
    ].join("\n");
  }
  return `\\subsection*{${title}}\n${content}`;
}

function lessonTitle(lesson: ParsedLesson): string {
  const firstLine = lesson.preamble.split(/\r?\n/).find((line) => line.trim() !== "") ?? "";
  return firstLine.startsWith("# ") ? firstLine.slice(2).trim() : lesson.realization.headword;
}

function lessonSequence(lesson: ParsedLesson): number {
  return Number(stringValue(lesson.frontmatter.sequence));
}

/**
 * How wide a short title may be before it is cut down, in display columns.
 *
 * A section's short title goes to the table of contents and the running head,
 * and both are one line wide. A headword that overflows there is not merely
 * ugly: `\@dottedtocline` sets `\parfillskip -\rightskip`, cancelling the
 * ragged-right stretch on the entry's LAST line, so a wrapped entry has to be
 * justified -- and a script with no hyphenation patterns cannot do that without
 * a badly stretched line. kannada and telugu each carried one.
 *
 * 40 is the corpus's 99th percentile: across the 1,663 non-practice lessons the
 * median short title is 7 columns wide and the 95th percentile is 23, so this
 * touches only the tail. That tail is month lists and weekday lists -- the
 * twelve Malayalam months, the seven Kannada weekdays -- which no table of
 * contents should be carrying in full anyway. A TOC line is a pointer, not the
 * content.
 */
const SHORT_TITLE_MAX_COLUMNS = 40;

/**
 * Estimate how many columns a string occupies.
 *
 * This is a proxy, not a measurement -- only XeLaTeX knows real widths, and
 * they differ per font. Combining marks are counted as zero because they stack
 * on the character before them, and East Asian wide forms as two. The proxy is
 * good enough to rank titles, which is all a cut-off needs; whether the cut-off
 * actually fixed the page is settled by building the book, not by this
 * function.
 */
function displayColumns(text: string): number {
  // East Asian Wide and Fullwidth ranges, written as escapes rather than as the
  // characters themselves so the source stays reviewable in a plain diff.
  const wide =
    /[\u1100-\u115F\u2E80-\uA4CF\uAC00-\uD7A3\uF900-\uFAFF\uFE30-\uFE4F\uFF00-\uFF60\uFFE0-\uFFE6]/u;
  let columns = 0;
  for (const character of text) {
    if (/\p{Mn}|\p{Me}/u.test(character)) continue;
    columns += wide.test(character) ? 2 : 1;
  }
  return columns;
}

/**
 * Cut a short title to the width budget at a word boundary, marking the cut.
 *
 * Cutting happens on the authored text, BEFORE the Markdown is rendered, so a
 * cut can never land inside a `\textbf{...}` the renderer has produced. It can
 * still land inside authored `**emphasis**`, so a truncation that leaves an odd
 * number of `*` or `_` runs drops one more word rather than emitting markup the
 * renderer would mis-pair.
 */
function truncateShortTitle(title: string, budget: number = SHORT_TITLE_MAX_COLUMNS): string {
  if (displayColumns(title) <= budget) return title;
  // A list written without spaces -- Japanese `て・みみ・くち・…` -- is one
  // "word" to the loop below and would never be cut. Its items are separated
  // by the middle dot, so cut there instead.
  if (!title.includes(" ") && title.includes("\u30fb")) {
    const items = title.split("\u30fb");
    const kept: string[] = [];
    for (const item of items) {
      if (displayColumns([...kept, item].join("\u30fb")) + 2 > budget) break;
      kept.push(item);
    }
    if (kept.length > 0 && kept.length < items.length) return `${kept.join("\u30fb")} \u2026`;
  }
  const words = title.split(" ");
  const balanced = (text: string): boolean =>
    (text.match(/\*\*/g)?.length ?? 0) % 2 === 0 &&
    (text.match(/(?<!\*)\*(?!\*)/g)?.length ?? 0) % 2 === 0 &&
    (text.match(/_/g)?.length ?? 0) % 2 === 0;
  let kept: string[] = [];
  for (const word of words) {
    const candidate = [...kept, word].join(" ");
    // The ellipsis costs two columns of its own: one for the character, one for
    // the space before it.
    if (displayColumns(candidate) + 2 > budget) break;
    kept.push(word);
  }
  while (kept.length > 1 && !balanced(kept.join(" "))) kept = kept.slice(0, -1);
  // Do not hand the ellipsis a dangling separator. Cutting `di - haz - ve -
  // pon - ten - sal - se - ven` between items leaves the separator that was
  // joining them to the item now gone, and `sal - se - ...` reads as though
  // something were missing from the middle rather than trimmed from the end.
  // The same goes for a trailing comma in `dies Lunae, Martis, Iovis, ...`.
  while (kept.length > 1 && /^[\u00b7\u2014\u2013/|,;:]+$/u.test(kept[kept.length - 1]!)) {
    kept = kept.slice(0, -1);
  }
  if (kept.length > 0) kept[kept.length - 1] = kept[kept.length - 1]!.replace(/[,;:]+$/u, "");
  // A single word wider than the whole budget cannot be cut at a word boundary.
  // Keep it whole: a truncated word is unreadable, and one long word is a
  // narrower defect than a wrapped list.
  //
  // Two kinds of title land here, and the second is a real limit rather than an
  // edge case. One is a genuinely long word. The other is any script that does
  // not separate words with spaces -- Chinese, Japanese, Thai -- where the whole
  // title is one "word" and so is never cut at all. Those books are short today
  // and none of them warns, but if one ever does, the fix is a script-aware
  // break rule and not a bigger budget.
  if (kept.length === 0) return words[0] ?? title;
  return `${kept.join(" ")} \u2026`;
}

/**
 * The contents line for one lesson: the headword in its own script, with its
 * romanization beside it -- `கண் (kaṇ)`.
 *
 * A contents page is where a reader first meets a book's words, and a line
 * that shows only the romanization hides the script the book teaches; one
 * that shows only the script is unreadable to a beginner. Both, in that
 * order, is the same pairing every lesson heading uses.
 *
 * The pair shares the one-line budget. When it does not fit whole, each half
 * is cut to half the budget (less the ` ()` that joins them) at a word
 * boundary, so a long weekday list keeps the start of both its forms rather
 * than all of one and none of the other.
 *
 * Only the headword is shown when there is nothing to pair: a practice
 * lesson, a track with no target script, a lesson with no romanization, a
 * romanization identical to the headword, or a bracketed placeholder
 * headword such as `(recap)` or `(X, continued)`, whose romanization only
 * repeats the placeholder.
 */
function sectionShortTitle(lesson: ParsedLesson, options?: InlineRenderOptionsInput): string {
  const clean = (text: string): string =>
    text.replaceAll("←", " from ").replaceAll("→", " to ").replace(/\s+/g, " ").trim();
  if (lesson.realization.type.startsWith("practice")) {
    return renderInlineMarkdown("Practice", options);
  }
  const headword = clean(lesson.realization.headword);
  const romanization = clean(stringValue(lesson.frontmatter.romanization));
  const paired =
    options !== undefined &&
    romanization !== "" &&
    romanization !== headword &&
    !headword.startsWith("(");
  // The short title is also the PDF bookmark. A bookmark cannot hold a font
  // command such as `\ta{...}`, and not every preamble maps its script
  // commands away inside PDF strings (Persian, Russian and Urdu warned once per
  // lesson). `\texorpdfstring` gives the bookmark the plain text instead, in
  // every book, whatever its preamble says.
  const bookmarked = (title: string): string => {
    const page = renderInlineMarkdown(title, options);
    const plain = renderInlineMarkdown(title);
    return page === plain ? page : `\\texorpdfstring{${page}}{${plain}}`;
  };
  if (!paired) return bookmarked(truncateShortTitle(headword));
  // ` (` and `)` cost three columns between them.
  const whole = `${headword} (${romanization})`;
  const half = Math.floor((SHORT_TITLE_MAX_COLUMNS - 3) / 2);
  const title = displayColumns(whole) <= SHORT_TITLE_MAX_COLUMNS
    ? whole
    : `${truncateShortTitle(headword, half)} (${truncateShortTitle(romanization, half)})`;
  return bookmarked(title);
}

interface InlineRenderTarget {
  inlineScripts?: InlineRenderOptions[];
  unicodeScript?: string;
  scriptCommand?: string;
}

function targetRenderOptions(
  target: InlineRenderTarget,
  description: string,
): InlineRenderOptionsInput | undefined {
  if (target.inlineScripts !== undefined) {
    if (target.unicodeScript !== undefined || target.scriptCommand !== undefined) {
      throw new Error(
        `${description}: inlineScripts cannot be combined with unicodeScript or scriptCommand`,
      );
    }
    if (target.inlineScripts.length === 0) {
      throw new Error(`${description}: inlineScripts must not be empty`);
    }
    return target.inlineScripts;
  }
  if (target.unicodeScript === undefined && target.scriptCommand === undefined) return undefined;
  if (target.unicodeScript === undefined || target.scriptCommand === undefined) {
    throw new Error(
      `${description}: unicodeScript and scriptCommand must be declared together`,
    );
  }
  return { unicodeScript: target.unicodeScript, scriptCommand: target.scriptCommand };
}

/** Render one canonical Markdown pronunciation/script reference as book back matter. */
export function renderReferenceAppendix(
  target: BookReferenceAppendixTarget,
  markdown: string,
): string {
  const renderOptions = targetRenderOptions(target, `${target.language} reference appendix`);
  const lines = markdown.replaceAll("\r\n", "\n").split("\n");
  const firstContent = lines.findIndex((line) => line.trim() !== "");
  if (firstContent < 0 || !/^#\s+\S/.test(lines[firstContent] ?? "")) {
    throw new Error(`${target.source}: reference must begin with a level-one Markdown heading`);
  }
  lines.splice(firstContent, 1);

  // The chapter title, the table-of-contents line and the running head are three
  // different jobs, and a book that was hand-set gave them three different
  // strings: `The Tamil script --- a reference` over the page, `The Tamil script
  // (reference)` in the contents (an em-dash reads as a hyphen at TOC size and
  // the entry has one line to fit in), and a bare `Tamil script` in the running
  // head, where "Pronunciation" would be a lie about a chapter that is mostly a
  // writing system. Both overrides are optional and both default to the shape
  // the first six generated references already emit, so those files are
  // unchanged byte for byte.
  //
  // A declared-but-blank override is rejected rather than defaulted. `?? ` reads
  // `""` as a deliberate value, which would blank every running head in the
  // appendix or empty its contents line, and a reference whose contents entry is
  // an empty dotted rule is not a failure anyone would go looking for.
  const override = (value: string | undefined, field: string): string | undefined => {
    if (value === undefined) return undefined;
    if (value.trim() === "") {
      throw new Error(`${target.language} reference appendix: ${field} must not be blank`);
    }
    return value;
  };
  const runningHead = renderInlineMarkdown(
    override(target.runningHead, "runningHead") ?? "Pronunciation",
    renderOptions,
  );
  const shortTitle = override(target.shortTitle, "shortTitle") ?? target.title;

  const output = [
    `% GENERATED FILE. Edit ${target.source}, then run npm run generate:books.`,
    "",
    `\\chapter*{${renderInlineMarkdown(target.title, renderOptions)}}`,
    `\\addcontentsline{toc}{chapter}{${renderInlineMarkdown(shortTitle, renderOptions)}}`,
    `\\markboth{${runningHead}}{${runningHead}}`,
    "",
  ];
  const body: string[] = [];
  const flushBody = (): void => {
    const rendered = renderReferenceMarkdown(body.join("\n"), renderOptions);
    if (rendered !== "") output.push(rendered, "");
    body.length = 0;
  };

  for (const line of lines) {
    const heading = /^(#{2,3})\s+(.+)$/.exec(line.trimEnd());
    if (!heading) {
      if (/^#\s+/.test(line)) {
        throw new Error(`${target.source}: reference may contain only one level-one heading`);
      }
      body.push(line);
      continue;
    }
    flushBody();
    const command = (heading[1] ?? "").length === 2 ? "section" : "subsection";
    output.push(
      `\\${command}*{${renderInlineMarkdown(heading[2] ?? "", renderOptions)}}`,
      "",
    );
  }
  flushBody();
  return `${output.join("\n").trimEnd()}\n`;
}

interface BookGlossaryEntry {
  headword: string;
  romanization: string;
  gloss: string;
  chapters: number[];
}

function glossarySortKey(value: string): string {
  return value
    .normalize("NFKD")
    .replace(/\p{Mark}/gu, "")
    .toLowerCase();
}

function compareGlossaryEntries(left: BookGlossaryEntry, right: BookGlossaryEntry): number {
  const leftKey = glossarySortKey(left.romanization || left.headword);
  const rightKey = glossarySortKey(right.romanization || right.headword);
  if (leftKey < rightKey) return -1;
  if (leftKey > rightKey) return 1;
  if (left.headword < right.headword) return -1;
  if (left.headword > right.headword) return 1;
  if (left.gloss < right.gloss) return -1;
  if (left.gloss > right.gloss) return 1;
  return 0;
}

function chapterList(chapters: number[]): string {
  const labels = chapters.map(String);
  if (labels.length === 1) return `Chapter ${labels[0]}`;
  if (labels.length === 2) return `Chapters ${labels[0]} and ${labels[1]}`;
  return `Chapters ${labels.slice(0, -1).join(", ")}, and ${labels.at(-1)}`;
}

/** Render one track's canonical words and phrases as compact, page-safe book back matter. */
export function renderBookGlossary(
  target: BookGlossaryTarget,
  allLessons: ParsedLesson[],
): string {
  const renderOptions = targetRenderOptions(target, `${target.language} glossary`);
  const entries = new Map<string, BookGlossaryEntry>();
  const lessons = allLessons.filter(
    (lesson) =>
      lesson.language === target.language && CONTENT_TYPES.has(lesson.realization.type),
  );

  for (const lesson of lessons) {
    const headword = lesson.realization.headword.trim();
    const romanization = lesson.realization.romanization.trim();
    const gloss = lesson.realization.gloss.trim();
    if (headword === "" || gloss === "") {
      throw new Error(
        `${lesson.realization.lessonId}: glossary entries require a headword and gloss`,
      );
    }
    if (!Number.isInteger(lesson.realization.chapter) || lesson.realization.chapter < 1) {
      throw new Error(`${lesson.realization.lessonId}: glossary entries require a chapter`);
    }
    const key = JSON.stringify([headword, romanization, gloss]);
    const existing = entries.get(key);
    if (existing) {
      if (!existing.chapters.includes(lesson.realization.chapter)) {
        existing.chapters.push(lesson.realization.chapter);
      }
      continue;
    }
    entries.set(key, {
      headword,
      romanization,
      gloss,
      chapters: [lesson.realization.chapter],
    });
  }
  if (entries.size === 0) {
    throw new Error(`${target.language} glossary: no canonical word or phrase lessons`);
  }

  const ordered = [...entries.values()].sort(compareGlossaryEntries);
  const output = [
    "% GENERATED FILE. Edit canonical lesson frontmatter, then run npm run generate:books.",
    `% canonical-entries: ${ordered.length}`,
    "",
    "\\chapter*{Glossary}",
    "\\addcontentsline{toc}{chapter}{Glossary}",
    "\\markboth{Glossary}{Glossary}",
    "",
    "This glossary collects every word and phrase taught in the book. Pronunciation appears when it differs from the written form; chapter numbers show where each entry is introduced.",
    "",
  ];

  for (const entry of ordered) {
    const headword = renderInlineMarkdown(entry.headword, renderOptions);
    const showRomanization =
      entry.romanization !== "" &&
      glossarySortKey(entry.romanization) !== glossarySortKey(entry.headword);
    const romanization = showRomanization
      ? `\\enspace\\emph{${renderInlineMarkdown(entry.romanization, renderOptions)}}`
      : "";
    const gloss = renderInlineMarkdown(entry.gloss, renderOptions);
    output.push(
      "\\noindent\\begin{minipage}[t]{\\linewidth}",
      "\\raggedright",
      `\\textbf{${headword}}${romanization}\\par`,
      `\\small ${gloss}\\par`,
      `\\footnotesize Introduced in ${chapterList(entry.chapters.sort((a, b) => a - b))}.`,
      "\\end{minipage}\\par\\medskip",
      "",
    );
  }
  return `${output.join("\n").trimEnd()}\n`;
}

interface BookAnswerKeyEntry {
  activity: CompiledLessonActivity;
  chapter: number;
  number: string;
  lessonTitle: string;
}

/**
 * Render the same executable retrieval contracts used by Language Ladder as
 * printable end-of-book review questions and a separate answer key.
 *
 * The prompt is repeated in the review section because some canonical lessons
 * still sit inside handwritten LaTeX chapters. Scraping those chapters or the
 * legacy `[YOU ...]` delivery cues would create a second, untyped definition of
 * correctness. Compiled activities are the only answer-bearing source.
 */
export function renderBookAnswerKey(
  target: BookAnswerKeyTarget,
  allLessons: ParsedLesson[],
): string {
  const renderOptions = targetRenderOptions(target, `${target.language} answer key`);
  const lessons = allLessons
    .filter((lesson) => lesson.language === target.language)
    .sort(
      (left, right) =>
        left.realization.chapter - right.realization.chapter ||
        lessonSequence(left) - lessonSequence(right) ||
        left.realization.lessonId.localeCompare(right.realization.lessonId),
    );
  const chapterCounts = new Map<number, number>();
  const activityIds = new Set<string>();
  const entries: BookAnswerKeyEntry[] = [];

  for (const lesson of lessons) {
    const chapter = lesson.realization.chapter;
    if (!Number.isInteger(chapter) || chapter < 1) {
      throw new Error(`${lesson.realization.lessonId}: answer-key entries require a chapter`);
    }
    for (const activity of compileLessonActivities(lesson.blocks)) {
      if (activityIds.has(activity.id)) {
        throw new Error(`${target.language} answer key: duplicate activity id '${activity.id}'`);
      }
      activityIds.add(activity.id);
      const inChapter = (chapterCounts.get(chapter) ?? 0) + 1;
      chapterCounts.set(chapter, inChapter);
      entries.push({
        activity,
        chapter,
        number: `${chapter}.${inChapter}`,
        lessonTitle: lessonTitle(lesson),
      });
    }
  }
  if (entries.length === 0) {
    throw new Error(`${target.language} answer key: no compiled lesson activities`);
  }

  const output = [
    "% GENERATED FILE. Edit canonical hl-activity contracts, then run npm run generate:books.",
    `% canonical-activities: ${entries.length}`,
    "",
    "\\chapter*{Review Questions}",
    "\\addcontentsline{toc}{chapter}{Review Questions}",
    "\\markboth{Review Questions}{Review Questions}",
    "",
    "Try these without looking ahead. Every prompt comes from the same canonical lesson data as its chapter. Answers begin in the next section.",
    "",
  ];
  let currentChapter: number | undefined;
  for (const entry of entries) {
    if (entry.chapter !== currentChapter) {
      currentChapter = entry.chapter;
      output.push(`\\section*{Chapter ${entry.chapter}}`, "");
    }
    const prompt = renderInlineMarkdown(entry.activity.prompt, renderOptions);
    const title = renderInlineMarkdown(entry.lessonTitle, renderOptions);
    output.push(
      "\\noindent\\begin{minipage}[t]{\\linewidth}",
      "\\raggedright",
      `\\hypertarget{review-${entry.activity.id}}{\\textbf{${entry.number}}} \\emph{${title}}\\par`,
      `\\small ${prompt}`,
      "\\end{minipage}\\par\\medskip",
      "",
    );
  }

  output.push(
    "\\chapter*{Answer Key}",
    "\\addcontentsline{toc}{chapter}{Answer Key}",
    "\\markboth{Answer Key}{Answer Key}",
    "",
    "The first response is the canonical display answer. When a question accepts other authored forms, they appear underneath.",
    "",
  );
  currentChapter = undefined;
  for (const entry of entries) {
    if (entry.chapter !== currentChapter) {
      currentChapter = entry.chapter;
      output.push(`\\section*{Chapter ${entry.chapter}}`, "");
    }
    const answer = renderInlineMarkdown(entry.activity.answer, renderOptions);
    const title = renderInlineMarkdown(entry.lessonTitle, renderOptions);
    const accepted = entry.activity.accepted.map((variant) =>
      renderInlineMarkdown(variant, renderOptions),
    );
    output.push(
      "\\noindent\\begin{minipage}[t]{\\linewidth}",
      "\\raggedright",
      `\\hyperlink{review-${entry.activity.id}}{\\textbf{${entry.number}}} \\emph{${title}}\\par`,
      `\\small \\textbf{Answer:} ${answer}\\par`,
      ...(accepted.length > 0
        ? [`\\footnotesize \\textbf{Also accepted:} ${accepted.join("; ")}`]
        : []),
      "\\end{minipage}\\par\\medskip",
      "",
    );
  }
  return `${output.join("\n").trimEnd()}\n`;
}

interface BookIndexEntry {
  term: string;
  headword: string;
  romanization: string;
  descriptor: string;
  facets: string[];
  chapters: number[];
}

const INDEX_LESSON_TYPES = new Map<string, string>([
  ["grammar", "grammar topic"],
  ["pattern", "productive pattern"],
  ["writing", "script and writing topic"],
  ["etymology", "etymology topic"],
  ["culture", "culture and usage topic"],
  ["pronunciation", "pronunciation topic"],
]);

const INDEX_BLOCK_FACETS = new Map<LessonBodyBlock["type"], string>([
  ["pronunciation", "pronunciation"],
  ["script", "script"],
  ["writing", "writing"],
  ["grammar", "grammar"],
  ["etymology", "etymology"],
  ["cognates", "family and cognates"],
  ["culture-pragmatics", "usage and culture"],
  ["comprehension", "reading"],
]);

const INDEX_FACET_ORDER = [
  "pronunciation",
  "script",
  "writing",
  "grammar",
  "etymology",
  "family and cognates",
  "usage and culture",
  "reading",
];

function indexSortKey(value: string): string {
  return glossarySortKey(value)
    .replace(/[*_`]/g, "")
    .replace(/[^\p{Letter}\p{Number}]+/gu, " ")
    .trim();
}

function compareBookIndexEntries(left: BookIndexEntry, right: BookIndexEntry): number {
  const leftKey = indexSortKey(left.term);
  const rightKey = indexSortKey(right.term);
  if (leftKey < rightKey) return -1;
  if (leftKey > rightKey) return 1;
  const leftDetail = indexSortKey(left.headword || left.descriptor);
  const rightDetail = indexSortKey(right.headword || right.descriptor);
  if (leftDetail < rightDetail) return -1;
  if (leftDetail > rightDetail) return 1;
  return 0;
}

function indexGroup(term: string): string {
  const key = indexSortKey(term);
  const first = key[0] ?? "";
  if (/^[a-z]$/i.test(first)) return first.toUpperCase();
  if (/^\d$/.test(first)) return "0--9";
  return "Other";
}

/**
 * Render a compact, English-first subject index from canonical curriculum data.
 *
 * The glossary already provides target-language lookup. This complementary view
 * starts from English meanings, dedicated topic lessons, and chapter titles. It
 * never mines prose for guessed keywords, and it deliberately excludes practice
 * drills: retrieval belongs in the review appendix, not in a subject index.
 */
export function renderBookIndex(
  target: BookIndexTarget,
  allLessons: ParsedLesson[],
  allChapters: BookIndexChapter[],
): string {
  const renderOptions = targetRenderOptions(target, `${target.language} index`);
  const chapters = allChapters
    .filter((chapter) => Number.isInteger(chapter.chapter) && chapter.chapter > 0)
    .sort((left, right) => left.chapter - right.chapter);
  if (chapters.length === 0) {
    throw new Error(`${target.language} index: no canonical chapter capabilities`);
  }
  const chapterByNumber = new Map<number, BookIndexChapter>();
  for (const chapter of chapters) {
    if (chapter.label.trim() === "") {
      throw new Error(`${target.language} index: chapter ${chapter.chapter} has no label`);
    }
    if (chapterByNumber.has(chapter.chapter)) {
      throw new Error(`${target.language} index: duplicate chapter ${chapter.chapter}`);
    }
    chapterByNumber.set(chapter.chapter, chapter);
  }

  const entries = new Map<string, BookIndexEntry>();
  let candidates = 0;
  const addEntry = (entry: Omit<BookIndexEntry, "chapters">, chapter: number): void => {
    if (!chapterByNumber.has(chapter)) {
      throw new Error(`${target.language} index: chapter ${chapter} is not in the capability ledger`);
    }
    const key = JSON.stringify([
      entry.term,
      entry.headword,
      entry.romanization,
      entry.descriptor,
    ]);
    const existing = entries.get(key);
    if (existing) {
      if (!existing.chapters.includes(chapter)) existing.chapters.push(chapter);
      existing.facets = [...new Set([...existing.facets, ...entry.facets])].sort(
        (left, right) => INDEX_FACET_ORDER.indexOf(left) - INDEX_FACET_ORDER.indexOf(right),
      );
      return;
    }
    entries.set(key, { ...entry, chapters: [chapter] });
  };

  for (const chapter of chapters) {
    candidates += 1;
    addEntry(
      {
        term: chapter.title.trim(),
        headword: "",
        romanization: "",
        descriptor: "chapter topic",
        facets: [],
      },
      chapter.chapter,
    );
  }

  const lessons = allLessons
    .filter((lesson) => lesson.language === target.language)
    .sort(
      (left, right) =>
        left.realization.chapter - right.realization.chapter ||
        lessonSequence(left) - lessonSequence(right) ||
        left.realization.lessonId.localeCompare(right.realization.lessonId),
    );
  for (const lesson of lessons) {
    const chapter = lesson.realization.chapter;
    const facets = [...new Set(lesson.blocks.flatMap((block) => {
      const facet = INDEX_BLOCK_FACETS.get(block.type);
      return facet ? [facet] : [];
    }))].sort(
      (left, right) => INDEX_FACET_ORDER.indexOf(left) - INDEX_FACET_ORDER.indexOf(right),
    );
    if (CONTENT_TYPES.has(lesson.realization.type)) {
      const term = lesson.realization.gloss.trim();
      const headword = lesson.realization.headword.trim();
      if (term === "" || headword === "") {
        throw new Error(`${lesson.realization.lessonId}: index entries require a headword and gloss`);
      }
      candidates += 1;
      addEntry(
        {
          term,
          headword,
          romanization: lesson.realization.romanization.trim(),
          descriptor: "",
          facets,
        },
        chapter,
      );
      continue;
    }
    const descriptor = INDEX_LESSON_TYPES.get(lesson.realization.type);
    if (!descriptor) continue;
    const term = lessonTitle(lesson).trim();
    if (term === "") {
      throw new Error(`${lesson.realization.lessonId}: topic index entry has no title`);
    }
    candidates += 1;
    addEntry(
      {
        term,
        headword: "",
        romanization: "",
        descriptor,
        facets,
      },
      chapter,
    );
  }
  if (entries.size === 0) throw new Error(`${target.language} index: no canonical entries`);

  const ordered = [...entries.values()].sort(compareBookIndexEntries);
  const output = [
    "% GENERATED FILE. Edit canonical lessons or chapters.json, then run npm run generate:books.",
    `% canonical-index-candidates: ${candidates}`,
    `% canonical-index-entries: ${ordered.length}`,
    "",
    "\\chapter*{Index}",
    "\\addcontentsline{toc}{chapter}{Index}",
    "\\markboth{Index}{Index}",
    "",
    "Look up an English meaning, a dedicated language topic, or a chapter topic. Each linked reference opens the chapter where the material is introduced; the focus labels name only explicitly typed lesson sections.",
    "",
  ];
  let currentGroup: string | undefined;
  for (const entry of ordered) {
    const group = indexGroup(entry.term);
    if (group !== currentGroup) {
      currentGroup = group;
      output.push(`\\section*{${group}}`, "");
    }
    const term = renderInlineMarkdown(entry.term, renderOptions);
    const headword = entry.headword === ""
      ? ""
      : `\\enspace\\emph{${renderInlineMarkdown(entry.headword, renderOptions)}}`;
    const showRomanization =
      entry.romanization !== "" &&
      glossarySortKey(entry.romanization) !== glossarySortKey(entry.headword);
    const romanization = showRomanization
      ? `\\enspace(${renderInlineMarkdown(entry.romanization, renderOptions)})`
      : "";
    const metadata = [
      ...(entry.descriptor === "" ? [] : [entry.descriptor]),
      ...(entry.facets.length === 0 ? [] : [`explicit focus: ${entry.facets.join(", ")}`]),
    ];
    const references = entry.chapters
      .sort((left, right) => left - right)
      .map((chapterNumber) => {
        const chapter = chapterByNumber.get(chapterNumber)!;
        return `\\hyperref[${chapter.label}]{Chapter~${chapterNumber}, p.~\\pageref*{${chapter.label}}}`;
      })
      .join("; ");
    output.push(
      "\\noindent\\begin{minipage}[t]{\\linewidth}",
      "\\raggedright",
      `\\textbf{${term}}${headword}${romanization}\\par`,
      `\\footnotesize ${metadata.length > 0 ? `${metadata.join("; ")}; ` : ""}${references}`,
      "\\end{minipage}\\par\\smallskip",
      "",
    );
  }
  return `${output.join("\n").trimEnd()}\n`;
}

/** Render one configured chapter from the same typed lesson AST the app receives. */
/**
 * The chapter opening a reader actually wants: what they will be able to do.
 *
 * DERIVED from the HL05 capability ledger, never authored into the .tex — 302
 * hand-written intros would be 302 places to drift from the lessons they describe,
 * and the generated file says at the top that editing it is pointless.
 *
 * It must stand alone in English. HL09 §8 is explicit, and the handwritten chapters
 * show why: several open with cross-track references — "the same wearing-down the
 * Hindi track shows", "every other track in this course" — which are simply dangling
 * pointers to a reader holding one language's PDF. English is the only requirement
 * for any book here, so an intro may never lean on another track.
 *
 * `canDo` is already first-person ("I can greet someone in Spanish…"), so it is
 * quoted as the goal rather than reflowed into second person; rewriting it would
 * make the book and the ledger disagree about the same sentence.
 */
function chapterIntro(
  capability: ChapterCapability | undefined,
  options?: InlineRenderOptionsInput,
): string[] {
  if (!capability?.canDo) return [];
  const goal = renderInlineMarkdown(capability.canDo, options);
  const payoff = capability.payoff?.summary
    ? renderInlineMarkdown(capability.payoff.summary, options)
    : "";
  return [
    "\\begin{chapteropening}",
    `\\textbf{By the end of this chapter:} \\emph{${goal}}`,
    ...(payoff ? ["", `${payoff}`] : []),
    "\\end{chapteropening}",
    "",
  ];
}

export function renderBookChapter(
  target: BookGenerationTarget,
  allLessons: ParsedLesson[],
  capability?: ChapterCapability,
): GeneratedBookChapter {
  const renderOptions = targetRenderOptions(target, `${target.language} chapter ${target.chapter}`);
  const lessons = allLessons
    .filter(
      (lesson) =>
        lesson.language === target.language && lesson.realization.chapter === target.chapter,
    )
    .sort(
      (left, right) =>
        lessonSequence(left) - lessonSequence(right) ||
        left.realization.lessonId.localeCompare(right.realization.lessonId),
    );
  if (lessons.length === 0) throw new Error(`${target.language} chapter ${target.chapter}: no lessons`);
  for (const lesson of lessons) {
    if (stringValue(lesson.frontmatter.schema_version) !== "2") {
      throw new Error(`${lesson.realization.lessonId}: generated books require schema version 2`);
    }
    if (!Number.isInteger(lessonSequence(lesson))) {
      throw new Error(`${lesson.realization.lessonId}: generated books require an integer sequence`);
    }
    if (lesson.blocks.some((block) => block.type === "unknown")) {
      throw new Error(`${lesson.realization.lessonId}: generated books require known body blocks`);
    }
  }

  const sourceHash = canonicalChapterHash(lessons, capability);
  const sections = lessons.map((lesson) => {
    const id = lesson.realization.lessonId;
    return [
      `\\section[${sectionShortTitle(lesson, renderOptions)}]{${renderInlineMarkdown(lessonTitle(lesson), renderOptions)}}`,
      `\\label{lesson:${id}}`,
      "",
      ...lesson.blocks.map((block) => renderBlock(block, renderOptions)),
    ].join("\n\n");
  });
  const tex = [
    "% GENERATED FILE. Edit canonical lessons, then run npm run generate:books.",
    `% canonical-source-hash: ${sourceHash}`,
    `% canonical-lessons: ${lessons.map((lesson) => lesson.realization.lessonId).join(", ")}`,
    "",
    `\\chapter{${renderInlineMarkdown(target.title, renderOptions)}}`,
    `\\label{${target.label}}`,
    `\\hlchaptermodality{${target.chapter}}`,
    "",
    // The blurb that used to sit here explained how the chapter was PRODUCED
    // ("generated from the canonical micro-lessons...") — true, and of no interest
    // whatsoever to somebody who just wants to learn Spanish. Books do not describe
    // their own build system. Removing it was right; leaving nothing was not, and
    // 288 of 407 chapters have opened on a bare title ever since.
    ...chapterIntro(capability, renderOptions),
    ...sections,
    "",
  ].join("\n");
  return {
    tex,
    sourceHash,
    lessonIds: lessons.map((lesson) => lesson.realization.lessonId),
  };
}
