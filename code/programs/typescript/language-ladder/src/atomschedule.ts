// ---------------------------------------------------------------------------
// atomschedule.ts — choosing what to review, from the learner's own record.
//
// This is the second half of HL10 §10.1, and the half that actually changes
// what the app does. `atommastery.ts` recorded what this learner holds; this
// decides what to put in front of them because of it.
//
// THE PROBLEM WITH REVIEWING BY LESSON. The obvious scheduler shows you the
// lesson you saw longest ago. That is wrong in a course this size, because a
// lesson practises a dozen atoms and you have almost certainly not forgotten
// all of them. Re-showing a whole lesson to refresh one atom is the tax that
// makes review feel like a chore.
//
// THE APPROACH: SET COVER, GREEDILY. Take the atoms that are actually due,
// then repeatedly pick the completed lesson that refreshes the most of them,
// removing those atoms from the pool each time. Three or four lessons usually
// cover a surprising fraction of the due set, because the corpus's R1–R4
// windows already built lessons that touch many atoms at once.
//
// Greedy set cover is not optimal — no cheap algorithm is — but it is within a
// known factor of optimal, it is trivial to explain to a learner ("these three
// lessons cover most of what you owe"), and it never produces a pathological
// result. Optimality is worth much less here than predictability.
//
// WHY ONLY COMPLETED LESSONS. You cannot review something you have not studied.
// A lesson the learner has never reached may well practise a due atom, but
// putting it in a review queue would leak the course's forward order and hand
// them a lesson whose prerequisites they do not hold.
// ---------------------------------------------------------------------------

import { type MasteryBook, dueAtoms } from "./atommastery.ts";

/** The minimum a lesson must expose for this module to schedule it. */
export interface SchedulableLesson {
  id: string;
  language: string;
  /** Atoms this lesson would refresh: everything its activities assess. */
  refreshes: readonly string[];
}

/** One recommendation: a lesson, and which due atoms it would refresh. */
export interface ReviewPick {
  lessonId: string;
  language: string;
  /** The due atoms this pick covers, sorted. Never empty. */
  covers: string[];
}

/**
 * Which atoms a lesson would refresh.
 *
 * MUST MATCH WHAT THE ANSWER PATHS CREDIT, exactly. Crediting and scheduling
 * disagreeing is not a cosmetic bug: an atom scheduled by a lesson that will
 * never credit it stays due forever, and the learner is handed the same review
 * every day with no way to clear it.
 *
 * The app credits two things, so this must union two things:
 *
 *   - an authored activity credits its own `assesses` list;
 *   - a lesson whose focused check is a plain meaning check has no activity at
 *     all, and credits the lesson's `introducesAtoms` instead.
 *
 * The second case is not rare. It is every lesson that never authored an
 * `hl-activity` in its recall block — which, at the time of writing, includes
 * the very first lesson of the course.
 */
export function refreshesOf(
  lesson: {
    activities?: ReadonlyArray<{ assesses: readonly string[] }>;
    introducesAtoms?: readonly string[];
  },
): string[] {
  const seen = new Set<string>();
  for (const activity of lesson.activities ?? []) {
    for (const atom of activity.assesses) {
      if (typeof atom === "string" && atom !== "") seen.add(atom);
    }
  }
  for (const atom of lesson.introducesAtoms ?? []) {
    if (typeof atom === "string" && atom !== "") seen.add(atom);
  }
  return [...seen].sort();
}

/**
 * Pick the lessons that best cover the learner's due atoms.
 *
 * `completed` is the set of lesson ids the learner has passed. `limit` caps how
 * many picks are returned — a review queue longer than a sitting is a queue
 * nobody starts.
 */
export function reviewPicks(
  book: MasteryBook,
  lessons: readonly SchedulableLesson[],
  completed: ReadonlySet<string>,
  now: number,
  limit = 3,
): ReviewPick[] {
  const due = new Set(dueAtoms(book, now).map((mastery) => mastery.atom));
  if (due.size === 0 || limit <= 0) return [];

  // Only lessons the learner has actually studied, and only ones that would
  // refresh something currently owed.
  const candidates = lessons.filter((lesson) => completed.has(lesson.id));

  const picks: ReviewPick[] = [];
  const remaining = new Set(due);
  const used = new Set<string>();

  while (picks.length < limit && remaining.size > 0) {
    let best: { lesson: SchedulableLesson; covers: string[] } | null = null;
    for (const lesson of candidates) {
      if (used.has(lesson.id)) continue;
      const covers = lesson.refreshes.filter((atom) => remaining.has(atom));
      if (covers.length === 0) continue;
      // Ties break on lesson id, so the same book and clock always produce the
      // same queue. A review list that reshuffles between renders is unusable.
      if (
        best === null ||
        covers.length > best.covers.length ||
        (covers.length === best.covers.length && lesson.id < best.lesson.id)
      ) {
        best = { lesson, covers };
      }
    }
    if (best === null) break;
    used.add(best.lesson.id);
    for (const atom of best.covers) remaining.delete(atom);
    picks.push({
      lessonId: best.lesson.id,
      language: best.lesson.language,
      covers: [...best.covers].sort(),
    });
  }
  return picks;
}

/** How much of the due set a queue of picks would clear, as a 0..1 fraction. */
export function coverageOf(book: MasteryBook, picks: readonly ReviewPick[], now: number): number {
  const due = dueAtoms(book, now).length;
  if (due === 0) return 1;
  const covered = new Set<string>();
  for (const pick of picks) for (const atom of pick.covers) covered.add(atom);
  return Math.round((covered.size / due) * 1000) / 1000;
}
