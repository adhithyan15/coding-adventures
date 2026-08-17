// ---------------------------------------------------------------------------
// literal-markup.ts — authoring markup that reaches the reader as text.
//
// WHY THIS MODULE EXISTS
//
// Eight lessons across four tracks wrote `&nbsp;` to space out a display line.
// `&nbsp;` is HTML. These lessons are Markdown, rendered to LaTeX, and the LaTeX
// escaper does exactly its job on the ampersand:
//
//     \gu{હા} \&nbsp;\&nbsp; \gu{ના}
//
// A reader of the PDF sees the characters `&nbsp;` on the page. Three of the four
// tracks were already merged, so it was live in three published books.
//
// EVERY EXISTING GATE MISSED IT, and the reason is the interesting part. It is
// not a schema error, not an untaught glyph, not a LaTeX injection, not a font
// gap, not a ramp violation. It is *correctly escaped text that should never have
// been text*. `check:books` even confirms the byte-identical `\&nbsp;` on every
// run, because the generator is faithfully reproducing the mistake.
//
// The whole suite checks that output is SAFE and REPRODUCIBLE. Nothing checked
// that it was MEANINGFUL.
//
// WHY IT IS A GATE RATHER THAN A REPORT
//
// The defect recurred ONE PR after being fixed — two lessons in the very next
// tranche had it again, from a template that still carried it. A mistake that
// survives its own fix by one iteration is not going to be fixed by remembering
// harder. The corpus is clean today (0 hits at both layers), so this can block
// from the first commit rather than start as inherited debt.
//
// BOTH LAYERS ARE CHECKED, and they catch different things:
//
//   - the LESSON SOURCE is where an author can act. `&nbsp;` in a `.md` names
//     the file to edit.
//   - the GENERATED .tex is what the reader actually gets. It catches markup
//     that arrives from a template or a generator change rather than from a
//     lesson, which the source scan cannot see.
//
// See BACKLOG HL-C217.
// ---------------------------------------------------------------------------
import type { ParsedLesson } from "./parse.js";

/**
 * HTML entities and bare HTML tags in Markdown meant for a LaTeX renderer.
 *
 * `&#\d+;` and `&#x…;` are included because a numeric entity is the same mistake
 * wearing a different hat, and an author who reaches for `&#160;` is even less
 * likely to check the rendered page than one who writes `&nbsp;`.
 *
 * The tag list is deliberately short and closed. Matching `<[a-z]+>` generally
 * would flag `<PAUSE 2s>`-style stage directions, arrows in prose, and every
 * `a < b` in a grammar explanation — a gate that cries wolf gets suppressed, and
 * then it catches nothing.
 */
// `<\s*\/?\s*` looks harmless and is quadratic: with the `\/?` matching empty,
// N whitespace characters can be split between the two `\s*` in O(N^2) ways, all
// of them tried before the tag list fails. Measured 2.6ms at N=2000 and 2,480ms
// at N=64,000, a clean 4x per doubling — and reachable from ordinary Markdown,
// because `blankExemptSpans` rewrites a long inline code span INTO spaces.
// Grouping the slash with its own `\s*` removes the ambiguity: 0.12ms at N=64,000.
const SOURCE_MARKUP =
  /&(?:nbsp|amp|lt|gt|quot|apos|#\d+|#x[0-9A-Fa-f]+);|<\s*(?:\/\s*)?(?:br|p|div|span|hr|table|tr|td|th|ul|ol|li|strong|em)\b[^>]*>/gi;

/**
 * The same mistake after the LaTeX escaper has been over it.
 *
 * The escaper turns `&` into `\&`, which is why this is a separate pattern
 * rather than the same one: by the time it reaches the book the text no longer
 * looks like HTML at all, and grepping the `.tex` for `&nbsp;` finds nothing.
 * That is precisely how it stayed invisible.
 */
// `--!>` closes an HTML comment as surely as `-->` does; CodeQL's js/bad-tag-filter
// flags a filter that knows only one of them, and it is right — an authoring
// comment ending `--!>` would have reached the reader unseen.
const RENDERED_MARKUP = /\\&(?:nbsp|amp|lt|gt|quot|apos|\#\d+|\#x[0-9A-Fa-f]+);|<!--|--!?>/g;

export interface LiteralMarkupFinding {
  /** Lesson id for a source finding, or the file path for a rendered one. */
  where: string;
  language: string;
  /** 1-indexed line within the lesson body or the file. */
  line: number;
  /** The offending text, verbatim. */
  markup: string;
  layer: "source" | "rendered";
}

export interface LiteralMarkupReport {
  findings: LiteralMarkupFinding[];
  summary: {
    lessonsScanned: number;
    sourceFindings: number;
    renderedFindings: number;
  };
}

/**
 * Blank out the spans where this markup is legitimate, preserving line numbers.
 *
 * Three exemptions, and each one is load-bearing:
 *
 *  - **HTML comments.** `<!-- hl-knowledge: … -->` and `<!-- hl-activity: … -->`
 *    ARE the directive syntax. Flagging them would flag every lesson in the
 *    corpus.
 *  - **Fenced code blocks** and **inline code spans.** A lesson explaining an
 *    entity, or showing a snippet, is quoting rather than emitting — and this
 *    module's own backlog entry does exactly that.
 *
 * Replacing with spaces rather than deleting keeps every offset and newline, so
 * a reported line number still points at the right line.
 */
/**
 * Remove or blank every HTML comment, by a linear scan rather than a regex.
 *
 * `/<!--[\s\S]*?-->/g` looks obvious and has three defects, all of which CodeQL
 * flagged and all of which are real:
 *
 *  - **It rescans to EOF from every unterminated `<!--`.** Quadratic: measured
 *    0.48ms at 2KB of repeated `<!--` and 76ms at 32KB, a clean 4x per doubling.
 *  - **It only knows `-->`.** HTML also ends a comment with `--!>`, so a comment
 *    closed that way would survive the strip and reach the reader.
 *  - **One pass can leave a fragment behind**, which is the incomplete-multi-
 *    character-sanitization class: strip a well-formed comment out of a malformed
 *    one and the leftovers can re-form.
 *
 * A scan fixes all three at once and is easier to read than the regex was.
 * Consuming the WHOLE span from `<!--` to its terminator means nothing can
 * re-form, and an unterminated comment runs to end-of-text, which is what a
 * browser does with one.
 */
export function stripHtmlComments(text: string, mode: "remove" | "blank"): string {
  let out = "";
  let cursor = 0;
  for (;;) {
    const start = text.indexOf("<!--", cursor);
    if (start === -1) {
      out += text.slice(cursor);
      return out;
    }
    out += text.slice(cursor, start);
    const plain = text.indexOf("-->", start + 4);
    const bang = text.indexOf("--!>", start + 4);
    let end: number;
    if (plain === -1 && bang === -1) end = text.length;
    else if (bang === -1) end = plain + 3;
    else if (plain === -1) end = bang + 4;
    else end = plain < bang ? plain + 3 : bang + 4;
    const span = text.slice(start, end);
    // Blanking rather than deleting preserves offsets and newlines, so a
    // reported line number still points at the right line.
    out += mode === "blank" ? span.replace(/[^\n]/g, " ") : "";
    cursor = end;
  }
}

function blankExemptSpans(text: string): string {
  const blank = (match: string) => match.replace(/[^\n]/g, " ");
  return stripHtmlComments(text, "blank")
    .replace(/```[\s\S]*?```/g, blank)
    .replace(/`[^`\n]*`/g, blank);
}

function scan(text: string, pattern: RegExp, exempt: boolean): { line: number; markup: string }[] {
  const out: { line: number; markup: string }[] = [];
  // The exemptions are MARKDOWN semantics and must only be applied to Markdown.
  // In LaTeX none of the three is a comment: `%` is. Applying them to generated
  // output made the gate blind in exactly the place it exists to watch — an
  // authoring comment that reaches the reader is invisible to a scanner that
  // treats `<!-- -->` as a comment, and `\&nbsp;` smuggled inside one scored
  // zero findings while the same string outside scored one. Backticks are the
  // same mistake: in LaTeX a backtick is an open quote, and blanking on it hid
  // 7,852 characters of the real corpus.
  const lines = (exempt ? blankExemptSpans(text) : text).split("\n");
  lines.forEach((line, index) => {
    // A fresh lastIndex per line: these are /g regexes and a shared one would
    // skip every other match on a line holding two.
    pattern.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = pattern.exec(line)) !== null) {
      out.push({ line: index + 1, markup: match[0] });
      if (match.index === pattern.lastIndex) pattern.lastIndex += 1;
    }
  });
  return out;
}

/** Scan lesson sources. */
export function measureLiteralMarkup(
  lessons: readonly ParsedLesson[],
  renderedFiles: readonly { path: string; language: string; text: string }[] = [],
): LiteralMarkupReport {
  const findings: LiteralMarkupFinding[] = [];

  for (const lesson of lessons) {
    for (const hit of scan(lesson.body, SOURCE_MARKUP, true)) {
      findings.push({
        where: lesson.realization.lessonId,
        language: lesson.language,
        line: hit.line,
        markup: hit.markup,
        layer: "source",
      });
    }
  }

  for (const file of renderedFiles) {
    for (const hit of scan(file.text, RENDERED_MARKUP, false)) {
      findings.push({
        where: file.path,
        language: file.language,
        line: hit.line,
        markup: hit.markup,
        layer: "rendered",
      });
    }
  }

  findings.sort(
    (a, b) => a.language.localeCompare(b.language) || a.where.localeCompare(b.where) || a.line - b.line,
  );

  return {
    findings,
    summary: {
      lessonsScanned: lessons.length,
      sourceFindings: findings.filter((f) => f.layer === "source").length,
      renderedFindings: findings.filter((f) => f.layer === "rendered").length,
    },
  };
}

/** Render for a terminal. */
export function renderLiteralMarkup(report: LiteralMarkupReport): string[] {
  const { sourceFindings, renderedFindings, lessonsScanned } = report.summary;
  if (sourceFindings + renderedFindings === 0) {
    return [`literal markup: none in ${lessonsScanned} lessons or any generated book`];
  }
  const lines = [
    `literal markup: ${sourceFindings} in lesson sources, ${renderedFindings} in generated books ` +
      `-- authoring markup that reaches the reader as text`,
  ];
  for (const finding of report.findings.slice(0, 25)) {
    // `[^>]*` admits ESC, CR and BEL, and this string goes straight to a
    // terminal. A finding must not be able to repaint the report that reports it.
    const safe = finding.markup.replace(/[\u0000-\u001f\u007f]/g, "?");
    lines.push(`  ${finding.layer.padEnd(8)} ${finding.where}:${finding.line}  ${safe}`);
  }
  if (report.findings.length > 25) lines.push(`  ... and ${report.findings.length - 25} more`);
  return lines;
}
