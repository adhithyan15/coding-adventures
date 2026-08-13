// ---------------------------------------------------------------------------
// letter-ledger.ts — the order a reader meets the letters, and whether it holds
// ---------------------------------------------------------------------------
//
// HL11 section 4 says letters are ordered by the WORDS THEY MAKE WRITABLE, not
// by the traditional recitation order. `data/scripts/<script>-ledger.json` is
// where that decision is recorded, one entry per position.
//
// The ledger is AUTHORED INTENT, in the same sense `chapters.json` is: a human
// proposes it (with `data/scripts/propose_letter_ledger.py`), reviews it, and
// commits it. Nothing here rewrites it. What this module does is ask whether it
// still says something true about the corpus, because a ledger and a curriculum
// can drift apart silently — the ledger claims a letter unlocks a word, the
// lesson teaching that word gets renamed, and the claim quietly becomes fiction.
//
// Six questions, each of which has a wrong answer that is invisible by eye:
//
//   1. Are the positions 1..N, in order, with no gaps or repeats?
//   2. Does every glyph actually belong to the script the ledger names?
//   3. Does a vowel sign ever arrive before any base letter?  In an abugida a
//      mark MODIFIES a letter; there is no mark without a base, so a ledger
//      that opens on one describes a lesson that cannot be written down.
//   4. Do the letters of a family sit together?  Splitting a family across the
//      ledger trades a reading ramp for a writing confusion.
//   5. Does every word a letter claims to unlock come from a real lesson?
//   6. Does any letter unlock nothing for a long stretch?  That is the Root
//      Ledger's rule (HL10) applied to glyphs: a letter taught early that pays
//      off nowhere is a step the reader climbed for free.
//
// Everything is report-only, per the HL05 and HL08 precedent. The debt predates
// the measurement, and a gate that fails on recorded debt teaches authors to
// route around it.

import type { ParsedLesson } from "./parse.js";

/** One position in a script's ledger. */
export interface LedgerLetter {
  position: number;
  glyph: string;
  /**
   * The glyph's code point as `U+XXXX`.
   *
   * Beside the name, this pins the row NUMERICALLY. A rendered glyph is not an
   * audit surface: it can be a lookalike from another script, and it can carry
   * invisible passengers that render as nothing at all. `U+0BB1` next to
   * `TAMIL LETTER RRA` is checkable by a reviewer who cannot read Tamil.
   */
  codePoint: string;
  /** The glyph's official Unicode name, so a non-reader can audit the row. */
  unicodeName: string;
  kind: "letter" | "vowel-sign";
  /** The whole family this glyph belongs to, if any, as one string. */
  family: string | null;
  /** Where that family claim comes from. Present whenever `family` is. */
  familySource: string | null;
  unlocks: LedgerUnlock[];
}

/** A word a letter completes, and the lesson that teaches it. */
export interface LedgerUnlock {
  word: string;
  romanization: string;
  lesson: string;
}

/** One script's ledger. */
export interface LetterLedger {
  script: string;
  version: number;
  note: string;
  /** The tracks that read this ledger. Hindi and Sanskrit share Devanagari. */
  tracks: string[];
  openingLessons: number;
  openingWords: number;
  letters: LedgerLetter[];
}

/** A problem with a ledger. Severity mirrors `validate()`'s vocabulary. */
export interface LedgerIssue {
  script: string;
  severity: "error" | "warning" | "info";
  code: string;
  message: string;
  position?: number;
}

/**
 * Unicode script names, as the ledger spells them, mapped to the property
 * escape that tests membership.
 *
 * Written out rather than derived because the ledger's `script` field is a
 * lower-cased file name and the regex needs the canonical property value; the
 * two agree today and a typo in either should be an error, not a silent pass.
 */
const SCRIPT_PATTERN: Record<string, RegExp> = {
  tamil: /\p{Script=Tamil}/u,
  telugu: /\p{Script=Telugu}/u,
  kannada: /\p{Script=Kannada}/u,
  malayalam: /\p{Script=Malayalam}/u,
  devanagari: /\p{Script=Devanagari}/u,
};

/** A combining mark: a vowel sign, or the vowel-killer. */
const COMBINING = /^\p{Mn}|^\p{Mc}/u;

/**
 * Check one ledger against the corpus that is supposed to justify it.
 *
 * `lessons` is the whole parsed corpus; only the ledger's own tracks are
 * consulted, so a Devanagari ledger is judged against Hindi and Sanskrit
 * together, which is how it was proposed.
 */
export function validateLetterLedger(
  ledger: LetterLedger,
  lessons: ParsedLesson[],
  options: { unspentWindow?: number } = {},
): LedgerIssue[] {
  const issues: LedgerIssue[] = [];
  const unspentWindow = options.unspentWindow ?? 6;
  const script = ledger.script;
  const add = (
    severity: LedgerIssue["severity"],
    code: string,
    message: string,
    position?: number,
  ) => issues.push({ script, severity, code, message, position });

  // 1. Positions are 1..N in order.
  ledger.letters.forEach((letter, index) => {
    if (letter.position !== index + 1) {
      add("error", "ledger-position-out-of-order",
        `position ${letter.position} sits at index ${index + 1}`, letter.position);
    }
  });

  const seen = new Set<string>();
  const names = new Set<string>();
  for (const letter of ledger.letters) {
    if (seen.has(letter.glyph)) {
      add("error", "ledger-duplicate-glyph",
        `${letter.unicodeName} appears more than once`, letter.position);
    }
    seen.add(letter.glyph);

    // Two rows sharing a name is the copy-paste failure. The code point below
    // pins each row to a character; this pins each NAME to one row, so a row
    // duplicated and half-edited cannot leave two positions claiming to be the
    // same letter while holding different glyphs.
    if (names.has(letter.unicodeName)) {
      add("error", "ledger-duplicate-name",
        `'${letter.unicodeName}' names more than one position`, letter.position);
    }
    names.add(letter.unicodeName);
  }

  // 2. Every glyph belongs to the script the ledger names.
  //
  // `Object.hasOwn` rather than a bare lookup: `SCRIPT_PATTERN` is an object
  // literal, so `SCRIPT_PATTERN["constructor"]` returns a function, sails past
  // the guard below, and turns an intended REPORT into a TypeError that takes
  // the whole validation run down with it.
  const pattern = Object.hasOwn(SCRIPT_PATTERN, script) ? SCRIPT_PATTERN[script] : undefined;
  if (!pattern) {
    add("error", "ledger-unknown-script", `no Unicode script pattern for '${script}'`);
  } else {
    for (const letter of ledger.letters) {
      // ONE code point, checked before anything else looks at the string.
      //
      // Without this, the two tests below are each satisfied by a different
      // part of a longer string: `pattern.test` is unanchored, so it passes if
      // ANY code point is in-script, and `COMBINING` is anchored, so it only
      // ever sees the first. A row could therefore declare itself
      // `TAMIL LETTER KA` while carrying trailing Latin text, a bidi override,
      // or a homoglyph -- invisible in review, and the ledger is exactly the
      // artifact review is supposed to be able to trust.
      const points = [...letter.glyph];
      if (points.length !== 1) {
        add("error", "ledger-glyph-not-one-code-point",
          `${letter.unicodeName} is ${points.length} code points, not one`,
          letter.position);
        continue;
      }

      // The name is a claim ABOUT the glyph; the code point IS the glyph. There
      // is no Unicode name database in the browser or in Node's standard
      // library, so the row carries its own code point and that is what gets
      // checked. The name's script prefix is checked too, which is the part of
      // the name a wrong row is most likely to get wrong.
      const actual = `U+${(letter.glyph.codePointAt(0) ?? 0).toString(16).toUpperCase().padStart(4, "0")}`;
      if (letter.codePoint !== actual) {
        add("error", "ledger-code-point-mismatch",
          `row says ${letter.codePoint} but the glyph is ${actual}`, letter.position);
      }
      if (!letter.unicodeName.startsWith(script.toUpperCase() + " ")) {
        add("error", "ledger-name-wrong-script",
          `'${letter.unicodeName}' does not name a ${script} character`,
          letter.position);
      }

      if (!pattern.test(letter.glyph)) {
        add("error", "ledger-foreign-glyph",
          `${letter.unicodeName} is not a ${script} character`, letter.position);
      }
      const isMark = COMBINING.test(letter.glyph);
      const declaredMark = letter.kind === "vowel-sign";
      if (isMark !== declaredMark) {
        add("error", "ledger-kind-mismatch",
          `${letter.unicodeName} is declared '${letter.kind}' but Unicode says ` +
          `${isMark ? "it is a combining mark" : "it is a base character"}`,
          letter.position);
      }
    }
  }

  // 3. No vowel sign before the first base letter.
  const firstBase = ledger.letters.findIndex((l) => l.kind === "letter");
  const firstMark = ledger.letters.findIndex((l) => l.kind === "vowel-sign");
  if (firstMark >= 0 && (firstBase < 0 || firstMark < firstBase)) {
    const mark = ledger.letters[firstMark];
    add("error", "ledger-mark-before-letter",
      `${mark?.unicodeName} is taught before any base letter, and a vowel sign ` +
      `has nothing to attach to until one exists`, mark?.position);
  }

  // 4. Families sit together.
  const familyPositions = new Map<string, number[]>();
  for (const letter of ledger.letters) {
    if (!letter.family) continue;
    if (!letter.familySource) {
      add("warning", "ledger-family-unsourced",
        `${letter.unicodeName} claims a family with no stated source`, letter.position);
    }
    const list = familyPositions.get(letter.family) ?? [];
    list.push(letter.position);
    familyPositions.set(letter.family, list);
  }
  for (const [family, positions] of familyPositions) {
    const sorted = [...positions].sort((a, b) => a - b);
    const contiguous = sorted.every((p, i) => i === 0 || p === (sorted[i - 1] ?? 0) + 1);
    if (!contiguous) {
      add("warning", "ledger-family-split",
        `the family '${family}' is split across positions ${sorted.join(", ")}; ` +
        `letters that share a shape are learned together or confused apart`);
    }
  }

  // 5. Every claimed unlock names a lesson that exists, in one of this ledger's
  //    tracks. This is the check that catches drift: a ledger keeps asserting a
  //    payoff long after the lesson delivering it was renamed or removed.
  const tracks = new Set(ledger.tracks);
  const known = new Set(
    lessons.filter((l) => tracks.has(l.language)).map((l) => l.frontmatter.id),
  );
  if (known.size === 0) {
    // "Not checked" and "checked, clean" must not look the same. A ledger
    // supplies its own `tracks`, so one renamed or mistyped track name would
    // otherwise make the ONLY check for fictional unlock claims vanish while
    // the report still read zero -- the same silent-zero failure
    // `loadChapterPolicy` carries a warning about.
    if (ledger.letters.some((l) => l.unlocks.length > 0)) {
      add("warning", "ledger-unlocks-unverified",
        `no lesson of ${ledger.tracks.join("/")} was loaded, so the unlock ` +
        `claims in this ledger were not checked against anything`);
    }
  } else {
    for (const letter of ledger.letters) {
      for (const unlock of letter.unlocks) {
        if (!known.has(unlock.lesson)) {
          add("error", "ledger-unlock-missing-lesson",
            `${letter.unicodeName} claims to unlock '${unlock.word}' from lesson ` +
            `'${unlock.lesson}', which no ${ledger.tracks.join("/")} lesson declares`,
            letter.position);
        }
      }
    }
  }

  // 6. Unspent letters. Only judged where the whole window fits inside the
  //    ledger -- a letter near the end has not been given its chance yet, and
  //    reporting it would be an artifact of where the list stops.
  ledger.letters.forEach((letter, index) => {
    if (letter.unlocks.length > 0) return;
    if (index + unspentWindow >= ledger.letters.length) return;
    const window = ledger.letters.slice(index + 1, index + 1 + unspentWindow);
    if (window.every((l) => l.unlocks.length === 0)) {
      add("info", "ledger-unspent-letter",
        `${letter.unicodeName} unlocks nothing within ${unspentWindow} positions; ` +
        `cut it or move it to where its payoff lives`, letter.position);
    }
  });

  return issues;
}

/** A ledger's headline numbers, for the gap report. */
export interface LedgerSummary {
  script: string;
  tracks: string[];
  positions: number;
  openingWords: number;
  /** Opening words fully writable after N positions, for a few values of N. */
  writableAfter: { position: number; words: number }[];
  /** The earliest position at which any real word becomes writable. */
  firstWritablePosition: number | null;
  firstWritableWord: string | null;
  issues: number;
}

/**
 * Roll a ledger up into the numbers HL11 section 7 asks for.
 *
 * `firstWritableWord` is deliberately a WORD and not a letter count. Twenty
 * taught letters is not an achievement a reader can feel; writing *thank you*
 * is. The measurement is of the payoff, not the effort — the same reason HL05
 * measures a chapter by what the reader can do at the end of it.
 */
export function summarizeLetterLedger(
  ledger: LetterLedger,
  issues: LedgerIssue[] = [],
): LedgerSummary {
  let running = 0;
  const cumulative = ledger.letters.map((letter) => {
    running += letter.unlocks.length;
    return { position: letter.position, words: running };
  });

  const first = ledger.letters.find((l) => l.unlocks.length > 0);
  const at = (position: number) =>
    cumulative.filter((c) => c.position <= position).pop()?.words ?? 0;

  return {
    script: ledger.script,
    tracks: ledger.tracks,
    positions: ledger.letters.length,
    openingWords: ledger.openingWords,
    writableAfter: [8, 16, 24].map((position) => ({ position, words: at(position) })),
    firstWritablePosition: first?.position ?? null,
    firstWritableWord: first?.unlocks[0]?.word ?? null,
    issues: issues.length,
  };
}
