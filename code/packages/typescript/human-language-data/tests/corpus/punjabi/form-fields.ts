import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { measureContinuity } from "../../../src/continuity.js";
import {
  DOC_SHARD_PLANS,
  defaultRepoRoot,
  unshardDocContents,
} from "../../../src/doc-shard-cli.js";
import { defaultCurriculumRoot, loadTrackLessons } from "../../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("builds the Punjabi phone field from introduced pieces to independent Gurmukhi writing", () => {
  const chapter = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .filter((lesson) => lesson.frontmatter.chapter === "30");

  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W08-pha",
    "PA-W08-pairin-bindi",
    "PA-W08-hora",
    "PA-W08-phone-label",
    "PA-W08-digit-zero",
    "PA-W08-phone-a",
    "PA-W08-phone-b",
    "PA-W08-digit-recognition",
    "PA-W08-phone-select",
    "PA-W08-phone-supported",
    "PA-W08-phone-grouping",
    "PA-W08-phone-delayed",
    "PA-W08-phone-dictation",
    "PA-W08-phone-repair",
    "PA-W08-phone-no-model",
  ]);
  expect(chapter.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 180)).toBe(true);
  expect(chapter.slice(0, 7).map((lesson) => lesson.frontmatter["introduces.knowledge"])).toEqual([
    ["PA-SCRIPT-PHA-01"],
    ["PA-SCRIPT-PAIRIN-BINDI-01"],
    ["PA-SCRIPT-HORA-01"],
    ["PA-FORM-LABEL-PHONE-01"],
    ["PA-SCRIPT-DIGIT-ZERO-01"],
    ["PA-FORM-PHONE-A-01", "PA-FORM-PHONE-DIGIT-ORDER-01"],
    ["PA-FORM-PHONE-B-01"],
  ]);

  const byId = new Map(chapter.map((lesson) => [lesson.realization.lessonId, lesson]));
  expect(byId.get("PA-W08-phone-supported")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["guided-copy"]);
  expect(byId.get("PA-W08-phone-delayed")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["delayed-copy"]);
  expect(byId.get("PA-W08-phone-dictation")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["dictation-transcription"]);

  const independent = byId.get("PA-W08-phone-no-model")!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean)).toEqual([
    "controlled-composition",
    "controlled-composition",
  ]);
  expect(independent.body).toContain("There is no value bank, support-language label,\nLatin-digit version, or copyable Gurmukhi answer below.");
  expect(independent.body).toContain("> ਖ — **ਫ਼ੋਨ: __________**");
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity?.prompt).not.toContain("੦੨੫ ੧੨੫");
  expect(activity?.prompt).not.toMatch(/[0-9]/);
  expect(activity?.answer).toBe("੦੨੫ ੧੨੫");
});

it("builds the first Punjabi A1 form field without a copyable independent answer", () => {
  const chapter = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .filter((lesson) => lesson.frontmatter.chapter === "15");

  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W02-a",
    "PA-W02-aman",
    "PA-W02-manan",
    "PA-W02-name-label",
    "PA-W02-name-select",
    "PA-W02-name-supported",
    "PA-W02-name-delayed",
    "PA-W02-name-no-model",
  ]);
  expect(chapter.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 180)).toBe(true);

  const supported = chapter.find((lesson) => lesson.realization.lessonId === "PA-W02-name-supported")!;
  expect(supported.body).toContain("This is supported entry, not independent writing evidence.");
  expect(supported.blocks.some((block) => block.writingStage !== undefined)).toBe(false);

  const independent = chapter.at(-1)!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean)).toEqual([
    "controlled-composition",
  ]);
  expect(independent.body).toContain("There is no value bank, support-language name, or romanized answer below.");
  expect(independent.body).toContain("> A — **ਨਾਂ: __________**");
  expect(independent.body).not.toContain("A ਅਮਨ");
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity?.prompt).not.toContain("Aman");
  expect(activity?.prompt).not.toContain("ਅਮਨ");
  expect(activity?.answer).toBe("ਅਮਨ");
});

it("builds the Punjabi A1 language field one script piece at a time", () => {
  const ordered = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence));
  const runway = ordered.filter((lesson) => lesson.frontmatter.chapter === "16");
  const entry = ordered.filter((lesson) => lesson.frontmatter.chapter === "17");

  expect(runway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W03-bha",
    "PA-W03-sha",
    "PA-W03-language-label",
    "PA-W03-pa",
    "PA-W03-tippi",
    "PA-W03-ja",
    "PA-W03-ba",
    "PA-W03-punjabi",
    "PA-W03-sihari",
    "PA-W03-da",
    "PA-W03-hindi",
  ]);
  expect(entry.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W03-language-select",
    "PA-W03-language-supported",
    "PA-W03-language-delayed",
    "PA-W03-language-no-model",
  ]);
  expect([...runway, ...entry].every(
    (lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 180,
  )).toBe(true);

  const supported = entry.find(
    (lesson) => lesson.realization.lessonId === "PA-W03-language-supported",
  )!;
  expect(supported.body).toContain("This is supported entry, not independent writing evidence.");
  expect(supported.blocks.some((block) => block.writingStage !== undefined)).toBe(false);

  const independent = entry.at(-1)!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean)).toEqual([
    "controlled-composition",
  ]);
  expect(independent.body).toContain(
    "There is no value bank, support-language label, or\nromanized answer below.",
  );
  expect(independent.body).toContain("> A — **ਭਾਸ਼ਾ: __________**");
  expect(independent.body).not.toContain("A — **ਪੰਜਾਬੀ**");
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity?.prompt).not.toContain("Punjabi");
  expect(activity?.prompt).not.toContain("ਪੰਜਾਬੀ");
  expect(activity?.answer).toBe("ਪੰਜਾਬੀ");
});

it("builds the Punjabi A1 residence field one script piece at a time", () => {
  const ordered = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence));
  const runway = ordered.filter((lesson) => lesson.frontmatter.chapter === "18");
  const entry = ordered.filter((lesson) => lesson.frontmatter.chapter === "19");

  expect(runway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W04-ra",
    "PA-W04-independent-i",
    "PA-W04-residence-label",
    "PA-W04-dda",
    "PA-W04-village",
    "PA-W04-city",
    "PA-R18-wellbeing-r4",
  ]);
  expect(entry.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W04-residence-select",
    "PA-W04-residence-supported",
    "PA-W04-residence-spacing",
    "PA-W04-residence-delayed",
    "PA-W04-residence-repair",
    "PA-W04-residence-no-model",
  ]);
  expect([...runway, ...entry].every(
    (lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 180,
  )).toBe(true);

  const supported = entry.find(
    (lesson) => lesson.realization.lessonId === "PA-W04-residence-supported",
  )!;
  expect(supported.body).toContain("This is supported entry, not independent writing evidence.");
  expect(supported.blocks.some((block) => block.writingStage !== undefined)).toBe(false);

  const independent = entry.at(-1)!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean)).toEqual([
    "controlled-composition",
  ]);
  expect(independent.body).toContain(
    "There is no value bank, support-language label, or romanized answer below.",
  );
  expect(independent.body).toContain("> A — **ਰਿਹਾਇਸ਼: __________**");
  expect(independent.body).not.toContain("A — **ਪਿੰਡ**");
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity?.prompt).not.toContain("village");
  expect(activity?.prompt).not.toContain("ਪਿੰਡ");
  expect(activity?.answer).toBe("ਪਿੰਡ");
});

it("builds the Punjabi A1 work field one script piece and one demand at a time", () => {
  const ordered = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence));
  const runway = ordered.filter((lesson) => lesson.frontmatter.chapter === "20");
  const entry = ordered.filter((lesson) => lesson.frontmatter.chapter === "21");

  expect(runway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W05-ka",
    "PA-W05-work-label",
    "PA-W05-kha",
    "PA-W05-farming",
    "PA-W05-au-matra",
    "PA-W05-job",
  ]);
  expect(entry.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W05-work-select",
    "PA-W05-work-supported",
    "PA-W05-work-spelling",
    "PA-W05-work-spacing",
    "PA-W05-work-agreement",
    "PA-W05-work-delayed",
    "PA-W05-work-repair",
    "PA-W05-work-no-model",
  ]);
  expect([...runway, ...entry].every(
    (lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 180,
  )).toBe(true);

  const supported = entry.find(
    (lesson) => lesson.realization.lessonId === "PA-W05-work-supported",
  )!;
  expect(supported.body).toContain("This is supported entry, not independent writing evidence.");
  expect(supported.blocks.some((block) => block.writingStage !== undefined)).toBe(false);

  const focusedLessons = [
    "PA-W05-work-spelling",
    "PA-W05-work-spacing",
    "PA-W05-work-agreement",
    "PA-W05-work-repair",
  ].map((id) => entry.find((lesson) => lesson.realization.lessonId === id)!);
  expect(focusedLessons.map((lesson) => lesson.frontmatter["introduces.knowledge"])).toEqual([
    ["PA-FORM-WORK-SPELLING-CHECK-01"],
    ["PA-FORM-WORK-SPACING-01"],
    ["PA-FORM-WORK-AGREEMENT-01"],
    ["PA-FORM-WORK-REPAIR-01"],
  ]);
  expect(focusedLessons[2]!.body).toContain(
    "This checks field-value meaning, not grammatical gender.",
  );

  const independent = entry.at(-1)!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean)).toEqual([
    "controlled-composition",
  ]);
  expect(independent.body).toContain(
    "There is no value bank, support-language label, or romanized answer below.",
  );
  expect(independent.body).toContain("> A — **ਕੰਮ: __________**");
  expect(independent.body).not.toContain("A — **ਖੇਤੀ**");
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity?.prompt).not.toContain("farming");
  expect(activity?.prompt).not.toContain("ਖੇਤੀ");
  expect(activity?.answer).toBe("ਖੇਤੀ");
});

