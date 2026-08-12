// ---------------------------------------------------------------------------
// atommastery.ts — remembering how well this learner holds each ATOM.
//
// THE PROBLEM (HL10 §10.1). Everything the app schedules today is keyed to a
// LESSON or a review cell: you complete a lesson and move on. At 5,000 lessons
// and roughly 10,000 atoms that is the wrong unit, because the thing a learner
// actually forgets is not a lesson. It is `ES-LEX-GRACIAS`.
//
// A lesson is a container. It teaches one or two atoms and practises a dozen
// more, and two learners who have "completed" the same lesson can hold entirely
// different subsets of it. Scheduling the container tells you nothing about the
// contents.
//
// THE DISTINCTION THAT MATTERS. The corpus already guarantees the *material*:
// the R1–R4 review windows mean some lesson somewhere practises every atom
// again, on a schedule fixed at authoring time. That is the CORPUS schedule and
// it is the same for everybody.
//
// What this module owns is the LEARNER schedule: when does *this* person need
// to see `ES-LEX-GRACIAS` again, given their own record of hits and misses.
// Conflating the two is why `reviews_of` never reinforced anything for anybody
// in particular — it was a promise about the book, read as a promise about you.
//
// THE MODEL. Deliberately small, and every part of it is explainable to the
// learner if we ever choose to show it:
//
//   strength  0..1, how well the atom is held right now.
//   lastSeen  when it was last practised, so strength can decay from there.
//   dueAt     when it should next be practised.
//   lapses    how many times it has been missed, ever. A high-lapse atom is a
//             genuinely hard one and worth surfacing differently.
//
// Strength moves asymptotically on a hit (you can approach 1 but never reach it
// in one go) and multiplicatively on a miss (a miss costs you most of what a
// hit gave). Between sessions it decays with a half-life. All three choices are
// the ordinary spaced-repetition shape, chosen here because they are easy to
// reason about rather than because they are optimal.
//
// PURITY. Every function here takes `now` as an argument and returns new
// values. Nothing reads the clock, nothing touches storage. That is what makes
// a memory model testable without waiting a week for an interval to elapse.
// ---------------------------------------------------------------------------

/** What we know about one atom, for one learner. */
export interface AtomMastery {
  /** The atom id, e.g. `ES-LEX-GRACIAS`. */
  atom: string;
  /** Sequence position of the lesson that first taught it, or 0 if unknown. */
  introducedAt: number;
  /** 0..1, decaying. How well it is held as of `lastSeen`. */
  strength: number;
  /** Epoch ms of the last practice. */
  lastSeen: number;
  /** Epoch ms at which it should next be practised. */
  dueAt: number;
  /** How many times this atom has ever been missed. */
  lapses: number;
}

/** Atom id → mastery. A plain Map so callers can iterate cheaply. */
export type MasteryBook = Map<string, AtomMastery>;

/**
 * How long an untouched atom takes to lose half its strength.
 *
 * Ten days is a deliberate middle: short enough that a month away genuinely
 * costs you something (which is true), long enough that a busy fortnight does
 * not wipe a hundred atoms and bury the learner in review the moment they come
 * back (which would be false, and punishing).
 */
export const HALF_LIFE_MS = 10 * 24 * 60 * 60 * 1000;

/** The shortest gap we will ever schedule: one minute. */
export const MIN_INTERVAL_MS = 60 * 1000;

/** The longest: 180 days. Beyond this the estimate is fiction anyway. */
export const MAX_INTERVAL_MS = 180 * 24 * 60 * 60 * 1000;

/** Strength at or above this counts as "held" for reporting and generation. */
export const HELD_THRESHOLD = 0.6;

/** A brand-new atom, never practised. */
export function newAtom(atom: string, introducedAt = 0, now = 0): AtomMastery {
  return {
    atom,
    introducedAt: Math.max(0, Math.trunc(introducedAt)),
    // Not zero: it has just been taught, so it is held weakly rather than not
    // at all. Starting at zero would make a freshly-taught atom look identical
    // to one the learner has never met.
    strength: 0.3,
    lastSeen: now,
    dueAt: now + intervalFor(0.3),
    lapses: 0,
  };
}

/**
 * How long to wait before practising an atom of this strength again.
 *
 * Cubic rather than linear: the gap between "just met it" and "know it cold"
 * should be days, not a constant factor. A strength of 1 gives the full 180
 * days; 0.5 gives about three weeks; 0.3 gives roughly five days.
 */
export function intervalFor(strength: number): number {
  const s = clamp01(strength);
  return Math.max(MIN_INTERVAL_MS, Math.round(MAX_INTERVAL_MS * s * s * s));
}

/**
 * The strength an atom actually has *now*, after decay since `lastSeen`.
 *
 * The stored `strength` is a reading taken at `lastSeen`; this is the reading
 * taken today. Callers that want to sort or filter by how well something is
 * held must use this, not the raw field.
 */
export function strengthNow(mastery: AtomMastery, now: number): number {
  const elapsed = Math.max(0, now - mastery.lastSeen);
  if (elapsed === 0) return clamp01(mastery.strength);
  return clamp01(mastery.strength * Math.pow(0.5, elapsed / HALF_LIFE_MS));
}

/**
 * Record one practice of one atom.
 *
 * A hit closes 40% of the remaining distance to 1 — so repeated success is
 * strongly rewarded early and barely moves an atom already known cold, which is
 * the right shape: there is nothing left to learn there.
 *
 * A miss keeps 35% of current strength and counts a lapse. It costs more than a
 * hit gains, deliberately: forgetting is evidence about the future, and the
 * cheapest moment to fix it is now.
 */
export function practise(mastery: AtomMastery, correct: boolean, now: number): AtomMastery {
  const current = strengthNow(mastery, now);
  const strength = correct ? current + (1 - current) * 0.4 : current * 0.35;
  return {
    ...mastery,
    strength: clamp01(strength),
    lastSeen: now,
    dueAt: now + intervalFor(strength),
    lapses: mastery.lapses + (correct ? 0 : 1),
  };
}

/**
 * Record one practice across every atom an activity assessed.
 *
 * Atoms not yet in the book are created first, so a lesson can credit atoms the
 * learner has only just met. Returns a NEW map; the input is untouched.
 */
export function practiseAll(
  book: MasteryBook,
  atoms: Iterable<string>,
  correct: boolean,
  now: number,
  introducedAt = 0,
): MasteryBook {
  const next = new Map(book);
  for (const atom of atoms) {
    if (typeof atom !== "string" || atom === "") continue;
    const existing = next.get(atom) ?? newAtom(atom, introducedAt, now);
    next.set(atom, practise(existing, correct, now));
  }
  return next;
}

/**
 * The atoms that are due, most overdue first.
 *
 * "Overdue" is measured in multiples of the interval that was scheduled, not in
 * absolute time — an atom on a five-day interval that is five days late has
 * decayed as much as one on a ninety-day interval that is ninety days late, and
 * ranking by raw lateness would bury the short-interval ones that need the work.
 */
export function dueAtoms(book: MasteryBook, now: number, limit = Infinity): AtomMastery[] {
  const due: Array<{ mastery: AtomMastery; overdue: number }> = [];
  for (const mastery of book.values()) {
    if (mastery.dueAt > now) continue;
    const scheduled = Math.max(MIN_INTERVAL_MS, mastery.dueAt - mastery.lastSeen);
    due.push({ mastery, overdue: (now - mastery.dueAt) / scheduled });
  }
  due.sort((a, b) => b.overdue - a.overdue || a.mastery.atom.localeCompare(b.mastery.atom));
  return due.slice(0, limit === Infinity ? undefined : Math.max(0, limit)).map((d) => d.mastery);
}

/** Atoms held at or above `HELD_THRESHOLD` right now — what a generator may use. */
export function heldAtoms(book: MasteryBook, now: number): string[] {
  const held: string[] = [];
  for (const mastery of book.values()) {
    if (strengthNow(mastery, now) >= HELD_THRESHOLD) held.push(mastery.atom);
  }
  return held.sort();
}

/** What the app can show, and what a test can pin. */
export interface MasterySummary {
  tracked: number;
  held: number;
  due: number;
  /** Atoms missed three or more times — the genuinely hard ones. */
  stubborn: number;
  /** Mean current strength across every tracked atom, 0 when none. */
  meanStrength: number;
}

export function masterySummary(book: MasteryBook, now: number): MasterySummary {
  let held = 0;
  let due = 0;
  let stubborn = 0;
  let total = 0;
  for (const mastery of book.values()) {
    const s = strengthNow(mastery, now);
    total += s;
    if (s >= HELD_THRESHOLD) held += 1;
    if (mastery.dueAt <= now) due += 1;
    if (mastery.lapses >= 3) stubborn += 1;
  }
  const tracked = book.size;
  return {
    tracked,
    held,
    due,
    stubborn,
    meanStrength: tracked === 0 ? 0 : Math.round((total / tracked) * 1000) / 1000,
  };
}

function clamp01(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(1, value));
}
