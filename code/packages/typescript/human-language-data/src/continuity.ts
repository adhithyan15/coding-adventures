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
    // Headwords carry their article: "el pan", "el agua", "la casa". A body that
    // says "bebo agua" is using the same word, so the bare noun must match too.
    // Only a SHORT leading function word is stripped, which leaves "buenos días"
    // whole — splitting that would report `días` from every lesson mentioning it.
    const match = /^(\S{1,3})\s+(.+)$/.exec(part);
    if (match) out.add(match[2]!.trim());
  }
  return [...out].filter((word) => word.length > 0);
}

/**
 * English words that are also common target-language headwords.
 *
 * Without this, every English sentence in a Spanish lesson reports `a`, `no`,
 * `son` and `me` as forward references. The list is small and deliberately so:
 * the length rule below does most of the work, and a denylist that grows without
 * bound is a sign the matching rule is wrong.
 */
const ENGLISH_COLLISIONS = new Set([
  "a", "an", "as", "at", "be", "but", "can", "do", "eat", "el", "en", "es", "fin",
  "for", "go", "he", "in", "is", "it", "la", "led", "me", "no", "of", "on", "once",
  "or", "past", "pie", "roman", "san", "sea", "sin", "so", "son", "some", "tag",
  "tan", "the", "three", "to", "us", "van", "was", "we", "y", "you",
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

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** Emphasised or code-spanned runs, where the corpus marks target language. */
function emphasisedRuns(body: string): string {
  const runs: string[] = [];
  for (const match of body.matchAll(/\*\*([^*]+)\*\*|\*([^*]+)\*|`([^`]+)`/g)) {
    runs.push(match[1] ?? match[2] ?? match[3] ?? "");
  }
  return runs.join("   ").toLowerCase();
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

    ordered.forEach((lesson, index) => {
      const body = lesson.body.toLowerCase();
      const emphasised = emphasisedRuns(lesson.body);
      const own = new Set(taughtWords(lesson));
      const reported = new Set<string>();

      for (const [word, teaching] of earliestTeaching) {
        if (teaching.at <= index || own.has(word) || reported.has(word)) continue;
        // The English-collision guard applies on BOTH paths, and BEFORE the regex is
        // built — otherwise every collision word costs a construction in every
        // lesson of the track for a value thrown away on the next line. Applying it
        // only to plain prose let "**no** glide, no drift" — ordinary emphasised
        // English — report `no` as a forward reference from 7 different lessons.
        if (ENGLISH_COLLISIONS.has(word)) continue;
        // A trailing hyphen means a compound like "pan-Hispanic" — English, not a
        // borrowed headword.
        const pattern = new RegExp(
          `(?<![\\p{L}\\p{M}-])${escapeRegExp(word)}(?![\\p{L}\\p{M}-])`,
          "u",
        );
        const inPlain = word.length >= 4 && pattern.test(body);
        const inEmphasis = pattern.test(emphasised);
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
