// ---------------------------------------------------------------------------
// glyph-coverage.ts — will every character in the book actually render?
//
// WHY THIS MODULE EXISTS
//
// Twice now a tranche has been written, tested green, security-reviewed, pushed,
// and failed CI eleven minutes later on one line:
//
//     latin missing_character rose to 2 against a baseline of 0
//     bengali missing_character rose to 29 against a baseline of 0
//
// Both were a character absent from the book's font: `ǣ` in an Old English
// citation (HL-C214), `ɔ` in twenty-nine Bengali romanizations (HL-C223). Both
// were avoidable, and both were caught only by XeLaTeX.
//
// EVERY LOCAL GATE READS THE CORPUS. NONE OF THEM OPENS A FONT. That is the gap.
//
// THE MODEL, WHICH TOOK THREE WRONG PROBES TO GET RIGHT
//
// A book is not one font, and this is where the earlier attempts went wrong:
//
//  1. First probe checked the track's SCRIPT characters against the track's
//     SCRIPT font. Those were never at risk — they sit inside `\bn{...}`, which
//     selects a face chosen precisely because it covers them.
//  2. Second probe checked everything else against Latin Modern, using a
//     hardcoded list of wrapper commands. The list was wrong (Sanskrit's is
//     `\sk`, not `\sa`), so 14,563 characters looked missing on a corpus that
//     compiles clean.
//  3. Third probe read the wrappers from the preamble, and still reported four
//     characters missing on a clean corpus — because the preambles map them with
//     `\newunicodechar{ṉ}{\b{n}}`, rendering them as composed accents instead.
//
// So a character in main-font text is safe when it is EITHER in Latin Modern OR
// mapped by `\newunicodechar` in that book's preamble. Validated against the
// committed corpus: 125 distinct main-font characters, 121 in the font, 4
// mapped, 0 unaccounted for — which is exactly the `missing_character=0` every
// book reports.
//
// WHY THE FONT IS A COMMITTED LIST RATHER THAN A LIVE QUERY
//
// Latin Modern ships with TeX Live. It is not resolvable from a plain checkout,
// so a live cmap query would make this gate silently unmeasured wherever TeX
// Live is absent — which is most places, including the test runner. The answer
// is therefore recorded in `core/main-font-charset.json`, generated once by
// querying the real font. Script fonts ARE vendored under `_fonts/`, so those
// are checked live.
//
// See BACKLOG HL-C214, HL-C223.
// ---------------------------------------------------------------------------

/** One character that will not render. */
export interface GlyphGap {
  /** File the character appears in, repo-relative. */
  file: string;
  language: string;
  /** `U+XXXX`. */
  codepoint: string;
  char: string;
  occurrences: number;
  /** Which font would have had to carry it. */
  layer: "main" | "script";
  /** The script font's filename, for a script-layer gap. */
  font?: string;
}

export interface GlyphCoverageReport {
  gaps: GlyphGap[];
  summary: {
    filesScanned: number;
    mainCharacters: number;
    scriptCharacters: number;
    gaps: number;
  };
}

export interface BookFonts {
  language: string;
  /** Preamble source, read for wrapper commands and `\newunicodechar` mappings. */
  preamble: string;
  /** Generated chapter/appendix files: repo-relative path to contents. */
  files: { path: string; text: string }[];
  /** Vendored script fonts by filename, each as the set of codepoints it covers. */
  scriptFonts: Record<string, ReadonlySet<number>>;
}

// Only the DECLARATION HEAD is a regex. `\w+` and `\\newfontfamily` are both
// unambiguous, so this part is linear; the option block and the braced argument
// are found by scanning, below.
const FONT_FAMILY_HEAD = /\\newfontfamily\\(\w+)/g;
// Two declaration forms are in use and BOTH must be read. Reading only the first
// is what made Sanskrit's `\sk` invisible to an earlier probe.
const WRAPPER_BRACE = /\\newcommand\{\\(\w+)\}\[1\]\{\{\\(\w+)\s*#1\}\}/g;
const WRAPPER_TEXT = /\\newcommand\{\\(\w+)\}\[1\]\{\\text(\w+)\{#1\}\}/g;
const UNICODE_CHAR = /\\newunicodechar\{(.)\}/gu;

/**
 * Font family name to the file it names, found by scanning rather than by regex.
 *
 * The obvious pattern is `\\newfontfamily\\(\w+)\s*(?:\[[^\]]*\])?\{([^}]*)\}`,
 * and CodeQL flags it as polynomial ReDoS for a reason the earlier fix here did
 * not address: `[^\]]*` inside `\[...\]` rescans to end-of-input from EVERY
 * unterminated `[`, so N declaration heads with no closing bracket cost O(N^2).
 * Moving the whitespace into the optional group fixed a different ambiguity and
 * left this one standing.
 *
 * A scan has no backtracking to exploit: find the head, step over one optional
 * `[...]` block with `indexOf`, take the next `{...}`. Same result, linear, and
 * easier to read than the pattern it replaces.
 */
function fontFamilies(preamble: string): Map<string, string> {
  const families = new Map<string, string>();
  FONT_FAMILY_HEAD.lastIndex = 0;
  let head: RegExpExecArray | null;
  while ((head = FONT_FAMILY_HEAD.exec(preamble)) !== null) {
    let cursor = head.index + head[0].length;
    while (cursor < preamble.length && /\s/.test(preamble[cursor]!)) cursor += 1;
    if (preamble[cursor] === "[") {
      const close = preamble.indexOf("]", cursor);
      // An unterminated option block is a malformed preamble, not a puzzle to
      // solve: abandon this declaration and carry on from the head.
      if (close === -1) continue;
      cursor = close + 1;
      while (cursor < preamble.length && /\s/.test(preamble[cursor]!)) cursor += 1;
    }
    if (preamble[cursor] !== "{") continue;
    const close = preamble.indexOf("}", cursor);
    if (close === -1) continue;
    families.set(head[1]!, preamble.slice(cursor + 1, close).trim());
  }
  return families;
}

/** Wrapper command name to the font file it selects. */
export function scriptWrappers(preamble: string): Map<string, string> {
  const families = fontFamilies(preamble);
  const out = new Map<string, string>();
  for (const [, cmd, family] of preamble.matchAll(WRAPPER_BRACE)) {
    const file = families.get(family);
    if (file) out.set(cmd, file);
  }
  for (const [, cmd, stem] of preamble.matchAll(WRAPPER_TEXT)) {
    // fontspec's `\newfontfamily\xfont` also defines `\textx`, so the command
    // body names the family with the `font` suffix stripped.
    const file = families.get(`${stem}font`) ?? families.get(stem);
    if (file) out.set(cmd, file);
  }
  return out;
}

/** Characters the preamble re-renders as something else, so the font need not have them. */
export function mappedCharacters(preamble: string): Set<string> {
  return new Set([...preamble.matchAll(UNICODE_CHAR)].map((m) => m[1]!));
}

/**
 * Split a book file into main-font text and per-font script runs.
 *
 * Applied repeatedly until stable, because a wrapper can sit inside another
 * command's argument and one pass would leave the outer text holding script
 * characters it does not own.
 */
function partition(text: string, wrappers: ReadonlyMap<string, string>): {
  main: string;
  script: { font: string; text: string }[];
} {
  const script: { font: string; text: string }[] = [];
  if (wrappers.size === 0) return { main: text, script };
  const names = [...wrappers.keys()].sort((a, b) => b.length - a.length).join("|");
  // One level of nesting inside the argument covers `\bn{\textbf{x}}`, which is
  // common; a plain `[^{}]*` silently fails on it and floods the main bucket.
  const pattern = new RegExp(`\\\\(${names})\\{((?:[^{}]|\\{[^{}]*\\})*)\\}`, "g");
  let main = text;
  // Bounded rather than `for(;;)`. The loop does terminate -- each pass removes a
  // whole `\cmd{...}` -- but it is quadratic in nesting depth, and ~1MB of nested
  // wrappers takes minutes. Real lessons nest one or two deep; a hundred passes is
  // far past anything legitimate, and exhausting it leaves the remaining text in
  // the MAIN bucket, which over-reports rather than falsely passing.
  for (let pass = 0; pass < 100; pass += 1) {
    let matched = false;
    main = main.replace(pattern, (_all, cmd: string, body: string) => {
      matched = true;
      script.push({ font: wrappers.get(cmd)!, text: body });
      return "";
    });
    if (!matched) break;
  }
  return { main, script };
}

function tally(text: string): Map<string, number> {
  const counts = new Map<string, number>();
  for (const ch of text) {
    if (ch.codePointAt(0)! > 127) counts.set(ch, (counts.get(ch) ?? 0) + 1);
  }
  return counts;
}

/**
 * Check every book against the fonts it actually loads.
 *
 * `mainCharset` is the committed list of characters verified present in Latin
 * Modern. A main-font character passes if it is in that list OR mapped by
 * `\newunicodechar` in that book's own preamble — per-book, because a mapping in
 * Tamil's preamble does nothing for Bengali's.
 */
export function measureGlyphCoverage(
  books: readonly BookFonts[],
  mainCharset: ReadonlySet<string>,
): GlyphCoverageReport {
  const gaps: GlyphGap[] = [];
  let filesScanned = 0;
  const mainChars = new Set<string>();
  const scriptChars = new Set<string>();

  for (const book of books) {
    const wrappers = scriptWrappers(book.preamble);
    const mapped = mappedCharacters(book.preamble);
    for (const file of book.files) {
      filesScanned += 1;
      const { main, script } = partition(file.text, wrappers);

      for (const [ch, n] of tally(main)) {
        mainChars.add(ch);
        if (mainCharset.has(ch) || mapped.has(ch)) continue;
        gaps.push({
          file: file.path,
          language: book.language,
          codepoint: `U+${ch.codePointAt(0)!.toString(16).toUpperCase().padStart(4, "0")}`,
          char: ch,
          occurrences: n,
          layer: "main",
        });
      }

      for (const run of script) {
        // `hasOwn` as well as the falsy check: `run.font` comes from an
        // unrestricted `([^}]*)` capture, so a preamble naming a font
        // `constructor` would otherwise resolve through the prototype chain to a
        // truthy value and throw on `.has`.
        const cover = Object.hasOwn(book.scriptFonts, run.font) ? book.scriptFonts[run.font] : undefined;
        // A font we cannot resolve is UNMEASURED, never clean. Skipping quietly
        // is how a gate reports success for work it did not do.
        if (!cover) continue;
        for (const [ch, n] of tally(run.text)) {
          scriptChars.add(ch);
          if (cover.has(ch.codePointAt(0)!)) continue;
          gaps.push({
            file: file.path,
            language: book.language,
            codepoint: `U+${ch.codePointAt(0)!.toString(16).toUpperCase().padStart(4, "0")}`,
            char: ch,
            occurrences: n,
            layer: "script",
            font: run.font,
          });
        }
      }
    }
  }

  gaps.sort(
    (a, b) => a.language.localeCompare(b.language) || a.file.localeCompare(b.file) || a.codepoint.localeCompare(b.codepoint),
  );
  return {
    gaps,
    summary: {
      filesScanned,
      mainCharacters: mainChars.size,
      scriptCharacters: scriptChars.size,
      gaps: gaps.length,
    },
  };
}

/** Render for a terminal. */
export function renderGlyphCoverage(report: GlyphCoverageReport): string[] {
  const { filesScanned, mainCharacters, scriptCharacters, gaps } = report.summary;
  if (gaps === 0) {
    return [
      `glyph coverage: every character renders -- ${mainCharacters} main-font and ` +
        `${scriptCharacters} script characters across ${filesScanned} generated files`,
    ];
  }
  const lines = [`glyph coverage: ${gaps} character(s) will not render`];
  for (const gap of report.gaps.slice(0, 20)) {
    const where = gap.layer === "main" ? "main font" : `script font ${gap.font}`;
    lines.push(`  ${gap.file}: ${gap.codepoint} '${gap.char}' x${gap.occurrences} -- absent from the ${where}`);
  }
  if (report.gaps.length > 20) lines.push(`  ... and ${report.gaps.length - 20} more`);
  return lines;
}
