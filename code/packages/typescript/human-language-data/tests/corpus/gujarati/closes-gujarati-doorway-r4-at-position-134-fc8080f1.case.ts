import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { measureContinuity, REINFORCEMENT_WINDOWS } from "../../../src/continuity.js";
import { defaultCurriculumRoot, loadTrackChapters, loadTrackLessons } from "../../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("closes Gujarati doorway R4 at position 134", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const doorway = [
    "GU-SCRIPT-JA-01",
    "GU-SCRIPT-O-MATRA-01",
    "GU-SCRIPT-ANUSVARA-01",
    "GU-SCRIPT-II-MATRA-01",
    "GU-SCRIPT-U-MATRA-01",
    "GU-SCRIPT-CHHA-01",
    "GU-SCRIPT-KA-01",
    "GU-SCRIPT-NNA-01",
    "GU-SCRIPT-SHA-01",
  ];
  const checkpoint = ordered[134]!;
  expect(checkpoint.realization.lessonId).toBe("GU-R19-doorway-nine-r4");
  expect(checkpoint.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(
    doorway.map((atom) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 134 - introducedAt;
    }),
  ).toEqual([108, 107, 106, 105, 104, 103, 102, 101, 100]);

  const beforeCheckpoint = measureContinuity(ordered.slice(0, 134));
  const afterCheckpoint = measureContinuity(lessons);
  expect(beforeCheckpoint.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(248);
  // HL-C286: 339 -> 283, and DOWN even though the track grew by 49 lessons.
  // The previous tranche's rise was eligibility, not neglect, and this one pays
  // that eligibility off: chapter 30 is the fifth-return slab HL-C271 filed,
  // and chapters 29 and 31-34 carry named distant bands (the chapter-16 table,
  // the chapter-17 household, the chapter-18 face, the chapter-24 map, hand and
  // money) at the exact windows a 228-lesson track makes measurable. R4 misses
  // alone fall 101 -> 43. R3 rose 115 -> 117 because the new chapters' own
  // atoms are now measurable at their third window and the last two chapters
  // sit too close to the end of the book to service theirs; that residue is
  // filed rather than hidden.
  // 283 -> 364. Decomposed against the same corpus measured without chapters
  // 35-41, because a bare rise here says nothing about whose debt it is:
  //   +41  the tranche's OWN atoms -- the last chapters in the book, whose R3
  //        and R4 windows fall past the final lesson and cannot be serviced.
  //   +47  PRE-EXISTING atoms whose windows did not exist until the track grew.
  //        R4 is distance 80-250; at 228 lessons an atom introduced at position
  //        151 had no R4 to miss, and at 263 it does. Nothing about those
  //        lessons changed.
  //   -4   of those 47, closed deliberately: chapter 41's `where` lesson reads
  //        the chapter-22 row (shaalaa, rasto, route-three) cold at distance
  //        ~106, which is inside R4 rather than decorative.
  //   -3   pre-existing misses closed outright by the chapter-opening
  //        retrievals: chhe, hun and kem all reach their R4 for the first time.
  // 364 -> 362. HL-C359's ordinal chapter, decomposed the same way against the
  // corpus measured without chapters 42:
  //    0   created by the tranche's OWN eight atoms. Each of the six lessons
  //        recalls the previous lesson's ordinal, and the payoff services the
  //        first lesson's R2; those are every window a 269-lesson track is long
  //        enough to judge for atoms introduced at positions 263-267.
  //   15   PRE-EXISTING (atom, window) slots that did not exist until the track
  //        grew by six -- one R1, five R2, six R3, three R4 -- and all fifteen
  //        are answered by name in the chapter's own warm-ups.
  //   -2   pre-existing R4 misses closed outright: mahino and atyaare are said
  //        and written again at distance ~80, the first time either has been
  //        inside its fourth window.
  // 362 -> 360. HL-C361's cardinal chapter, decomposed the same way against the
  // corpus measured without chapter 43:
  //    0   created by the tranche's OWN seven atoms. Each lesson recalls the
  //        previous one's item by name, and the last three lessons service the
  //        R2 of chha, saat and the new letter; those are every window a
  //        277-lesson track is long enough to judge for atoms introduced at
  //        positions 269-275.
  //   16   PRE-EXISTING (atom, window) slots that did not exist until the track
  //        grew by eight -- six R2, nine R3, one R4 -- assigned to the exact new
  //        lesson at the window's minimum distance, one slot per lesson on a
  //        diagonal, and all sixteen answered by name in the warm-ups.
  //   -2   pre-existing R4 misses closed outright, and they are exactly the two
  //        letters the new writing lesson sets beside ttha: GU-SCRIPT-TTA-01 and
  //        GU-SCRIPT-THA-01 reach their fourth window for the first time.
  // The doorway assertion below is unchanged and still passes, which is the
  // property this test actually owns.
  // 360 -> 359. Chapter 44's reading rung closes one more window than it opens:
  // its three lessons retrieve signboard words and the opening greetings at a
  // long interval, and the three reading skills they introduce sit at the end of
  // the track where only their R1 is judgeable yet.
  // 359 -> 350. HL-C432's second-pass lesson, decomposed against the corpus
  // measured without it, because a bare fall here says nothing about whose debt
  // it closed:
  //   -10  closed outright by the lesson's thirteen retrievals -- nine R4 and
  //        one R2. The R4s are the opening greeting read, saarun, the chapter-2
  //        and chapter-6 recaps, the three standing vowels, kha and ddha; the R2
  //        is GU-SCRIPT-KAAGAL-01, whose window was still open at distance ~6.
  //    +1  PRE-EXISTING and created by the track growing 280 -> 281:
  //        GU-LEX-KERI-01's R4 (distance 80-250) did not exist at 280 lessons
  //        and does at 281. Nothing about that lesson changed.
  // The lesson sits at sequence 1825, which is index 220 -- far past the
  // checkpoint at 134 -- so every position assertion above is untouched, and
  // that is why it was placed there rather than beside the atoms it retrieves.
  // 350 -> 362. HL-C443 chapter 45 (gha, the ai sign, two reviews), decomposed
  // against the corpus measured without it:
  //    0   created by the chapter's own two atoms: each letter lesson recalls
  //        the one before, and the two reviews give both their revisits.
  //   +12  PRE-EXISTING (atom, window) slots that did not exist until the track
  //        grew 281 -> 285: four R4, five R3, two R2 and one R1, all on atoms of
  //        chapters 37-44 (ordinals, the question words, the reading skills)
  //        whose next window only now falls inside the track.
  //    0   closed.
  // The lessons sit at the end of the track, far past the checkpoint at 134, so
  // every position assertion above is untouched.
  expect(afterCheckpoint.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(362);
  expect(
    afterCheckpoint.reinforcement.filter(
      (defect) => doorway.includes(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);
});
