/**
 * porter.ts — the Porter stemmer (Martin Porter, 1980).
 *
 * =============================================================================
 * WHAT IS A STEMMER?
 * =============================================================================
 *
 * A stemmer collapses morphological variants of a word to a
 * shared root form so that searches for one variant match
 * documents containing another:
 *
 *     run  / running / runs / ran    →  "run"  (or "ran" for irregulars)
 *     happy / happiness / happily    →  "happi"
 *     index / indexes / indexed      →  "index"
 *
 * The Porter stemmer is the textbook English stemmer — published
 * in 1980, ~30 lines of rules, and still the default choice for
 * most search engines that don't need linguistic precision.  It
 * works by suffix-stripping in five sequential steps, with each
 * suffix rule gated by a "measure" check that prevents
 * over-stripping of short words.
 *
 * =============================================================================
 * THE MEASURE m(W)
 * =============================================================================
 *
 * Porter's "measure" of a word W is the number of consonant-vowel
 * transitions in W, written m(W).  Formally W is split into a
 * sequence of consonant-runs (C) and vowel-runs (V), then
 * m = number of VC pairs:
 *
 *     [C](VC)^m[V]
 *
 * Examples (consonants = c, vowels = v, y treated specially):
 *
 *     "tr"           → C    →  m = 0
 *     "ee"           → V    →  m = 0
 *     "tree"         → C V   →  m = 0
 *     "trouble"      → C V C   →  m = 1
 *     "troubles"     → C V C V C   →  m = 2
 *     "troublesome"  → C V C V C V C   →  m = 3
 *
 * Suffix-stripping rules have the form `(condition) S1 → S2`
 * where the condition is `m > N` (and sometimes `*v*`, `*o`,
 * etc.).  The condition is checked on the word AFTER stripping
 * S1 — so "agreement" with rule `(m > 1) EMENT →` would strip
 * to "agree" only if "agre" has m > 1 (which it doesn't, so no
 * strip).
 *
 * =============================================================================
 * WHY THIS IS WORTH 200+ LINES
 * =============================================================================
 *
 * Stemmer quality directly affects search recall.  A bad stemmer
 * either misses relevant documents (under-stripping) or returns
 * noise (over-stripping).  Porter is the well-known sweet spot:
 * decades of empirical tuning, MIT-licensed reference
 * implementations in every language, and used by everything from
 * Lucene to old-school Unix `nroff` tools.
 *
 * We port a clean, well-commented reference (no clever tricks)
 * rather than rolling our own — both because the algorithm has
 * a million corner cases and because deviation from the
 * published reference produces non-portable stems.
 *
 * @module porter
 */

// ─────────────────────────────────────────────────────────────────────
// Vowel classification (Porter convention: 'y' is sometimes a vowel)
// ─────────────────────────────────────────────────────────────────────

const VOWELS: ReadonlySet<string> = new Set(["a", "e", "i", "o", "u"]);

/**
 * True iff position `i` in `w` is a consonant under Porter's
 * convention.  'y' is a consonant if the previous char is a
 * vowel, vowel otherwise.  Position 0 'y' is treated as a
 * consonant.
 *
 * Implementation note: written iteratively rather than
 * recursively.  A naive recursive version (`isConsonantAt(w, i-1)`
 * for the 'y' lookback) overflows V8's stack on inputs like
 * `"y".repeat(10000)` — a single very-long y-run in untrusted
 * input would crash the indexer (security review caught this
 * as a MEDIUM finding before push).
 *
 * The iterative form walks backwards through any 'y'-run to
 * find the first non-'y' character (or the word start) and
 * computes the final classification from the parity of the
 * run length plus the "base" character's class.
 *
 * Behaviour-preserving derivation:
 *   - If the run starts at position 0 (j < 0): position 0 'y'
 *     is a consonant by spec.  Each subsequent 'y' alternates.
 *     So result = (runLen is odd).
 *   - If `w[j]` is a vowel: first 'y' flips to consonant.  Each
 *     subsequent 'y' alternates.  result = (runLen is odd).
 *   - If `w[j]` is a non-'y' consonant: first 'y' flips to vowel.
 *     Each subsequent 'y' alternates.  result = (runLen is even).
 *
 * In all three cases combined:
 *     result = baseIsConsonant === (runLen % 2 === 0)
 * where `baseIsConsonant = false` at word start (treat-as-vowel)
 * or `!VOWELS.has(w[j])` otherwise.
 */
function isConsonantAt(w: string, i: number): boolean {
  const ch = w[i]!;
  if (VOWELS.has(ch)) return false;
  if (ch !== "y") return true;
  // Walk back through any 'y'-run.  `j` ends up pointing at
  // the first non-'y' char before the run, or -1 if the run
  // started at position 0.
  let j = i - 1;
  while (j >= 0 && w[j] === "y") j--;
  // "Base" classification — what the run is anchored against.
  // Word start (j < 0) is treated as if a vowel preceded the
  // run (which makes the first 'y' a consonant, matching
  // Porter's spec for position 0 'y').
  const baseIsConsonant = j < 0 ? false : !VOWELS.has(w[j]!);
  const runLen = i - j; // 1..N
  // Behaviour-preserving identity (see comment above).
  return baseIsConsonant === (runLen % 2 === 0);
}

/**
 * Porter's measure m(W) — number of VC pairs after stripping
 * the optional leading C and trailing V.
 */
function measure(w: string): number {
  // Build a 0/1 mask: 0 = vowel, 1 = consonant.
  // Then count VC transitions: positions where mask[i] = 0
  // followed by mask[i+1] = 1.
  if (w.length === 0) return 0;
  // First, find the start of the first vowel-run (skip leading
  // consonants).
  let i = 0;
  while (i < w.length && isConsonantAt(w, i)) i++;
  if (i >= w.length) return 0; // all consonants, no VC pair
  // From here on, count VC transitions.
  let m = 0;
  let inVowelRun = true;
  for (; i < w.length; i++) {
    if (isConsonantAt(w, i)) {
      if (inVowelRun) {
        m++;
        inVowelRun = false;
      }
    } else {
      inVowelRun = true;
    }
  }
  return m;
}

/** True iff `w` contains a vowel anywhere. */
function containsVowel(w: string): boolean {
  for (let i = 0; i < w.length; i++) {
    if (!isConsonantAt(w, i)) return true;
  }
  return false;
}

/**
 * True iff `w` ends in a double consonant (e.g. "tt", "ll").
 */
function endsDoubleConsonant(w: string): boolean {
  if (w.length < 2) return false;
  if (w[w.length - 1] !== w[w.length - 2]) return false;
  return isConsonantAt(w, w.length - 1);
}

/**
 * True iff `w` ends in CVC (consonant-vowel-consonant) where
 * the final C is not w, x, or y.  Used by step 1b's `(m=1 and *o)`
 * condition.
 */
function endsCVC(w: string): boolean {
  if (w.length < 3) return false;
  const n = w.length;
  if (!isConsonantAt(w, n - 1)) return false;
  if (isConsonantAt(w, n - 2)) return false;
  if (!isConsonantAt(w, n - 3)) return false;
  const last = w[n - 1]!;
  if (last === "w" || last === "x" || last === "y") return false;
  return true;
}

// ─────────────────────────────────────────────────────────────────────
// Suffix matching utilities
// ─────────────────────────────────────────────────────────────────────

/**
 * Apply the first matching rule from a table.  Each rule is
 * `[suf, repl, predicate]`.  Returns the new word after the
 * first hit, or `w` unchanged if no rule fires.
 */
function applyFirstMatchingRule(
  w: string,
  rules: readonly [string, string, (stem: string) => boolean][],
): string {
  for (const [suf, repl, pred] of rules) {
    if (w.endsWith(suf)) {
      const stem = w.slice(0, w.length - suf.length);
      if (pred(stem)) {
        return stem + repl;
      }
      // Suffix matched but predicate failed — Porter's
      // convention is to STOP looking (only one rule per step).
      return w;
    }
  }
  return w;
}

// ─────────────────────────────────────────────────────────────────────
// Steps 1a, 1b, 1b', 1c — plurals + past participles + -y → -i
// ─────────────────────────────────────────────────────────────────────

function step1a(w: string): string {
  // Order matters — Porter spec: try in order, first match wins.
  if (w.endsWith("sses")) return w.slice(0, -4) + "ss";
  if (w.endsWith("ies")) return w.slice(0, -3) + "i";
  if (w.endsWith("ss")) return w;
  if (w.endsWith("s")) return w.slice(0, -1);
  return w;
}

function step1b(w: string): string {
  // (m > 0) EED → EE
  if (w.endsWith("eed")) {
    const stem = w.slice(0, -3);
    if (measure(stem) > 0) return stem + "ee";
    return w;
  }
  // (*v*) ED → ε
  // (*v*) ING → ε
  let stem: string | null = null;
  let stripped = w;
  if (w.endsWith("ed")) {
    const s = w.slice(0, -2);
    if (containsVowel(s)) {
      stem = s;
      stripped = s;
    }
  } else if (w.endsWith("ing")) {
    const s = w.slice(0, -3);
    if (containsVowel(s)) {
      stem = s;
      stripped = s;
    }
  }
  if (stem === null) return w;
  // After ED / ING stripped: apply step 1b' adjustments.
  if (stripped.endsWith("at")) return stripped + "e";
  if (stripped.endsWith("bl")) return stripped + "e";
  if (stripped.endsWith("iz")) return stripped + "e";
  // (*d and not (*L or *S or *Z)) → strip the doubled consonant
  if (endsDoubleConsonant(stripped)) {
    const last = stripped[stripped.length - 1]!;
    if (last !== "l" && last !== "s" && last !== "z") {
      return stripped.slice(0, -1);
    }
    return stripped;
  }
  // (m=1 and *o) → add E
  if (measure(stripped) === 1 && endsCVC(stripped)) return stripped + "e";
  return stripped;
}

function step1c(w: string): string {
  // (*v*) Y → I
  if (!w.endsWith("y")) return w;
  const stem = w.slice(0, -1);
  if (!containsVowel(stem)) return w;
  return stem + "i";
}

// ─────────────────────────────────────────────────────────────────────
// Step 2 — common -ational, -tional, etc. suffixes
// ─────────────────────────────────────────────────────────────────────

const STEP_2_RULES: readonly [string, string, (s: string) => boolean][] = [
  ["ational", "ate", (s) => measure(s) > 0],
  ["tional",  "tion", (s) => measure(s) > 0],
  ["enci",    "ence", (s) => measure(s) > 0],
  ["anci",    "ance", (s) => measure(s) > 0],
  ["izer",    "ize",  (s) => measure(s) > 0],
  ["abli",    "able", (s) => measure(s) > 0],
  ["alli",    "al",   (s) => measure(s) > 0],
  ["entli",   "ent",  (s) => measure(s) > 0],
  ["eli",     "e",    (s) => measure(s) > 0],
  ["ousli",   "ous",  (s) => measure(s) > 0],
  ["ization", "ize",  (s) => measure(s) > 0],
  ["ation",   "ate",  (s) => measure(s) > 0],
  ["ator",    "ate",  (s) => measure(s) > 0],
  ["alism",   "al",   (s) => measure(s) > 0],
  ["iveness", "ive",  (s) => measure(s) > 0],
  ["fulness", "ful",  (s) => measure(s) > 0],
  ["ousness", "ous",  (s) => measure(s) > 0],
  ["aliti",   "al",   (s) => measure(s) > 0],
  ["iviti",   "ive",  (s) => measure(s) > 0],
  ["biliti",  "ble",  (s) => measure(s) > 0],
];

function step2(w: string): string {
  return applyFirstMatchingRule(w, STEP_2_RULES);
}

// ─────────────────────────────────────────────────────────────────────
// Step 3 — -icate, -ative, -alize, etc.
// ─────────────────────────────────────────────────────────────────────

const STEP_3_RULES: readonly [string, string, (s: string) => boolean][] = [
  ["icate", "ic", (s) => measure(s) > 0],
  ["ative", "",   (s) => measure(s) > 0],
  ["alize", "al", (s) => measure(s) > 0],
  ["iciti", "ic", (s) => measure(s) > 0],
  ["ical",  "ic", (s) => measure(s) > 0],
  ["ful",   "",   (s) => measure(s) > 0],
  ["ness",  "",   (s) => measure(s) > 0],
];

function step3(w: string): string {
  return applyFirstMatchingRule(w, STEP_3_RULES);
}

// ─────────────────────────────────────────────────────────────────────
// Step 4 — -al, -ance, -ence, -er, -ic, -able, -ible, ...
// ─────────────────────────────────────────────────────────────────────

const STEP_4_SUFFIXES: readonly string[] = [
  "al", "ance", "ence", "er", "ic", "able", "ible", "ant", "ement", "ment",
  "ent", "ou", "ism", "ate", "iti", "ous", "ive", "ize",
];

function step4(w: string): string {
  // Step 4 strips with predicate m > 1.  Special case: -ion is
  // stripped only if preceded by 's' or 't'.
  for (const suf of STEP_4_SUFFIXES) {
    if (w.endsWith(suf)) {
      const stem = w.slice(0, w.length - suf.length);
      if (measure(stem) > 1) return stem;
      return w;
    }
  }
  // -ion (after 's' or 't')
  if (w.endsWith("ion")) {
    const stem = w.slice(0, -3);
    if (stem.length === 0) return w;
    const last = stem[stem.length - 1]!;
    if ((last === "s" || last === "t") && measure(stem) > 1) return stem;
  }
  return w;
}

// ─────────────────────────────────────────────────────────────────────
// Step 5a/5b — final 'e' and double-'l' cleanup
// ─────────────────────────────────────────────────────────────────────

function step5a(w: string): string {
  // (m > 1) E →
  // (m = 1 and not *o) E →
  if (!w.endsWith("e")) return w;
  const stem = w.slice(0, -1);
  const m = measure(stem);
  if (m > 1) return stem;
  if (m === 1 && !endsCVC(stem)) return stem;
  return w;
}

function step5b(w: string): string {
  // (m > 1 and *d and *L) → strip one L
  if (w.length < 2) return w;
  if (w[w.length - 1] !== "l") return w;
  if (!endsDoubleConsonant(w)) return w;
  if (measure(w.slice(0, -1)) <= 1) return w;
  return w.slice(0, -1);
}

// ─────────────────────────────────────────────────────────────────────
// Public entry
// ─────────────────────────────────────────────────────────────────────

/**
 * Reduce a single word to its Porter stem.
 *
 * @param word - The input word.  Should already be lowercased
 *               (the stemmer doesn't lowercase — it's the
 *               caller's job since the normaliser handles it).
 *               Non-ASCII letters pass through unchanged
 *               (Porter is an English-only algorithm).
 * @returns The stem.  Words ≤ 2 chars are returned unchanged
 *          (too short for any rule to fire usefully).
 */
export function porterStem(word: string): string {
  if (word.length <= 2) return word;
  let w = word;
  w = step1a(w);
  w = step1b(w);
  w = step1c(w);
  w = step2(w);
  w = step3(w);
  w = step4(w);
  w = step5a(w);
  w = step5b(w);
  return w;
}
