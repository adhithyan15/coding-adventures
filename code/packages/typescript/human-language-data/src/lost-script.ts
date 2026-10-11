// ---------------------------------------------------------------------------
// lost-script.ts — non-Latin script that an encoder turned into "?".
//
// WHAT HAPPENED
//
// Twenty-six Japanese lessons were authored through a tool that wrote text in
// a legacy single-byte encoding. Every character that encoding could not hold
// came out as one ASCII question mark. English survived; kana and kanji did
// not, so a review cue that should have read
//
//     Say **いいえ** and tap all three morae before writing the new sign.
//
// reached the corpus — and the published book, and the narration — as
//
//     Say ??? and tap all three morae before writing the new sign.
//
// One `?` per lost CHARACTER, not per byte: いいえ is three characters and left
// three marks. That one-to-one count is what made the damage recoverable (see
// the Japanese track changelog), and it is also what gives the damage a SHAPE a
// scanner can see.
//
// WHY EVERY EXISTING GATE MISSED IT
//
// The result is valid UTF-8, valid Markdown and valid LaTeX. It escapes
// cleanly, every glyph is in every font, and `check:books` happily confirmed
// the byte-identical `Say ??? and tap` on every run, because the generator was
// faithfully reproducing what it was given. As with `literal-markup.ts`, the
// suite checked that output was SAFE and REPRODUCIBLE; nothing checked that it
// still said anything. It sat in a published chapter for seven weeks until a
// reviewer read it by eye.
//
// THE THREE SHAPES
//
// A question mark is ordinary punctuation, so the gate cannot ban it. It looks
// for the three places a `?` stands where a WORD belongs:
//
//   rule    example of the damage                  why ordinary prose never does it
//   ------  -------------------------------------  ---------------------------------
//   run     `Say ??? and`, `**??**`                two or more marks that do not
//                                                  follow a word: punctuation closes
//                                                  a word, it does not start one
//   lone    `write ? once`, `Write ?, then`,       a mark with space before it and
//           `before tracing ?.`                    a sentence continuing after it is
//                                                  a word, not the end of a question
//   glued   `?hard to exist?`, `espa?ol`           a mark with a letter straight
//                                                  after it is inside a word, or is
//                                                  a lost quotation mark
//
// Doubled punctuation after a word — the review lessons' `Why?? (**quārē**.)`,
// where a gloss that already ends in `?` meets the prompt's own `?` — is NOT
// lost script: the word is right there. It is a different, cosmetic slip, and
// the run rule deliberately lets it through rather than conflate the two.
//
// THE ALLOWANCES, each forced by a real line of the corpus:
//
//   - French typography puts a space BEFORE `?` ("Combien ? has had no possible
//     answer"). That is exactly the lone shape, and nothing about the mark
//     itself distinguishes it, so the lone rule is off for French. The run and
//     glued rules still apply there: a spaced `?` is never doubled or glued.
//   - A `?` named beside the inverted `¿` ("the ¿ ? marks") cannot be lost
//     script, because the encoder that ate the kana would have eaten the `¿`
//     first. A surviving non-ASCII neighbour proves the span came through.
//   - A `?` named beside its sibling marks ("# ? and ! — the marks that go only
//     at the end") is talking ABOUT punctuation, which four tracks teach.
//   - Code spans, fenced code and URLs quote rather than emit, and a URL's query
//     string is full of glued `?`. They are blanked before scanning.
//
// BOTH LAYERS, as in `literal-markup.ts`: the LESSON SOURCE names the file an
// author edits; the GENERATED .tex is what the reader actually gets, and catches
// damage that arrives through a chapter intro, a template or a generator rather
// than a lesson. Narration is generated from the same sources and normalises
// the space before punctuation away ("Say???"), which would blur the run rule
// into ordinary doubled punctuation, so it is left to the source layer.
//
// EVERY PATTERN IS LINEAR. No quantifier nests inside another, each lookaround
// is a fixed-width class or a single `\s+` that stops at the first non-space,
// and the blanking passes are index scans, not lazy `[\s\S]*?` regexes. The
// tests drive each one with 50,000-character adversarial input.
// ---------------------------------------------------------------------------
import { stripControlCharacters } from "./constants.js";
import type { ParsedLesson } from "./parse.js";

/** Which of the three damage shapes a finding matched. */
export type LostScriptRule = "run" | "glued" | "lone";

export interface LostScriptFinding {
  /** Lesson id for a source finding, or the file path for a rendered one. */
  where: string;
  language: string;
  /** 1-indexed line within the lesson body or the file. */
  line: number;
  rule: LostScriptRule;
  /** A short window of the line around the damage, control characters removed. */
  excerpt: string;
  layer: "source" | "rendered";
}

export interface LostScriptReport {
  findings: LostScriptFinding[];
  summary: {
    lessonsScanned: number;
    filesScanned: number;
    sourceFindings: number;
    renderedFindings: number;
  };
}

/**
 * Tracks whose typography spaces the question mark off its word.
 *
 * French writes `Comment ça va ?` with a (thin, often ordinary) space before
 * the mark. That is the lone shape exactly, so the lone rule cannot run there.
 * Add a track here only for the same reason — a typographic convention, cited —
 * never to quiet a finding.
 */
export const SPACED_QUESTION_MARK_LANGUAGES: ReadonlySet<string> = new Set(["french"]);

/**
 * RUN — two or more marks that do not follow a word.
 *
 * The lookbehind refuses a letter, combining mark or digit (so `Why??` passes:
 * a word is present) and refuses another `?` (so a run is only ever matched
 * from its first mark, which keeps a long run one match, not N).
 */
const RUN = /(?<![\p{L}\p{M}\p{N}?])\?{2,}/gu;

/**
 * GLUED — a mark with a letter (or combining mark) immediately after it.
 *
 * Ordinary prose always follows a question mark with space, a quote, a bracket
 * or more punctuation. A letter means the mark is inside a word (`espa?ol`) or
 * replaced a quotation mark (`?hard to exist?`). URLs are blanked first, since
 * `page.php?cd=1` is the one place a glued mark is correct.
 */
const GLUED = /\?(?=[\p{L}\p{M}])/gu;

/**
 * LONE — a mark that stands where a word belongs.
 *
 * Space (or the start of the text) before it, and after it either more sentence
 * in lower case (`write ? once`) or clause punctuation (`Write ?, then`,
 * `before tracing ?.`). A real question in English ends on its word, with no
 * space before the mark, so this shape does not occur in English prose.
 *
 *   (?<!\S)                  nothing but whitespace (or the start) before it
 *   (?<!¿[ \t])              not the second half of a named `¿ ?` pair
 *   \?
 *   (?=\s+\p{Ll}|[,;:.])     the sentence carries on after it
 *   (?!\s+(?:and|or)\s+[!.,¡¿])   not named beside its sibling marks
 *
 * `\s+` crosses a newline on purpose: lessons wrap at 80 columns, and
 * `write ?\nonce` is the same damage as `write ? once`. Each `\s+` only ever
 * runs forward over one whitespace run from one `?`, so the total work is
 * linear in the input.
 */
const LONE = /(?<!\S)(?<!¿[ \t])\?(?=\s+\p{Ll}|[,;:.])(?!\s+(?:and|or)\s+[!.,¡¿])/gu;

/** A URL, which carries `?` legitimately and is not prose. */
const URL = /https?:\/\/\S+/g;

/** Replace every character but newline with a space, keeping offsets and lines. */
function blank(span: string): string {
  return span.replace(/[^\n]/g, " ");
}

/**
 * Blank fenced code blocks by an index scan.
 *
 * `/```[\s\S]*?```/g` rescans to the end of the text from every unterminated
 * fence, which is quadratic in the number of fences; `literal-markup.ts` met the
 * same trap with HTML comments. Walking `indexOf` from fence to fence visits each
 * character once. An unterminated fence runs to the end of the text, which is
 * how Markdown renders one.
 */
function blankFences(text: string): string {
  let out = "";
  let cursor = 0;
  for (;;) {
    const open = text.indexOf("```", cursor);
    if (open === -1) return out + text.slice(cursor);
    const close = text.indexOf("```", open + 3);
    const end = close === -1 ? text.length : close + 3;
    out += text.slice(cursor, open) + blank(text.slice(open, end));
    cursor = end;
  }
}

/**
 * Blank what quotes rather than emits.
 *
 * Markdown exemptions (code) apply to lesson sources only. In LaTeX a backtick
 * is an opening quotation mark, and `literal-markup.ts` found that blanking on
 * it hid thousands of characters of the real book; URLs are blanked on both
 * layers because both print them.
 */
function blankExempt(text: string, markdown: boolean): string {
  let out = text.replace(URL, blank);
  if (markdown) out = blankFences(out).replace(/`[^`\n]*`/g, blank);
  return out;
}

/** Offsets at which each line starts, for turning a match index into a line. */
function lineStarts(text: string): number[] {
  const starts = [0];
  for (let i = text.indexOf("\n"); i !== -1; i = text.indexOf("\n", i + 1)) starts.push(i + 1);
  return starts;
}

/** 1-indexed line containing `index`, by binary search over `starts`. */
function lineOf(starts: readonly number[], index: number): number {
  let lo = 0;
  let hi = starts.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (starts[mid]! <= index) lo = mid;
    else hi = mid - 1;
  }
  return lo + 1;
}

/** Scan one text, returning each hit's line, rule and a short excerpt. */
function scan(
  original: string,
  language: string,
  markdown: boolean,
): { line: number; rule: LostScriptRule; excerpt: string }[] {
  const text = blankExempt(original, markdown);
  const starts = lineStarts(text);
  const rules: [LostScriptRule, RegExp][] = [
    ["run", RUN],
    ["glued", GLUED],
  ];
  if (!SPACED_QUESTION_MARK_LANGUAGES.has(language)) rules.push(["lone", LONE]);

  const hits: { line: number; rule: LostScriptRule; excerpt: string; index: number }[] = [];
  for (const [rule, pattern] of rules) {
    // These are shared /g regexes: reset before each text, or a scan would
    // start where the previous text's last match left off.
    pattern.lastIndex = 0;
    for (const match of text.matchAll(pattern)) {
      const index = match.index;
      // The excerpt is cut from the ORIGINAL text, so the reader sees what is
      // actually on the page, and is bounded so a 50 KB line stays one line.
      const from = Math.max(0, index - 24);
      const excerpt = stripControlCharacters(original.slice(from, index + 24).replace(/\s+/g, " ")).trim();
      hits.push({ line: lineOf(starts, index), rule, excerpt, index });
    }
  }
  hits.sort((a, b) => a.index - b.index);
  return hits.map(({ line, rule, excerpt }) => ({ line, rule, excerpt }));
}

/**
 * Scan lesson bodies and, optionally, generated book files.
 *
 * `renderedFiles[].language` is the track (the first path segment of a book
 * file), which is what decides whether the French allowance applies.
 */
export function measureLostScript(
  lessons: readonly ParsedLesson[],
  renderedFiles: readonly { path: string; language: string; text: string }[] = [],
): LostScriptReport {
  const findings: LostScriptFinding[] = [];

  for (const lesson of lessons) {
    // The body, comments included: an `hl-activity` prompt is read by the
    // learner, so a lost word inside one is as real as a lost word in prose.
    for (const hit of scan(lesson.body, lesson.language, true)) {
      findings.push({ where: lesson.realization.lessonId, language: lesson.language, layer: "source", ...hit });
    }
  }

  for (const file of renderedFiles) {
    for (const hit of scan(file.text, file.language, false)) {
      findings.push({ where: file.path, language: file.language, layer: "rendered", ...hit });
    }
  }

  findings.sort(
    (a, b) => a.language.localeCompare(b.language) || a.where.localeCompare(b.where) || a.line - b.line,
  );

  return {
    findings,
    summary: {
      lessonsScanned: lessons.length,
      filesScanned: renderedFiles.length,
      sourceFindings: findings.filter((f) => f.layer === "source").length,
      renderedFindings: findings.filter((f) => f.layer === "rendered").length,
    },
  };
}

/** Render for a terminal: a clean run is a positive statement, not silence. */
export function renderLostScript(report: LostScriptReport): string[] {
  const { sourceFindings, renderedFindings, lessonsScanned, filesScanned } = report.summary;
  if (sourceFindings + renderedFindings === 0) {
    return [`lost script: no "?" standing in for a word in ${lessonsScanned} lessons or ${filesScanned} book files`];
  }
  const lines = [
    `lost script: ${sourceFindings} in lesson sources, ${renderedFindings} in generated books ` +
      `-- "?" where a word belongs, the mark an encoder leaves for a character it cannot hold`,
  ];
  for (const finding of report.findings.slice(0, 25)) {
    lines.push(
      `  ${finding.layer.padEnd(8)} ${finding.rule.padEnd(5)} ${finding.where}:${finding.line}  ${finding.excerpt}`,
    );
  }
  if (report.findings.length > 25) lines.push(`  ... and ${report.findings.length - 25} more`);
  return lines;
}
