import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { readingOrder } from "../../../src/ramp.js";
import {
  languageWritingStages,
} from "../assert-language-corpus.js";

it("pins Sanskrit's pre-A1 writing ladder", () => {
  const track = languageWritingStages("sanskrit");

  // The track had NO stage evidence at all, so every writing-stage debt read as
  // outstanding while the lessons that could discharge them sat unmarked.
  //
  // The ORDER is asserted rather than the set: `missing-stage-prerequisite`
  // makes a delayed copy invalid unless the tracing and the guided copy come
  // earlier IN SEQUENCE, so a set-equality assertion would pass on a ladder
  // whose rungs are in the wrong order and therefore prove nothing.
  expect(track.validEvidence.map((entry) => [entry.lessonId, entry.stage])).toEqual([
    ["SA-S02-letter-na", "observe-trace"],
    ["SA-S02-copy-the-three-strokes", "guided-copy"],
    ["SA-S02-delayed-copy", "delayed-copy"],
    ["SA-S02-dictation", "dictation-transcription"],
    ["SA-S201-sign-visarga", "observe-trace"],
    ["SA-W03-mama-guided-copy", "guided-copy"],
    ["SA-W03-mama-delayed-copy", "delayed-copy"],
    ["SA-W03-mama-dictation", "dictation-transcription"],
    ["SA-W03-mama-nama-guided-copy", "guided-copy"],
    ["SA-W03-mama-nama-delayed-copy", "delayed-copy"],
    ["SA-W03-mama-nama-dictation", "dictation-transcription"],
    ["SA-W03-namah-guided-copy", "guided-copy"],
    ["SA-W03-namah-delayed-copy", "delayed-copy"],
    ["SA-W03-namah-dictation", "dictation-transcription"],
    ["SA-W05-vocalic-r-guided-copy", "guided-copy"],
    ["SA-W05-vocalic-r-delayed-copy", "delayed-copy"],
    ["SA-W05-vocalic-r-dictation", "dictation-transcription"],
  ]);
  expect(track.defects).toEqual([]);
  expect(track.levels[0]).toMatchObject({
    level: "pre-A1",
    missingStages: [],
    complete: true,
  });
});

it("extends the sourced single-letter ladder through words, visarga, and vocalic r", () => {
  const ids = [
    "SA-W03-mama-guided-copy",
    "SA-W03-mama-delayed-copy",
    "SA-W03-mama-dictation",
    "SA-W03-mama-nama-guided-copy",
    "SA-W03-mama-nama-delayed-copy",
    "SA-W03-mama-nama-dictation",
    "SA-W03-namah-guided-copy",
    "SA-W03-namah-delayed-copy",
    "SA-W03-namah-dictation",
    "SA-W05-vocalic-r-guided-copy",
    "SA-W05-vocalic-r-delayed-copy",
    "SA-W05-vocalic-r-dictation",
  ];
  const lessons = loadTrackLessons("sanskrit")
    .sort(readingOrder)
    .filter((lesson) => ids.includes(lesson.realization.lessonId));

  expect(lessons.map((lesson) => lesson.realization.lessonId)).toEqual(ids);
  expect(lessons.map((lesson) => Number(lesson.frontmatter["duration.max_seconds"]))).toEqual([
    150, 120, 120, 180, 150, 150, 150, 120, 120, 150, 120, 120,
  ]);
  for (const [start, end] of [[0, 8], [9, 11]]) {
    for (let index = start + 1; index <= end; index += 1) {
      expect(lessons[index]?.frontmatter.prerequisites).toContain(ids[index - 1]);
    }
  }
  expect(lessons[9]?.frontmatter.prerequisites).toContain("SA-S205-vowel-sign-vocalic-r");

  const markdown = lessons.map((lesson) =>
    lesson.blocks.map((block) => block.markdown).join("\n"),
  );
  expect(markdown[0]).toContain("new vocabulary");
  expect(markdown[2]).toContain("no visible Devanagari model and no romanized answer");
  expect(markdown[3]).toContain("न + ◌ा");
  expect(markdown[4]).toContain("Keep the two words apart");
  expect(markdown[5]).toContain("from sound and meaning alone");
  expect(markdown[5]).toContain("connected text");
  expect(markdown[6]).toContain("not new vocabulary");
  expect(markdown[7]).toContain("no visible answer and no");
  expect(markdown[7]).toContain("romanization");
  expect(markdown[8]).toContain("from sound and meaning alone");
  expect(markdown[8]).toContain("Vocalic ṛ");
  expect(markdown[9]).toContain("not a new word or a new shape");
  expect(markdown[10]).toContain("no visible answer and no romanization");
  expect(markdown[11]).toContain("no visible Devanagari model and no romanized answer");
  expect(markdown[11]).toContain("sound and function alone");
  expect(markdown[11]).toContain("Conjuncts, connected text");
});
