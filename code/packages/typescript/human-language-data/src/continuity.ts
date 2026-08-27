// Does the course have a memory of itself? — HL09 step 1.
//
// The ramp budgets (HL08, HL-C18C) measure how big each STEP is. They cannot see
// whether the steps hold together, and reading Spanish chapters 1-8 showed that
// is where the course actually fails:
//
//   "A gentle ramp is not made of small steps; it's made of steps you can still
//    stand on."
//
// Three things go wrong, and each is invisible to a per-lesson budget.
//
// 1. ORDER. 56 of Spanish's 146 lessons carry no `sequence`, so their reading
//    order exists only inside hand-typed LaTeX. French is worse: 64 of 73. A ramp
//    whose order is unknown cannot be verified at all — every other measurement in
//    this file is conditional on knowing what comes first.
//
// 2. REINFORCEMENT. 93 of Spanish's 182 taught atoms (51%) are never practised
//    again; the median atom is revisited ZERO times. HL00 specified the schedule
//    (N+1, N+3, N+7, N+15), defined a `review` lesson type to carry it, and named
//    session-map.md as the artifact that verifies it. The corpus has zero `review`
//    lessons and a session map covering 3 chapters of 33. The schedule was
//    specified and never built.
//
// 3. FORWARD REFERENCES. Starved of anything to review, later chapters reach
//    sideways for whatever they need: chapter 7 rewards the learner with
//    "Como pan y bebo agua" — and `pan` and `agua` are taught in chapter 26.
//
// The third is a SYMPTOM of the second, which is why they are measured together.
// Fix the scheduler and the forward references lose their cause.
//
// WHAT THIS UNDERCOUNTS, stated because it changes how the number reads. A word the
// course never teaches ANYWHERE is invisible: chapter 7's untaught "¿Algo más?" and
// its `un`/`una` never appear, because nothing in the data marks them as target
// language. Hyphenated headwords are split on the hyphen (right for a range like
// "once — quince", wrong for "dix-sept"), so a hyphenated compound never becomes a
// matcher. And the plain-prose floor counts UTF-16 units, so two-character CJK words
// are reachable only through emphasis — part of why Chinese and Japanese report zero.
// Every one of these makes the published figure a FLOOR, never an overstatement.
//
// Report-only, per the HL05 precedent: the debt predates the measurement.

import type { ParsedLesson } from "./parse.js";
import { readingOrder, frontmatterList } from "./ramp.js";
import { CONTENT_TYPES } from "./constants.js";

/**
 * The spaced-retrieval windows from HL09 §7.
 *
 * Expanding, because that is how retrieval practice works: consolidate before
 * decay, then retrieve at growing distances. An atom must reappear in some later
 * lesson's `practises.knowledge` inside each window.
 */
export const REINFORCEMENT_WINDOWS = [
  { name: "R1", from: 1, to: 3, purpose: "consolidate before it decays" },
  { name: "R2", from: 5, to: 15, purpose: "first real retrieval" },
  { name: "R3", from: 20, to: 60, purpose: "durable" },
  { name: "R4", from: 80, to: 250, purpose: "recognition at distance" },
] as const;

export type WindowName = (typeof REINFORCEMENT_WINDOWS)[number]["name"];

/** A lesson whose declared order is missing or collides with another's. */
export interface OrderDefect {
  lessonId: string;
  language: string;
  chapter: number | null;
  kind: "no-sequence" | "duplicate-sequence" | "forward-prerequisite" | "forward-review";
  /** For a duplicate, the other lesson; for a forward prerequisite, the one needed. */
  other?: string;
  detail: string;
}

/** One atom that misses a reinforcement window it was long enough to have. */
export interface ReinforcementDefect {
  atom: string;
  language: string;
  introducedBy: string;
  /** Zero-based position in the track's reading order. */
  introducedAt: number;
  /** Windows the track was long enough to contain, and the atom missed. */
  missed: WindowName[];
  /** Total later lessons practising this atom, at any distance. */
  revisits: number;
}

/** A lesson using target-language material that only a LATER lesson teaches. */
export interface ForwardReference {
  lessonId: string;
  language: string;
  /** Position of the lesson doing the borrowing. */
  position: number;
  /** The word it used. */
  word: string;
  /** The lesson that actually teaches that word. */
  taughtBy: string;
  /** How many lessons later the learner will meet it properly. */
  lessonsEarly: number;
}

export interface TrackContinuity {
  language: string;
  lessonCount: number;
  lessonsWithoutSequence: number;
  forwardPrerequisites: number;
  /** Lessons whose `reviews_of` names a lesson the learner has not reached. */
  forwardReviews: number;
  /** Atoms introduced in this track that any window measurement could see. */
  atomsTaught: number;
  /** Atoms never practised again at any distance. */
  atomsNeverRevisited: number;
  forwardReferences: number;
}

export interface ContinuityReport {
  windows: typeof REINFORCEMENT_WINDOWS;
  order: OrderDefect[];
  reinforcement: ReinforcementDefect[];
  forwardReferences: ForwardReference[];
  tracks: TrackContinuity[];
  summary: {
    /** Lessons with no `sequence`. Until this is zero, the rest is provisional. */
    lessonsWithoutSequence: number;
    /** Tracks where at least one lesson has no declared order. */
    tracksWithUnorderedLessons: number;
    forwardPrerequisites: number;
    /** You cannot review a lesson that has not happened yet. */
    forwardReviews: number;
    atomsTaught: number;
    /** Atoms never practised again at ANY distance — the headline number. */
    atomsNeverRevisited: number;
    /** Share of taught atoms that are never revisited. */
    neverRevisitedPercent: number;
    /** Per-window miss counts, over atoms whose track was long enough to have it. */
    missedByWindow: Record<WindowName, number>;
    forwardReferences: number;
  };
}

/**
 * Atoms a lesson PRACTISES — the reinforcement signal.
 *
 * Deliberately NOT `reviews_of`, which 144 of Spanish's 146 lessons set and which
 * cannot close a window: it names LESSON ids while atoms live in another
 * namespace, so it has never reinforced anything. Measuring it would report a
 * corpus that reinforces beautifully and teaches nothing twice.
 */
function practisedAtoms(lesson: ParsedLesson): string[] {
  const atoms = new Set(frontmatterList(lesson, "practises.knowledge"));
  for (const block of lesson.blocks ?? []) {
    for (const atom of block.knowledge?.assesses ?? []) atoms.add(atom);
  }
  return [...atoms];
}

function declaredSequence(lesson: ParsedLesson): number | null {
  const raw = lesson.frontmatter.sequence;
  const value = typeof raw === "number" ? raw : Number(raw);
  return typeof raw !== "undefined" && String(raw).trim() !== "" && Number.isFinite(value)
    ? value
    : null;
}

/**
 * Definite articles that may precede a headword, by track.
 *
 * Only languages whose headwords actually carry an article appear here; a track
 * absent from this map never has a leading word stripped. Every entry was taken from
 * a census of the corpus's own headwords, so the map describes what is written rather
 * than what the language could in principle write.
 */
const DEFINITE_ARTICLES: Partial<Record<string, Set<string>>> = {
  spanish: new Set(["el", "la", "los", "las"]),
  italian: new Set(["il", "lo", "la", "i", "gli", "le"]),
  french: new Set(["le", "la", "les"]),
  portuguese: new Set(["o", "a", "os", "as"]),
  german: new Set(["der", "die", "das"]),
};

/**
 * Headwords, normalised for matching, that a lesson teaches.
 *
 * A multi-word headword ("buenos días") is kept whole: matching its parts
 * separately would report `días` as a forward reference from any lesson that
 * mentions the phrase.
 */
function taughtWords(lesson: ParsedLesson): string[] {
  const headword = lesson.realization.headword ?? "";
  // A headword may hold alternatives — "hola / buenas", "sí, no" — or a RANGE,
  // written with a dash: "dieciséis — diecinueve", "once — quince".
  const parts = headword
    .split(/[/,]|\s+y\s+|\s*[—–-]\s*/)
    .map((part) => part.trim().toLowerCase())
    .filter((part) => part.length > 0);

  const out = new Set(parts);
  for (const part of parts) {
    // Headwords carry their article: "el pan", "la casa". A body saying "bebo agua"
    // is using the same word, so the bare noun has to match too.
    //
    // This used to strip ANY leading word of three characters or fewer. A census of the
    // corpus's own headwords shows that rule firing on 227 of 1,453 lessons (247
    // headword parts), of which only 49 lessons (64 parts) actually begin with an
    // article. It was registering `llamo` as taught by "me llamo", `favor` by "por
    // favor", `dia` by "bom dia", and the night- and afternoon-word of every
    // ശുഭ / शुभ / శుభ / ಶುಭ greeting in Malayalam, Hindi, Telugu and Kannada — because
    // all those openers are three characters.
    //
    // The failure that surfaced it: "así que" is a legitimate headword, `así` is three
    // characters, so `que` got registered as first taught there — reporting ten earlier
    // lessons that use `que` as forward references to it. The lesson had to be renamed
    // to work around the measurement.
    //
    // So the rule is an ALLOWLIST of actual definite articles, per language, grounded
    // in that census rather than in a length guess. Spanish `lo` is deliberately absent
    // — in "lo siento" it is a pronoun, and stripping it would register `siento`.
    // Italian `a` likewise: "a domani" is a preposition, not an article.
    const articles = DEFINITE_ARTICLES[lesson.language];
    if (!articles) continue;
    const match = /^(\S+)\s+(.+)$/.exec(part);
    if (match && articles.has(match[1]!)) out.add(match[2]!.trim());
  }
  return [...out].filter((word) => word.length > 0);
}

/**
 * Every whitespace-separated token of a lesson's own headword, however it is written.
 *
 * Distinct from `taughtWords`, which decides what a lesson TEACHES to the rest of the
 * corpus and so is deliberately conservative. This decides only what a lesson may not
 * be accused of borrowing from someone else, and there conservatism is the wrong
 * direction: any token of "mع السلامة" is that lesson's own material.
 */
function ownHeadwordTokens(lesson: ParsedLesson): string[] {
  const headword = (lesson.realization.headword ?? "").toLowerCase();
  return headword
    .split(/[/,\u060C\u061B]|\s+y\s+|\s*[\u2014\u2013-]\s*|\s+/)
    .map((token) => token.trim())
    .filter((token) => token.length > 0);
}


/**
 * English words that are also common target-language headwords.
 *
 * Without this, every English sentence in a Spanish lesson reports `a`, `no`,
 * `son` and `me` as forward references. The list is small and deliberately so:
 * the length rule below does most of the work, and a denylist that grows without
 * bound is a sign the matching rule is wrong.
 *
 * HOW TO EXTEND IT, IF YOU MUST
 *
 * By census, never by guesswork. Run the detector, take the reports that matched
 * in PLAIN prose only (an emphasised match is the corpus marking target language,
 * so it is not English), and read each one in place. In the sweep that added the
 * last three, 18 reports qualified and only 3 were actually English -- so a list
 * built from a plausible-looking wordlist would have suppressed 15 real defects.
 *
 * The tempting alternative -- applying this guard to the plain path only, and
 * trusting emphasis to mean "target language" -- was tried and is wrong. Authors
 * emphasise English for stress too ("**no** glide, no drift"), which reported
 * `no` from seven lessons. Both paths keep the guard.
 */
const ENGLISH_COLLISIONS = new Set([
  "a", "an", "as", "at", "be", "but", "can", "do", "eat", "el", "en", "es", "fin",
  "for", "go", "he", "in", "is", "it", "la", "led", "me", "no", "of", "on", "once",
  "or", "past", "pie", "roman", "san", "sea", "sin", "so", "son", "some", "tag",
  "tan", "the", "three", "to", "us", "van", "was", "we", "y", "you",
  // Added by census, not by guesswork (HL-C103). The three below were the ONLY
  // English homographs among the 18 plain-prose-only reports in a full corpus
  // sweep -- the other 15 were genuine target-language forward references and
  // must keep reporting:
  //   comes   "*comer* comes from Latin *comedere*" -> the Spanish tu-form
  //   hand    "in the old German (Fraktur) hand"    -> German Hand
  //   regular "Regular stress: TAR-de"              -> Spanish regular, "so-so"
  "comes", "hand", "regular",
]);

/**
 * Is this headword usable as a forward-reference matcher at all?
 *
 * Two classes are excluded on principle rather than by list, because a denylist
 * that has to name them is a sign the rule is wrong:
 *
 * - **Single characters.** A writing lesson teaching the Cyrillic letter `е` or a
 *   Devanagari mātrā `ा` would otherwise match in every lesson of that script.
 *   That was the worst false positive in the first census — five scripts' worth.
 * - **Pattern notation.** `e→ie` and `o→ue` are how the corpus writes a stem
 *   change. They are descriptions of a rule, not words a learner meets.
 */
function usableAsMatcher(word: string): boolean {
  return [...word].length >= 2 && !/[→?()[\]]/u.test(word);
}

/**
 * The character class that may not sit against a matched word.
 *
 * ONE regex for the whole corpus, and that is the entire point. The forward-reference
 * matcher below used to build `(?<![\p{L}\p{M}-])<word>(?![\p{L}\p{M}-])` per taught
 * word, and a Unicode-property class is not a cheap thing to build: measured on this
 * corpus, `new RegExp` of that shape costs ~162µs and its first `.test()` another
 * ~168µs, because `\p{L}` and `\p{M}` expand into large code-point tables. At ~2,700
 * taught words that is ~0.9s of pure regex setup, which was the single largest line
 * in the gap report's profile — larger than reading all 2,771 lessons off disk.
 *
 * The lookarounds never varied; only the literal between them did. So the literal is
 * found with `indexOf` — which is a native substring search costing a fraction of a
 * microsecond — and the two lookarounds are re-expressed as this one shared class,
 * compiled once and applied to the single code point on each side of the hit.
 *
 * Four regexes need this class, and all four are built from this ONE string rather
 * than written out four times. That is not tidiness. The candidate index and the run
 * skip are each only sound while their class agrees with the one the boundary test
 * uses: widen the boundary class alone and the index starts hiding real forward
 * references; widen the run class alone and the skip jumps over positions that could
 * still match. Written out separately, a future edit that adds `\p{N}` to one of them
 * would do exactly that — silently, and with a green suite, because no test can see a
 * class that is merely inconsistent with another. Sharing the source makes that
 * divergence unrepresentable rather than merely unlikely.
 */
const WORD_ADJACENT_CLASS = "[\\p{L}\\p{M}-]";

/** One code point: is it word-adjacent? */
const WORD_ADJACENT = new RegExp(WORD_ADJACENT_CLASS, "u");

/** Maximal runs of that same class — the only places a match can begin or end. */
const WORD_RUN = new RegExp(`${WORD_ADJACENT_CLASS}+`, "gu");

/** A word's own leading run, anchored. */
const LEADING_RUN = new RegExp(`^${WORD_ADJACENT_CLASS}+`, "u");

/**
 * Add every candidate that any run of `text` could possibly reach.
 *
 * `String.match` with a global pattern, not `matchAll`: this runs over every lesson
 * body in the corpus, and `matchAll` allocates a match OBJECT — with `index`,
 * `input` and `groups` — for each of the ~1.4 million runs, where `match` returns
 * the plain strings. That difference alone was worth ~90ms of the report.
 */
function addReachableCandidates(
  text: string,
  byLeadingRun: Map<string, number[]>,
  reachable: Set<number>,
): void {
  // `match` resets this shared regex's `lastIndex` on entry and on exit, so the
  // module-level instance is safe to reuse and costs nothing to recompile.
  const runs = text.match(WORD_RUN);
  if (!runs) return;
  for (const run of runs) {
    const bucket = byLeadingRun.get(run);
    if (bucket) for (const position of bucket) reachable.add(position);
  }
}

/**
 * The run a word must land on to occur at all — its leading word-adjacent run.
 *
 * `""` when the word opens on something outside the class (`¿qué`, a digit), which
 * the caller must then treat as un-indexable rather than as matching nothing.
 */
function leadingRun(word: string): string {
  const match = LEADING_RUN.exec(word);
  return match ? match[0] : "";
}

/** Is this code point one that may not sit against a match? `""` never is. */
function isWordAdjacent(codePoint: string): boolean {
  return codePoint !== "" && WORD_ADJACENT.test(codePoint);
}

/**
 * The run of word-adjacent characters starting at `at`, consumed in one step.
 *
 * Sticky rather than global, and `lastIndex` is set on every call rather than
 * carried between them, so this shared instance holds no state across calls.
 */
const WORD_RUN_AT = new RegExp(`${WORD_ADJACENT_CLASS}*`, "uy");

/**
 * The first position after the word-adjacent run beginning at `at` — the next place
 * a match could possibly start.
 *
 * This is what keeps `occursAsWholeWord` linear in the haystack rather than
 * quadratic. When an occurrence is rejected, retrying one character later is
 * pointless work: every position inside a run is preceded by a word-adjacent
 * character, so every one of them fails the same way, and a long needle pays a full
 * comparison at each. A body of a million `a`s against a 4,000-character headword
 * took ~3.9 SECONDS that way and takes two `indexOf` calls now. That input is not
 * reachable today — the corpus is repo-controlled, and its longest matcher is 94
 * characters — but a fix for a superlinear walk should not leave a superlinear walk
 * behind it.
 *
 * When `haystack[at]` is not word-adjacent the run is empty and this is just
 * `at + 1`, which is the naive step; nothing is ever skipped that could have matched.
 */
function afterRunAt(haystack: string, at: number): number {
  WORD_RUN_AT.lastIndex = at;
  const run = WORD_RUN_AT.exec(haystack);
  return at + (run ? run[0].length : 0) + 1;
}

/** Would slicing `text` at `at` cut a surrogate pair in half? */
function splitsSurrogatePair(text: string, at: number): boolean {
  if (at <= 0 || at >= text.length) return false;
  const high = text.charCodeAt(at - 1);
  const low = text.charCodeAt(at);
  return high >= 0xd800 && high <= 0xdbff && low >= 0xdc00 && low <= 0xdfff;
}

/** The code point ending at `end`, or `""` at the start of the string. */
function codePointBefore(text: string, end: number): string {
  if (end <= 0) return "";
  // A lookbehind under the `u` flag steps back by a CODE POINT, not a code unit, so
  // a surrogate pair has to be taken whole. Devanagari and Arabic stay in the BMP,
  // but the corpus already carries Japanese, and emoji are one authoring session away.
  return splitsSurrogatePair(text, end - 1) ? text.slice(end - 2, end) : text[end - 1]!;
}

/**
 * Does `needle` occur in `haystack` with neither a letter, a combining mark, nor a
 * hyphen against either end?
 *
 * Exactly the predicate `(?<![\p{L}\p{M}-])<escaped needle>(?![\p{L}\p{M}-])` with the
 * `u` flag decides, and deliberately so — this replaced that regex for cost, not for
 * behaviour. Every occurrence is examined, not just the first, because the first may
 * be glued to a letter while a later one is free.
 */
interface WholeWordSearchProbe {
  candidateChecks: number;
  skippedRuns: number;
}

function occursAsWholeWord(
  haystack: string,
  needle: string,
  probe?: WholeWordSearchProbe,
): boolean {
  if (needle.length === 0) return false;
  for (let from = 0; from <= haystack.length; ) {
    const at = haystack.indexOf(needle, from);
    if (at < 0) return false;
    if (probe) probe.candidateChecks += 1;
    // A `u`-flag match runs from one code-point boundary to another, so an
    // occurrence that begins or ends halfway through a surrogate pair is one the
    // regex would refuse — `indexOf` finds the high half of `𐀀` when asked for a
    // lone `\uD800`, and the regex does not. Well-formed text never asks (Node's
    // UTF-8 reader cannot produce a lone surrogate), so this costs two integer
    // comparisons to keep the two definitions provably the same rather than the
    // same in practice.
    const end = at + needle.length;
    const splits = splitsSurrogatePair(haystack, at) || splitsSurrogatePair(haystack, end);
    const fits =
      !splits &&
      !isWordAdjacent(codePointBefore(haystack, at)) &&
      (end >= haystack.length || !isWordAdjacent(String.fromCodePoint(haystack.codePointAt(end)!)));
    if (fits) return true;
    // The run skip is only meaningful from a code-point boundary: asked to scan
    // from inside a surrogate pair, a `u`-flag regex sees the whole astral
    // character and would skip past positions that can still match. A split is
    // rare and cannot drive the quadratic case, so it takes the plain step. Found
    // by fuzzing this against the regex it replaced, not by reading it.
    const next = splits ? at + 1 : afterRunAt(haystack, at);
    if (probe && next > at + 1) probe.skippedRuns += 1;
    from = next;
  }
  return false;
}

/**
 * Deterministic complexity evidence for the whole-word walk.
 *
 * This is exported from the implementation module, but deliberately not from the
 * package barrel: the continuity regression test can count candidate checks without
 * making a wall-clock budget part of the public API. A retry-one-character-later
 * regression turns one rejected word-adjacent run into thousands of checks, while
 * the production run skip keeps it to one.
 */
export function diagnoseWholeWordSearch(
  haystack: string,
  needle: string,
): WholeWordSearchProbe & { matched: boolean } {
  const probe: WholeWordSearchProbe = { candidateChecks: 0, skippedRuns: 0 };
  const matched = occursAsWholeWord(haystack, needle, probe);
  return { ...probe, matched };
}

/**
 * Learner-facing text that can constitute an early lexical use.
 *
 * Etymology blocks deliberately do not. They name roots and historical forms
 * as EXPLANATIONS, and those obligations have their own root ledger; treating
 * every cognate as vocabulary made Malayalam `അത്` look taught 131 lessons
 * early merely because `അതെ` explains where it came from.
 *
 * A decomposition equation is likewise a statement about PARTS, not a claim
 * that each part is a free-standing word. Tamil `புரி + கிற் + அது → புரிகிறது`
 * is the canonical failure: `அது` is the verb's ending there, not the pronoun.
 * The two-symbol test is intentionally narrow so an ordinary sentence that
 * happens to contain an arrow or a plus sign remains visible.
 */
function forwardReferenceBody(lesson: ParsedLesson): string {
  // Walk the LOSSLESS body rather than joining parsed block Markdown. The parser
  // intentionally removes hl-knowledge/activity directives from block.markdown;
  // silently dropping them here would broaden this change beyond the two teaching
  // contexts under review and make the corpus movement harder to account for.
  const kept: string[] = [];
  let blockIndex = -1;
  let inEtymology = false;
  for (const line of lesson.body.split(/\r?\n/)) {
    const trimmed = line.trimStart();
    if (trimmed.startsWith("## ") && !trimmed.startsWith("### ")) {
      blockIndex += 1;
      inEtymology = lesson.blocks[blockIndex]?.type === "etymology";
    }
    if (inEtymology || (line.includes("+") && line.includes("→"))) continue;
    kept.push(line);
  }
  return kept.join("\n");
}

/** Emphasised or code-spanned runs, where the corpus marks target language. */
function emphasisedRuns(body: string): string {
  const runs: string[] = [];
  for (const match of body.matchAll(/\*\*([^*]+)\*\*|\*([^*]+)\*|`([^`]+)`/g)) {
    runs.push(match[1] ?? match[2] ?? match[3] ?? "");
  }
  return runs.join("   ").toLowerCase();
}

/** Measure order integrity, reinforcement windows, and forward references. */
export function measureContinuity(lessons: ParsedLesson[]): ContinuityReport {
  const order: OrderDefect[] = [];
  const reinforcement: ReinforcementDefect[] = [];
  const forwardReferences: ForwardReference[] = [];
  const tracks: TrackContinuity[] = [];
  // Seeded FROM the window list, not hand-written: a hand-written literal compiles
  // fine when a window is added and then silently drops that column from the
  // rendered report whenever it happens to have zero misses.
  const missedByWindow = Object.fromEntries(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, 0]),
  ) as Record<WindowName, number>;

  const byTrack = new Map<string, ParsedLesson[]>();
  for (const lesson of lessons) {
    let group = byTrack.get(lesson.language);
    if (!group) byTrack.set(lesson.language, (group = []));
    group.push(lesson);
  }

  for (const [language, group] of [...byTrack].sort((a, b) => a[0].localeCompare(b[0]))) {
    const ordered = [...group].sort(readingOrder);
    const last = ordered.length - 1;
    const positionOf = new Map<string, number>();
    ordered.forEach((lesson, index) => positionOf.set(lesson.realization.lessonId, index));

    const track: TrackContinuity = {
      language,
      lessonCount: ordered.length,
      lessonsWithoutSequence: 0,
      forwardPrerequisites: 0,
      forwardReviews: 0,
      atomsTaught: 0,
      atomsNeverRevisited: 0,
      forwardReferences: 0,
    };

    // ---- (a) order integrity ----
    const seenSequence = new Map<number, string>();
    ordered.forEach((lesson, index) => {
      const id = lesson.realization.lessonId;
      const chapter =
        typeof lesson.realization.chapter === "number" && Number.isFinite(lesson.realization.chapter)
          ? lesson.realization.chapter
          : null;
      const sequence = declaredSequence(lesson);

      if (sequence === null) {
        track.lessonsWithoutSequence += 1;
        order.push({
          lessonId: id,
          language,
          chapter,
          kind: "no-sequence",
          detail: "no declared reading order; position is a fallback, not a fact",
        });
      } else {
        const clash = seenSequence.get(sequence);
        if (clash !== undefined) {
          order.push({
            lessonId: id,
            language,
            chapter,
            kind: "duplicate-sequence",
            other: clash,
            detail: `sequence ${sequence} is also claimed by ${clash}`,
          });
        } else {
          seenSequence.set(sequence, id);
        }
      }

      // You cannot review a lesson that has not happened yet. `reviews_of` cannot
      // close a reinforcement window (it names lessons, not atoms), but it is still
      // an authored claim about order, and a claim pointing forward is wrong on its
      // own terms: ES-C07-beber reviews ES-C07-vivir, which curriculum.json places
      // AFTER it.
      for (const reviewed of frontmatterList(lesson, "reviews_of")) {
        const at = positionOf.get(reviewed);
        if (at !== undefined && at > index) {
          track.forwardReviews += 1;
          order.push({
            lessonId: id,
            language,
            chapter,
            kind: "forward-review",
            other: reviewed,
            detail: `reviews ${reviewed}, which the learner does not reach for another ${at - index} lesson(s)`,
          });
        }
      }

      for (const prerequisite of frontmatterList(lesson, "prerequisites")) {
        const at = positionOf.get(prerequisite);
        if (at !== undefined && at > index) {
          track.forwardPrerequisites += 1;
          order.push({
            lessonId: id,
            language,
            chapter,
            kind: "forward-prerequisite",
            other: prerequisite,
            detail: `requires ${prerequisite}, which comes ${at - index} lesson(s) later`,
          });
        }
      }
    });

    // ---- (b) reinforcement windows ----
    const firstTaught = new Map<string, { by: string; at: number }>();
    ordered.forEach((lesson, index) => {
      for (const atom of frontmatterList(lesson, "introduces.knowledge")) {
        if (!firstTaught.has(atom)) {
          firstTaught.set(atom, { by: lesson.realization.lessonId, at: index });
        }
      }
      for (const block of lesson.blocks ?? []) {
        for (const atom of block.knowledge?.introduces ?? []) {
          if (!firstTaught.has(atom)) {
            firstTaught.set(atom, { by: lesson.realization.lessonId, at: index });
          }
        }
      }
    });

    const practisedAt = new Map<string, number[]>();
    ordered.forEach((lesson, index) => {
      for (const atom of practisedAtoms(lesson)) {
        let positions = practisedAt.get(atom);
        if (!positions) practisedAt.set(atom, (positions = []));
        positions.push(index);
      }
    });

    track.atomsTaught = firstTaught.size;
    for (const [atom, { by, at }] of [...firstTaught].sort((a, b) => a[0].localeCompare(b[0]))) {
      const later = (practisedAt.get(atom) ?? []).filter((position) => position > at);
      if (later.length === 0) track.atomsNeverRevisited += 1;

      const missed: WindowName[] = [];
      for (const window of REINFORCEMENT_WINDOWS) {
        // Only judge a window the track was long enough to contain. A 25-lesson
        // track missing R4 has not failed; it has not got there yet.
        if (at + window.from > last) continue;
        if (!later.some((position) => position - at >= window.from && position - at <= window.to)) {
          missed.push(window.name);
          missedByWindow[window.name] = (missedByWindow[window.name] ?? 0) + 1;
        }
      }
      if (missed.length > 0) {
        reinforcement.push({
          atom,
          language,
          introducedBy: by,
          introducedAt: at,
          missed,
          revisits: later.length,
        });
      }
    }

    // ---- (c) forward references ----
    // Only words the course ITSELF teaches later are reported. That is provable
    // — the later lesson's own headword is the evidence — and it never
    // false-positives on ordinary English prose. The honest blind spot: a word
    // the course never teaches anywhere (chapter 7's "¿Algo más?") is invisible
    // here, because nothing in the data says it is target language at all.
    const earliestTeaching = new Map<string, { by: string; at: number }>();
    ordered.forEach((lesson, index) => {
      // Only lessons that teach a WORD or PHRASE create a matcher. A `writing`
      // lesson teaching one letter, or a `grammar` lesson whose headword is a
      // rule, is not vocabulary the learner "meets too early" — and treating it
      // as such produced the single-character false positives above.
      if (!CONTENT_TYPES.has(lesson.realization.type)) return;
      for (const word of taughtWords(lesson).filter(usableAsMatcher)) {
        const existing = earliestTeaching.get(word);
        if (!existing || index < existing.at) {
          earliestTeaching.set(word, { by: lesson.realization.lessonId, at: index });
        }
      }
    });

    // The candidate list, built ONCE per track, and INDEXED by leading run.
    //
    // This is the whole module's cost centre, and it used to be written the obvious
    // way: for every lesson, ask every word the track teaches whether it appears.
    // That is quadratic in track length — 549 Spanish lessons against ~550 taught
    // words is 300,000 questions — and the corpus grows by design, so the quadratic
    // term is exactly what turned this walk into a recurring CI timeout. Each
    // question was also dear:
    //
    //   1. `new RegExp` inside the inner loop rebuilt the same per-word pattern once
    //      per lesson, and those patterns are expensive — see `WORD_ADJACENT`.
    //   2. The English-collision guard ran per lesson for a decision that depends
    //      only on the word, so a collision word was re-rejected 549 times instead
    //      of being dropped from the list once.
    //   3. Every surviving question scanned the whole lesson body with a lookbehind
    //      regex, for a word that is almost never in that lesson at all.
    //
    // So the question is turned around. A match must BEGIN a maximal run of
    // word-adjacent characters and must run to the end of that run — that is what
    // the two lookarounds say — so a word can only occur in a lesson whose text
    // contains the word's own leading run as a whole run. `hola` can only be in a
    // lesson whose runs include `hola`; `buenos días` only in one whose runs
    // include `buenos`. Indexing candidates by that run turns "every word against
    // every lesson" into "the handful of words this lesson's vocabulary can even
    // reach", which is proportional to the lesson, not to the corpus.
    //
    // The index only ever ADDS candidates that must then pass the real test below,
    // so it cannot invent a forward reference; and the runs are drawn with the same
    // character class the lookarounds use, so it cannot hide one either.
    //
    // Order is preserved exactly: candidates keep `earliestTeaching`'s own insertion
    // order and are visited by ascending position, and the final
    // `forwardReferences.sort` is NOT a total order (two words taught by the same
    // lesson tie on both keys), so a stable sort leaves this order observable in the
    // published report. Reordering here would silently reshuffle the printed list.
    const candidates: Array<{ word: string; teaching: { by: string; at: number } }> = [];
    const byLeadingRun = new Map<string, number[]>();
    // Words that begin on punctuation or a digit — `¿qué`, `3d` — have no leading
    // run to index by, so they stay in the always-considered list. There are very
    // few, and silently dropping them would lose real defects.
    const unindexed: number[] = [];
    for (const [word, teaching] of earliestTeaching) {
      // The English-collision guard applies on BOTH paths, and BEFORE any matching is
      // attempted — otherwise every collision word costs work in every lesson of the
      // track for a value thrown away on the next line. Applying it only to plain
      // prose let "**no** glide, no drift" — ordinary emphasised English — report
      // `no` as a forward reference from 7 different lessons.
      if (ENGLISH_COLLISIONS.has(word)) continue;
      const position = candidates.length;
      candidates.push({ word, teaching });
      const run = leadingRun(word);
      if (run === "") {
        unindexed.push(position);
        continue;
      }
      let bucket = byLeadingRun.get(run);
      if (!bucket) byLeadingRun.set(run, (bucket = []));
      bucket.push(position);
    }

    ordered.forEach((lesson, index) => {
      const learnerBody = forwardReferenceBody(lesson);
      const body = learnerBody.toLowerCase();
      const emphasised = emphasisedRuns(learnerBody);
      // A lesson can never forward-reference a word sitting in its own headword —
      // `mع السلامة` contains `السلامة`, `bom dia` contains `dia`. The old strip
      // seeded these incidentally; the allowlist does not, so they are added
      // explicitly. Without this, four lessons were reported as borrowing a word they
      // were themselves teaching, which is exactly what this module's own docstring
      // says must not happen.
      const own = new Set([...taughtWords(lesson), ...ownHeadwordTokens(lesson)]);
      const reported = new Set<string>();

      // Both haystacks are indexed, not just the body: the emphasis path has no
      // four-character floor, so a word can be reachable there and nowhere else.
      const reachable = new Set<number>(unindexed);
      addReachableCandidates(body, byLeadingRun, reachable);
      addReachableCandidates(emphasised, byLeadingRun, reachable);

      for (const position of [...reachable].sort((a, b) => a - b)) {
        const { word, teaching } = candidates[position]!;
        if (teaching.at <= index || own.has(word) || reported.has(word)) continue;
        // A plain substring test before the boundary walk: a word absent from the
        // text cannot be present at a word boundary either, and both haystacks and
        // every candidate word are already lower-cased, so the two agree on case.
        //
        // A trailing hyphen means a compound like "pan-Hispanic" — English, not a
        // borrowed headword — which is why `-` counts as word-adjacent.
        const inBodyText = body.includes(word);
        const inEmphasisText = emphasised.includes(word);
        if (!inBodyText && !inEmphasisText) continue;
        const inPlain = word.length >= 4 && inBodyText && occursAsWholeWord(body, word);
        const inEmphasis = inEmphasisText && occursAsWholeWord(emphasised, word);
        if (!inPlain && !inEmphasis) continue;

        reported.add(word);
        track.forwardReferences += 1;
        forwardReferences.push({
          lessonId: lesson.realization.lessonId,
          language,
          position: index,
          word,
          taughtBy: teaching.by,
          lessonsEarly: teaching.at - index,
        });
      }
    });

    tracks.push(track);
  }

  // Worst first, so each list is a work queue rather than a set.
  order.sort((a, b) => a.language.localeCompare(b.language) || a.lessonId.localeCompare(b.lessonId));
  reinforcement.sort(
    (a, b) => b.missed.length - a.missed.length || a.atom.localeCompare(b.atom),
  );
  forwardReferences.sort(
    (a, b) => b.lessonsEarly - a.lessonsEarly || a.lessonId.localeCompare(b.lessonId),
  );

  const atomsTaught = tracks.reduce((sum, track) => sum + track.atomsTaught, 0);
  const atomsNeverRevisited = tracks.reduce((sum, track) => sum + track.atomsNeverRevisited, 0);

  return {
    windows: REINFORCEMENT_WINDOWS,
    order,
    reinforcement,
    forwardReferences,
    tracks,
    summary: {
      lessonsWithoutSequence: tracks.reduce((sum, t) => sum + t.lessonsWithoutSequence, 0),
      tracksWithUnorderedLessons: tracks.filter((t) => t.lessonsWithoutSequence > 0).length,
      forwardPrerequisites: tracks.reduce((sum, t) => sum + t.forwardPrerequisites, 0),
      forwardReviews: tracks.reduce((sum, t) => sum + t.forwardReviews, 0),
      atomsTaught,
      atomsNeverRevisited,
      neverRevisitedPercent:
        atomsTaught === 0 ? 0 : Math.round((atomsNeverRevisited / atomsTaught) * 100),
      missedByWindow,
      forwardReferences: tracks.reduce((sum, t) => sum + t.forwardReferences, 0),
    },
  };
}
