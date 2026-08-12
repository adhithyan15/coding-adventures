// ---------------------------------------------------------------------------
// synthesisdrill.ts — practice the course could never have authored (HL10 §10.3).
//
// HL09 §6 puts one synthesis activity at the end of every chapter: a prompt
// whose correct answer is an utterance the course has never shown. Those are
// hand-written, so there is exactly one per chapter, and a learner who wants
// more has nowhere to get them.
//
// This generates more, from the learner's own record. The constraint is the
// forward-reference rule (§8.2) run backwards: where the corpus checks that no
// lesson uses a word it has not taught, this checks that no drill uses an atom
// the learner does not currently HOLD. Same rule, opposite direction, and the
// mastery book is what makes the second one possible.
//
// WHAT A DRILL IS. Two to four pieces the learner holds, drawn from DIFFERENT
// domains, and an instruction to put them in one sentence. The pieces have been
// practised; the combination has not. That gap is the entire exercise, and it
// is where a language stops being a list of things you know.
//
// WHY DOMAINS. Three food words in one drill is a vocabulary quiz. A food word,
// a time word and a verb is a sentence you have to build. The corpus already
// carries the information needed to tell those apart: every lesson has a
// `concept_tag`, and the tags are domain-prefixed (`ES-FOOD-WATER`,
// `ES-TIME-MORNING`, `VERB-EAT`).
//
// WHAT THIS DELIBERATELY DOES NOT DO. The spec's example prompt is situational:
// "You are hungry, it is morning, and you are speaking to your boss." Producing
// that needs per-atom situational metadata the corpus does not carry — a food
// word is tagged as food, not as "something you eat when hungry". Inventing the
// situation would mean inventing facts about the atoms, so the prompt here
// names the pieces plainly instead. The richer framing is a corpus change, not
// a generator change, and pretending otherwise would produce prompts that are
// occasionally nonsense.
//
// SCORING, HONESTLY. An open utterance cannot be graded without a parser this
// app does not have. So the check is the one thing that IS mechanically
// decidable and is also exactly what the drill claims to test: did the answer
// actually contain each piece? Whether the sentence around them is good Spanish
// is not something a substring test can know, and this does not pretend to.
// ---------------------------------------------------------------------------

import { HELD_THRESHOLD, type MasteryBook, strengthNow } from "./atommastery.ts";

/** The minimum a lesson must expose to supply a drill piece. */
export interface DrillableLesson {
  id: string;
  language: string;
  headword: string;
  gloss: string;
  /** The lesson's `concept_tag`; its prefix is the domain. */
  concept: string;
  /** Atoms the lesson teaches — the ones whose strength gates it. */
  introducesAtoms: readonly string[];
}

/** One piece of a drill: something held, and what kind of thing it is. */
export interface DrillPiece {
  lessonId: string;
  headword: string;
  gloss: string;
  /** Human-readable domain, e.g. "a food word". */
  domain: string;
  /** Machine domain key, used to keep a drill's pieces distinct. */
  domainKey: string;
}

export interface SynthesisDrill {
  language: string;
  prompt: string;
  pieces: DrillPiece[];
}

/**
 * Domain keys that make a usable drill piece, and how to say them.
 *
 * Grammar-only tags (`ES-GRAMMAR-…`, `ES-REVIEW-…`, `ES-SOUND-…`) are absent on
 * purpose: "use a grammar rule in a sentence" is not an instruction anybody can
 * follow. A drill is built from things with a headword you can actually say.
 */
const DOMAINS = new Map<string, string>([
  ["ES-FOOD", "a food or drink word"],
  ["ES-PLACE", "a place word"],
  ["ES-OBJECT", "an everyday object"],
  ["ES-ANIMAL", "an animal"],
  ["ES-FAMILY", "a family word"],
  ["ES-BODY", "a body word"],
  ["ES-COLOUR", "a colour"],
  ["ES-TIME", "a time expression"],
  ["ES-DAYS", "a day of the week"],
  ["ES-MONTHS", "a month"],
  ["ES-SEASONS", "a season"],
  ["ES-WEATHER", "a weather expression"],
  ["ES-NUM", "a number"],
  ["VERB", "a verb"],
  ["GREETING", "a greeting"],
  ["FAREWELL", "a farewell"],
  ["COURTESY", "a courtesy phrase"],
  ["ES-COURTESY", "a courtesy phrase"],
  ["ES-QUESTION", "a question word"],
]);

/** The domain a concept tag belongs to, or null when it makes no drill piece. */
export function domainOf(concept: string): { key: string; label: string } | null {
  if (typeof concept !== "string" || concept === "") return null;
  // Longest prefix wins, so `ES-COURTESY` is not swallowed by a shorter key.
  let best: { key: string; label: string } | null = null;
  for (const [key, label] of DOMAINS) {
    if (concept === key || concept.startsWith(`${key}-`)) {
      if (best === null || key.length > best.key.length) best = { key, label };
    }
  }
  return best;
}

/** Is every atom this lesson teaches currently held? */
export function lessonIsHeld(book: MasteryBook, lesson: DrillableLesson, now: number): boolean {
  const atoms = lesson.introducesAtoms;
  if (atoms.length === 0) return false;
  return atoms.every((atom) => {
    const mastery = book.get(atom);
    return mastery !== undefined && strengthNow(mastery, now) >= HELD_THRESHOLD;
  });
}

/**
 * Build one drill, or null when the learner does not yet hold enough.
 *
 * `seed` makes the choice deterministic: the same seed and the same book give
 * the same drill, so a re-render does not silently swap the exercise out from
 * under somebody halfway through answering it. The caller supplies the seed
 * (a counter, a timestamp) rather than this reading a clock.
 */
export function synthesisDrill(
  book: MasteryBook,
  lessons: readonly DrillableLesson[],
  now: number,
  seed = 0,
  size = 3,
): SynthesisDrill | null {
  const byDomain = new Map<string, DrillPiece[]>();
  for (const lesson of lessons) {
    const domain = domainOf(lesson.concept);
    if (!domain) continue;
    if (lesson.headword.trim() === "" || lesson.headword.startsWith("(")) continue;
    if (!lessonIsHeld(book, lesson, now)) continue;
    const piece: DrillPiece = {
      lessonId: lesson.id,
      headword: lesson.headword,
      gloss: lesson.gloss,
      domain: domain.label,
      domainKey: domain.key,
    };
    const bucket = byDomain.get(domain.key);
    if (bucket) bucket.push(piece);
    else byDomain.set(domain.key, [piece]);
  }

  // Two pieces is the smallest thing that is a combination rather than a recall.
  const domains = [...byDomain.keys()].sort();
  if (domains.length < 2) return null;

  const wanted = Math.max(2, Math.min(size, domains.length));
  const pieces: DrillPiece[] = [];
  for (let i = 0; i < wanted; i += 1) {
    // Rotate through the domains by seed so consecutive drills differ, and pick
    // within a domain by the same seed so the choice is reproducible.
    const domainKey = domains[(seed + i) % domains.length]!;
    const bucket = byDomain.get(domainKey)!;
    const sorted = [...bucket].sort((a, b) => a.lessonId.localeCompare(b.lessonId));
    pieces.push(sorted[(seed + i) % sorted.length]!);
  }

  const language = pieces[0] ? lessonLanguage(lessons, pieces[0].lessonId) : "";
  const list = pieces.map((piece) => piece.domain).join(", ");
  return {
    language,
    prompt:
      `Say one sentence that uses all of these: ${list}.` +
      " You have practised each of them. You have never been shown them together.",
    pieces,
  };
}

function lessonLanguage(lessons: readonly DrillableLesson[], id: string): string {
  return lessons.find((lesson) => lesson.id === id)?.language ?? "";
}

/**
 * Which pieces the answer actually contained.
 *
 * Accent- and case-insensitive, because a learner typing on an English keyboard
 * should not fail a synthesis drill over a missing acute. Returns the pieces
 * found; the caller decides what to do about the ones that are missing.
 */
export function piecesUsed(answer: string, pieces: readonly DrillPiece[]): DrillPiece[] {
  const haystack = fold(answer);
  return pieces.filter((piece) => {
    const needle = fold(piece.headword);
    return needle !== "" && haystack.includes(needle);
  });
}

function fold(value: string): string {
  return value
    .normalize("NFKD")
    .replace(/\p{M}/gu, "")
    .toLocaleLowerCase("en")
    .replace(/[^\p{L}\p{N}\s]/gu, " ")
    .replace(/\s+/g, " ")
    .trim();
}
