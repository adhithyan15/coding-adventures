import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { lessonModalities, orderChapterLessons, unstartableCause } from "../../../src/modality.js";

// The Punjabi A2 vocabulary: chapters 225-364, five tranches (a-e) of 28
// chapters with five word lessons each, plus a review closing every few
// chapters. With the A2 core (211-224) it takes the track from A1 to A2 on the
// level gate. These cases pin the shape a later edit could silently break:
// the size of the tranche, that every chapter still opens by ear, and that no
// chapter sits on a B1 node (which would make the track REACH B1 without
// attaining it).
const VOCABULARY = Array.from({ length: 140 }, (_, index) => 225 + index);

// The nodes the five tranches were planned on (HL112): A2, A1 and pre-A1 only.
const ALLOWED_NODES = new Set([
  "SPINE-SAY-WHAT-I-DO",
  "SPINE-NAME-EVERYDAY-ACTIONS",
  "SPINE-NAME-EVERYDAY-THINGS",
  "SPINE-READ-PRACTICAL-TEXTS",
  "SPINE-ASK-LOCATION",
  "SPINE-SAY-WHAT-I-WANT",
  "SPINE-TALK-ABOUT-FUTURE",
  "SPINE-DESCRIBE-QUALITIES",
  "SPINE-TALK-ABOUT-PAST",
  "SPINE-CHECK-WELLBEING",
  "SPINE-EXCHANGE-NAMES",
  "SPINE-NEGATE-AND-ASK",
  "SPINE-SAY-WHAT-I-LIKE",
  "SPINE-SAY-WHAT-I-HAVE-AND-CAN-DO",
]);

function vocabularyLessons() {
  return loadTrackLessons("punjabi").filter((lesson) => {
    const chapter = Number(lesson.frontmatter.chapter);
    return chapter >= 225 && chapter <= 364;
  });
}

it("teaches 700 distinct Punjabi A2 words in 140 chapters, closed by reviews", () => {
  const lessons = vocabularyLessons();
  const words = lessons.filter((lesson) => lesson.frontmatter.type === "word");
  const reviews = lessons.filter((lesson) => lesson.frontmatter.type === "review");
  expect(words).toHaveLength(700);
  expect(new Set(words.map((lesson) => lesson.realization.headword)).size).toBe(700);
  expect(reviews).toHaveLength(25);
  expect(words.length + reviews.length).toBe(lessons.length);
  expect(new Set(lessons.map((lesson) => Number(lesson.frontmatter.chapter)))).toEqual(new Set(VOCABULARY));
});

it("opens every Punjabi A2 vocabulary chapter by ear", () => {
  const lessons = vocabularyLessons();
  for (const chapter of VOCABULARY) {
    const ordered = lessons
      .filter((lesson) => Number(lesson.frontmatter.chapter) === chapter)
      .sort((a, b) => Number(a.frontmatter.sequence) - Number(b.frontmatter.sequence));
    expect(ordered.length, `chapter ${chapter}`).toBeGreaterThanOrEqual(5);
    const [entry] = orderChapterLessons(lessonModalities([ordered[0]!]));
    expect(entry?.coreModality, ordered[0]!.realization.lessonId).toBe("voice");
    expect(unstartableCause([entry!])).toBeNull();
  }
});

it("keeps every vocabulary lesson on an A2, A1 or pre-A1 node", () => {
  for (const lesson of vocabularyLessons()) {
    expect(ALLOWED_NODES.has(String(lesson.frontmatter.spine_node)), lesson.realization.lessonId).toBe(true);
  }
});
