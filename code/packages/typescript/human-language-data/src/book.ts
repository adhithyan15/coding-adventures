import { canonicalChapterHash } from "./hash.js";
import { hasOwn } from "./constants.js";
import type { LessonBodyBlock, ChapterCapability } from "./types.js";
import type { ParsedLesson } from "./parse.js";

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
    const script = scripts.find((candidate) => candidate.matcher.test(character));
    if (script) {
      const run: string[] = [];
      while (cursor < markdown.length) {
        const nextCodePoint = markdown.codePointAt(cursor);
        const next = nextCodePoint === undefined ? "" : String.fromCodePoint(nextCodePoint);
        if (!script.matcher.test(next)) break;
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
): string {
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

function sectionShortTitle(lesson: ParsedLesson, options?: InlineRenderOptionsInput): string {
  const title = lesson.realization.type.startsWith("practice")
    ? "Practice"
    : options && stringValue(lesson.frontmatter.romanization).trim() !== ""
      ? stringValue(lesson.frontmatter.romanization)
      : lesson.realization.headword;
  return renderInlineMarkdown(
    title.replaceAll("←", " from ").replaceAll("→", " to ").replace(/\s+/g, " ").trim(),
    options,
  );
}

function targetRenderOptions(target: BookGenerationTarget): InlineRenderOptionsInput | undefined {
  if (target.inlineScripts !== undefined) {
    if (target.unicodeScript !== undefined || target.scriptCommand !== undefined) {
      throw new Error(
        `${target.language} chapter ${target.chapter}: inlineScripts cannot be combined with unicodeScript or scriptCommand`,
      );
    }
    if (target.inlineScripts.length === 0) {
      throw new Error(`${target.language} chapter ${target.chapter}: inlineScripts must not be empty`);
    }
    return target.inlineScripts;
  }
  if (target.unicodeScript === undefined && target.scriptCommand === undefined) return undefined;
  if (target.unicodeScript === undefined || target.scriptCommand === undefined) {
    throw new Error(
      `${target.language} chapter ${target.chapter}: unicodeScript and scriptCommand must be declared together`,
    );
  }
  return { unicodeScript: target.unicodeScript, scriptCommand: target.scriptCommand };
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
  const renderOptions = targetRenderOptions(target);
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
