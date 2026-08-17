import { canonicalChapterHash } from "./hash.js";
import { CONTENT_TYPES, hasOwn } from "./constants.js";
import { compileLessonActivities } from "./activity.js";
import type {
  LessonBodyBlock,
  ChapterCapability,
  CompiledLessonActivity,
} from "./types.js";
import { stripHtmlComments } from "./literal-markup.js";
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
    const lessonNoun = chapter.lessonCount === 1 ? "lesson" : "lessons";
    const prefix =
      chapter.drivablePrefix === 0
        ? `none of the ${chapter.lessonCount} ${lessonNoun}`
        : chapter.drivablePrefix === chapter.lessonCount
          ? `all ${chapter.lessonCount} ${lessonNoun}`
          : `first ${chapter.drivablePrefix} of ${chapter.lessonCount} ${lessonNoun}`;
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
// Three cue shapes exist across the 1,096 lesson files, and every one of them
// sits at the start of a line or fills a whole bullet:
//
//   [PAUSE 2s]      "hold here" --- meaningless in print, so it is deleted.
//   [REPEAT x2]     "say the next bit twice" --- said in prose instead.
//   [YOU SAY: ...]  "the learner's turn" --- typeset as a practice prompt.
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
 * prototype. `YOU_CUE` already restricts the verb to A--Z and spaces, which
 * rules that out today; the guard means it stays ruled out if the cue grammar
 * is ever widened.
 */
function cueVoice(verb: string): CueVoice {
  const known = hasOwn(CUE_VOICES, verb) ? CUE_VOICES[verb] : undefined;
  return known ?? { item: verb.charAt(0) + verb.slice(1).toLowerCase() };
}

/** `[PAUSE 2s]` / `[PAUSE 1s each]`, always at the head of a line. */
const PAUSE_CUE = /^\[PAUSE [^\]]*\][ \t]*/;

/** `[REPEAT x2]`, always at the head of a line, always followed by the copy. */
const REPEAT_CUE = /^\[REPEAT x(\d+)\][ \t]*/;

/**
 * `[YOU SAY: ...]` filling a whole bullet. The closing bracket is anchored to
 * the end because the copy itself may contain brackets --- "what [is] your
 * name?" is a real lesson line --- so a lazy match would truncate it.
 */
const YOU_CUE = /^\[YOU ([A-Z]+(?: [A-Z]+)*):[ \t]*([\s\S]*)\]$/;

/** A Markdown bullet, matching renderMarkdown's own column-zero rule. */
const LIST_ITEM = /^- /;

/**
 * Strip the timing cues from one non-bullet line.
 *
 * Returns `undefined` when the line was *nothing but* a cue --- a bare
 * `[PAUSE 1s]` on its own line --- because the right print treatment of an
 * instruction to wait is no ink at all.
 */
function stripDeliveryCues(line: string): string | undefined {
  let text = line.replace(PAUSE_CUE, "");
  const repeat = REPEAT_CUE.exec(text);
  if (repeat) {
    const times = Number(repeat[1]);
    const lead = times === 2 ? "Twice through" : `${times} times through`;
    text = `*${lead}:* ${text.slice(repeat[0].length)}`;
  }
  if (line.trim() !== "" && text.trim() === "") return undefined;
  return text;
}

interface ParsedCue {
  verb: string;
  content: string;
}

function parseCue(item: string): ParsedCue | undefined {
  const match = YOU_CUE.exec(item.trim());
  if (!match) return undefined;
  return { verb: match[1] ?? "", content: (match[2] ?? "").trim() };
}

/**
 * Gather one Markdown list into logical items, folding each item's wrapped
 * continuation lines into it exactly the way renderMarkdown would (first line
 * after `- `, continuations trimmed, joined by a single space). Because the
 * join is identical, re-emitting an item on one line cannot change the LaTeX.
 */
function collectListItems(lines: string[], start: number): { items: string[]; next: number } {
  const items: string[] = [];
  let cursor = start;
  while (cursor < lines.length) {
    const line = (lines[cursor] ?? "").trimEnd();
    if (LIST_ITEM.test(line)) {
      items.push(line.slice(2));
      cursor += 1;
      continue;
    }
    if (items.length > 0 && /^\s+\S/.test(line)) {
      items[items.length - 1] = `${items.at(-1) ?? ""} ${line.trim()}`;
      cursor += 1;
      continue;
    }
    break;
  }
  return { items, next: cursor };
}

/**
 * Turn a collected list back into Markdown, in book voice.
 *
 * Uniform-verb lists of two or more bullets get the instruction once, above
 * the list. Anything else --- a lone cue, a mixed list, a list of ordinary
 * bullets --- keeps the per-bullet form, so no information is lost.
 */
function renderCueList(items: string[]): string[] {
  const cues = items.map(parseCue);
  if (cues.every((cue) => cue === undefined)) return items.map((item) => `- ${item}`);

  const verbs = new Set(cues.map((cue) => cue?.verb));
  const uniformVerb = verbs.size === 1 ? [...verbs][0] : undefined;
  if (uniformVerb !== undefined && items.length > 1) {
    const lead = cueVoice(uniformVerb).lead;
    if (lead !== undefined) {
      return [`${lead}:`, "", ...cues.map((cue) => `- ${cue?.content ?? ""}`)];
    }
  }
  return items.map((item, index) => {
    const cue = cues[index];
    if (!cue) return `- ${item}`;
    return `- *${cueVoice(cue.verb).item}:* ${cue.content}`;
  });
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
    const stripped = stripDeliveryCues(line);
    if (stripped !== undefined) output.push(stripped);
    cursor += 1;
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
    output.push("\\end{itemize}", "");
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
    output.push(
      "\\noindent",
      `\\begin{tabularx}{\\linewidth}{@{}${columns}@{}}`,
      "\\toprule",
      `${cells(header)
        .map((cell) => `\\textbf{${cell}}`)
        .join(" & ")} \\\\`,
      "\\midrule",
      ...body.map((row) => `${cells(row).join(" & ")} \\\\`),
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
    if (quote.length > 0 && /^\s+/.test(line)) {
      quote.push(line.trim());
      continue;
    }
    if (line.startsWith("- ")) {
      flushParagraph();
      flushQuote();
      if (!listOpen) {
        output.push("\\begin{itemize}", "\\raggedright");
        if (tableLayout === "records") output.push("\\setlength{\\itemsep}{0.35em}");
        listOpen = true;
      }
      flushListItem();
      listItem = [line.slice(2)];
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
  if (block.type === "grammar" || block.type === "notice") {
    return `\\begin{grammarlens}[title={${title}}]\n${content}\n\\end{grammarlens}`;
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
function truncateShortTitle(title: string): string {
  if (displayColumns(title) <= SHORT_TITLE_MAX_COLUMNS) return title;
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
    if (displayColumns(candidate) + 2 > SHORT_TITLE_MAX_COLUMNS) break;
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

function sectionShortTitle(lesson: ParsedLesson, options?: InlineRenderOptionsInput): string {
  const title = lesson.realization.type.startsWith("practice")
    ? "Practice"
    : options && stringValue(lesson.frontmatter.romanization).trim() !== ""
      ? stringValue(lesson.frontmatter.romanization)
      : lesson.realization.headword;
  return renderInlineMarkdown(
    truncateShortTitle(
      title.replaceAll("←", " from ").replaceAll("→", " to ").replace(/\s+/g, " ").trim(),
    ),
    options,
  );
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

  const output = [
    `% GENERATED FILE. Edit ${target.source}, then run npm run generate:books.`,
    "",
    `\\chapter*{${renderInlineMarkdown(target.title, renderOptions)}}`,
    `\\addcontentsline{toc}{chapter}{${renderInlineMarkdown(target.title, renderOptions)}}`,
    "\\markboth{Pronunciation}{Pronunciation}",
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
  ["culture-pragmatics", "usage and culture"],
]);

const INDEX_FACET_ORDER = [
  "pronunciation",
  "script",
  "writing",
  "grammar",
  "etymology",
  "usage and culture",
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
