import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../src/activity.js";
import { measureContinuity } from "../../src/continuity.js";
import { defaultCurriculumRoot, loadTrackLessons } from "../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";

it("pins Punjabi continuity", () => expectLanguageContinuity("punjabi"));
it("pins Punjabi modality", () => expectLanguageModality("punjabi"));
it("keeps the Punjabi changelog free of literal patch markup", () => {
  const changelog = readFileSync(
    join(defaultCurriculumRoot(), "punjabi", "CHANGELOG.md"),
    "utf8",
  );
  expect(changelog).not.toMatch(/^@@$/m);
  expect(changelog).not.toMatch(/^\+##/m);
  expect(changelog.indexOf("Punjabi A1 phone-field writing ladder"))
    .toBeLessThan(changelog.indexOf("Punjabi A1 age-field writing ladder"));
});
// 173 -> 196 with the pre-A1 courtesy-and-parting tranche (Chapters 31-36): 23 new
// lessons, of which 14 are content lessons carrying a headword and 9 are single-glyph
// script sessions the budget counter does not measure.
// 196 -> 214 with the pre-A1 script runway inserted into Chapters 2-13 (#13068): 18
// recognition sessions, each teaching at most three Gurmukhi pieces BEFORE a lesson
// asks the reader to decode them. They are what took Punjabi's script-closure debt
// from 40 violating lessons to 0 and its never-taught glyph count from 8 to 0.
// 214 -> 216 when Chapters 4 and 5 stopped being hand-written .tex and became
// generated from their lessons: the migration to schema v2 split the two Chapter 5
// sessions that had packed several headwords each, so panjābī and karnā now get a
// lesson apiece. The same migration is why idioms and culture claims move: the
// farewell lessons finally DECLARE the units they were always teaching
// (phir milāṁge and rabb rākhā as idioms, rabb rākhā's Arabic-plus-Sanskrit blend
// as a culture claim), which a schema-v1 lesson had no field to say.
it("pins Punjabi lesson-content budgets", () =>
  expectLanguageLessonBudgets("punjabi", {
    // 226 -> 261. Seven chapters of five, one new item per lesson, closing the
    // Jorr column the A1 inventory measured at 0 of 11. One culture claim: the
    // Sanskritic / Perso-Arabic pair rule, which the inventory said one lesson
    // would close and which this book has owed since it taught dhanvaad and
    // shukriya side by side in its first chapter.
    lessons: 261,
    idioms: 6,
    senses: 3,
    cultureClaims: 9,
    unitPrefix: "PA",
  }));

it("keeps Punjabi's 261-row session map aligned with canonical order", () => {
  const ordered = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const markdown = readFileSync(
    join(defaultCurriculumRoot(), "punjabi", "session-map.md"),
    "utf8",
  );
  const rows = [...markdown.matchAll(/^\| (\d+) \| (\d+) \| ([^|]+?) \| (.+) \|$/gm)].map(
    (match) => ({
      session: Number(match[1]),
      chapter: match[2],
      lessonId: match[3]!.trim(),
    }),
  );
  expect(rows).toHaveLength(261);
  expect(rows.map((row) => row.session)).toEqual(Array.from({ length: 261 }, (_, index) => index + 1));
  expect(rows.map((row) => row.lessonId)).toEqual(
    ordered.map((lesson) => lesson.realization.lessonId),
  );
  expect(rows.map((row) => row.chapter)).toEqual(
    ordered.map((lesson) => String(lesson.frontmatter.chapter)),
  );
});

it("pins Punjabi's meaning-first response and writing runway", () => {
  const opening = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .slice(0, 10);
  expect(opening.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-C01-sat-sri-akal",
    "PA-C01-namaste",
    "PA-C01-han-nahin",
    "PA-W01-ha",
    "PA-W01-aa-matra",
    "PA-W01-bindi",
    "PA-W01-haan-assemble",
    "PA-W01-haan-guided-copy",
    "PA-W01-haan-delayed-copy",
    "PA-W01-haan-dictation",
  ]);
  expect(opening.every((lesson) => lesson.frontmatter.chapter === "1")).toBe(true);

  const meaningFirst = opening[2]!;
  expect(meaningFirst.realization.gloss).toContain("yes / no");
  expect(meaningFirst.frontmatter.skills).toEqual(["listening", "speaking"]);
  expect(meaningFirst.body).toContain("The printed forms are labels, not a reading test.");
  expect(meaningFirst.body).toContain("The next three tiny sessions will teach one\npiece at a time");
});

it("pins Punjabi's complete pre-A1 writing runway", () => {
  const punjabi = languageWritingStages("punjabi");
  expect(punjabi.defects).toEqual([]);
  expect(punjabi.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  expect(punjabi.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "controlled-composition",
    "controlled-composition",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    // Chapters 31-36. Nine single-letter observe/trace sessions, interleaved with the
    // guided and delayed copies that spend each new letter on a word already known by
    // ear. The pattern alternates on purpose: no two assembly steps run back to back.
    "observe-trace",
    "guided-copy",
    "observe-trace",
    "observe-trace",
    "guided-copy",
    "observe-trace",
    "guided-copy",
    "observe-trace",
    "guided-copy",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "delayed-copy",
    // 51 -> 59. The joining tranche adds eight: PA-W10-thatha is the only new
    // LETTER in seven chapters and contributes an observe-trace and a guided
    // copy, and six word-writing lessons -- nahin, ate, par, ki, je, oh --
    // contribute one guided copy each. Every one of those six spends ZERO new
    // signs.
    "guided-copy",
    // 51 -> 59. The joining tranche adds eight: PA-W10-thatha is the only new
    // LETTER in seven chapters and contributes an observe-trace and a guided
    // copy, and six word-writing lessons -- nahin, ate, par, ki, je, oh --
    // contribute one guided copy each. Every one of those six spends ZERO new
    // signs.
    "guided-copy",
    // 51 -> 59. The joining tranche adds eight: PA-W10-thatha is the only new
    // LETTER in seven chapters and contributes an observe-trace and a guided
    // copy, and six word-writing lessons -- nahin, ate, par, ki, je, oh --
    // contribute one guided copy each. Every one of those six spends ZERO new
    // signs.
    "guided-copy",
    // 51 -> 59. The joining tranche adds eight: PA-W10-thatha is the only new
    // LETTER in seven chapters and contributes an observe-trace and a guided
    // copy, and six word-writing lessons -- nahin, ate, par, ki, je, oh --
    // contribute one guided copy each. Every one of those six spends ZERO new
    // signs.
    "guided-copy",
    // 51 -> 59. The joining tranche adds eight: PA-W10-thatha is the only new
    // LETTER in seven chapters and contributes an observe-trace and a guided
    // copy, and six word-writing lessons -- nahin, ate, par, ki, je, oh --
    // contribute one guided copy each. Every one of those six spends ZERO new
    // signs.
    "guided-copy",
    // 51 -> 59. The joining tranche adds eight: PA-W10-thatha is the only new
    // LETTER in seven chapters and contributes an observe-trace and a guided
    // copy, and six word-writing lessons -- nahin, ate, par, ki, je, oh --
    // contribute one guided copy each. Every one of those six spends ZERO new
    // signs.
    "guided-copy",
    "observe-trace",
    "guided-copy",
  ]);
});

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

it("integrates Punjabi language, residence, and work with separate repair passes", () => {
  const ordered = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence));
  const preparation = ordered.filter((lesson) => lesson.frontmatter.chapter === "22");
  const checkpoint = ordered.filter((lesson) => lesson.frontmatter.chapter === "23");

  expect(preparation.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W06-three-field-labels",
    "PA-W06-three-field-cues",
    "PA-W06-two-field-supported",
    "PA-W06-three-field-supported",
  ]);
  expect(checkpoint.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W06-selection-repair",
    "PA-W06-spelling-repair",
    "PA-W06-spacing-repair",
    "PA-W06-placement-repair",
    "PA-W06-mixed-repair",
    "PA-W06-three-field-no-model",
    "PA-R23-three-no-model-r1",
  ]);
  expect([...preparation, ...checkpoint].every(
    (lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 180,
  )).toBe(true);

  const supported = preparation.at(-1)!;
  expect(supported.body).toContain("This is supported entry, not independent writing evidence.");
  expect(supported.blocks.some((block) => block.writingStage !== undefined)).toBe(false);

  const focused = checkpoint.slice(0, 5);
  expect(focused.map((lesson) => lesson.frontmatter["introduces.knowledge"])).toEqual([
    ["PA-FORM-THREE-SELECTION-REPAIR-01"],
    ["PA-FORM-THREE-SPELLING-REPAIR-01"],
    ["PA-FORM-THREE-SPACING-REPAIR-01"],
    ["PA-FORM-THREE-PLACEMENT-REPAIR-01"],
    ["PA-FORM-THREE-MIXED-REPAIR-01"],
  ]);

  const independent = checkpoint.find(
    (lesson) => lesson.realization.lessonId === "PA-W06-three-field-no-model",
  )!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean)).toEqual([
    "controlled-composition",
  ]);
  expect(independent.body).toContain(
    "There is no value bank, support-language label, romanization, or copyable answer below.",
  );
  expect(independent.body).toContain("> A — **ਭਾਸ਼ਾ: __________**");
  expect(independent.body).toContain("> B — **ਰਿਹਾਇਸ਼: __________**");
  expect(independent.body).toContain("> A — **ਕੰਮ: __________**");
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity?.prompt).not.toContain("ਪੰਜਾਬੀ");
  expect(activity?.prompt).not.toContain("ਸ਼ਹਿਰ");
  expect(activity?.prompt).not.toContain("ਖੇਤੀ");
  expect(activity?.answer).toBe("ਭਾਸ਼ਾ: ਪੰਜਾਬੀ\nਰਿਹਾਇਸ਼: ਸ਼ਹਿਰ\nਕੰਮ: ਖੇਤੀ");
});

it("migrates Punjabi Chapter 2 without inventing Gurmukhi writing credit", () => {
  const chapter = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .filter((lesson) => lesson.frontmatter.chapter === "2");

  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-C02-naam",
    "PA-C02-mera",
    "PA-C02-hai",
    "PA-C02-mera-naam-hai",
    "PA-C02-tu-tusi",
    "PA-C02-ki",
    "PA-S02-mamma-rara-lava",
    "PA-C02-tuhada-naam-ki-hai",
    "PA-C02-khushi",
    "PA-S02-sassa-tatta-sihari",
    "PA-C02-practice",
  ]);
  expect(chapter.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
  expect(
    chapter.every(
      (lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 240,
    ),
  ).toBe(true);
  expect(chapter.every((lesson) => lesson.frontmatter.skills?.includes("listening"))).toBe(true);
  expect(chapter.every((lesson) => lesson.frontmatter.skills?.includes("speaking"))).toBe(true);
  // The chapter used to have NO writing at all. #13068's recognition runway adds
  // some, and the promise the chapter actually made was about INDEPENDENT
  // writing, so the check gets narrower rather than looser: the only lessons
  // here that touch the hand are the `delivery: script` runway sessions, every
  // one of them keeps the model visible, and no content lesson claims writing.
  for (const lesson of chapter) {
    if (!lesson.frontmatter.skills?.includes("writing")) continue;
    expect(lesson.frontmatter.delivery).toBe("script");
    expect(lesson.realization.lessonId.startsWith("PA-S")).toBe(true);
    expect(lesson.body).toContain("observe-and-trace with the model visible");
    expect(lesson.body).toContain("Nothing in these");
    // no independent-writing stage is claimed anywhere in chapters 2-13
    expect(lesson.body).not.toContain("hl-writing-stage");
  }

  const payoff = chapter.at(-1)!;
  expect(payoff.body).toContain("Independent Gurmukhi reading and writing are **not scored here**");
  expect(payoff.body).toContain("A romanized answer never counts as Gurmukhi writing");
});

it("migrates Punjabi Chapter 3 as a gentle oral wellbeing exchange", () => {
  const chapter = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .filter((lesson) => lesson.frontmatter.chapter === "3");

  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-S03-nanna-bihari-dulava",
    "PA-C03-kivein",
    "PA-C03-tusi-kivein-ho",
    "PA-R03-wellbeing-r1",
    "PA-C03-main",
    "PA-S03-retroflex-row",
    "PA-C03-thik",
    "PA-C03-koi-gall-nahin",
    "PA-C03-practice",
  ]);
  expect(chapter.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
  expect(
    chapter.every(
      (lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 240,
    ),
  ).toBe(true);
  expect(chapter.every((lesson) => lesson.frontmatter.skills?.includes("listening"))).toBe(true);
  expect(chapter.every((lesson) => lesson.frontmatter.skills?.includes("speaking"))).toBe(true);
  // The chapter used to have NO writing at all. #13068's recognition runway adds
  // some, and the promise the chapter actually made was about INDEPENDENT
  // writing, so the check gets narrower rather than looser: the only lessons
  // here that touch the hand are the `delivery: script` runway sessions, every
  // one of them keeps the model visible, and no content lesson claims writing.
  for (const lesson of chapter) {
    if (!lesson.frontmatter.skills?.includes("writing")) continue;
    expect(lesson.frontmatter.delivery).toBe("script");
    expect(lesson.realization.lessonId.startsWith("PA-S")).toBe(true);
    expect(lesson.body).toContain("observe-and-trace with the model visible");
    expect(lesson.body).toContain("Nothing in these");
    // no independent-writing stage is claimed anywhere in chapters 2-13
    expect(lesson.body).not.toContain("hl-writing-stage");
  }

  const payoff = chapter.at(-1)!;
  expect(payoff.body).toContain("return the question");
  expect(payoff.body).toContain("Independent Gurmukhi reading and writing are **not scored here**");
  expect(payoff.body).toContain("A romanized answer never counts as Gurmukhi writing");
});

it("closes Chapter 3's oral R1-R4 windows without inventing script credit", () => {
  const ordered = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const checkpointIds = [
    "PA-R03-wellbeing-r1",
    "PA-R04-wellbeing-r2",
    "PA-R07-wellbeing-r3",
    "PA-R18-wellbeing-r4",
  ];
  const checkpoints = checkpointIds.map((id) =>
    ordered.find((lesson) => lesson.realization.lessonId === id)!,
  );
  expect(checkpoints.map((lesson) => ordered.indexOf(lesson))).toEqual([27, 36, 58, 127]);
  expect(checkpoints.map((lesson) => lesson.frontmatter["introduces.knowledge"])).toEqual([
    [],
    [],
    [],
    [],
  ]);
  expect(checkpoints.every((lesson) => lesson.frontmatter.skills?.includes("listening"))).toBe(true);
  expect(checkpoints.every((lesson) => lesson.frontmatter.skills?.includes("speaking"))).toBe(true);
  expect(checkpoints.every((lesson) => !lesson.frontmatter.skills?.includes("reading"))).toBe(true);
  expect(checkpoints.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(checkpoints.flatMap((lesson) => compileLessonActivities(lesson.blocks))).toHaveLength(11);
  // Chapter 4 used to be hand-written .tex with the R2 checkpoint spliced in behind a
  // `canonical-insertion` comment -- a chapter no lesson-level gate could see. It is now
  // GENERATED from its lessons, so the checkpoint is a canonical lesson of the chapter
  // rather than a comment promising one. That is the stronger claim, and it is what this
  // assertion now makes: the header must name the lesson, and its prose must be present.
  const generatedChapter4 = readFileSync(
    join(defaultCurriculumRoot(), "punjabi", "book", "chapters", "ch04-farewells.tex"),
    "utf8",
  );
  expect(generatedChapter4.startsWith("% GENERATED FILE.")).toBe(true);
  expect(generatedChapter4).not.toContain("canonical-insertion:");
  expect(generatedChapter4).toContain("% canonical-lessons: ");
  expect(generatedChapter4).toContain("PA-R04-wellbeing-r2");
  expect(generatedChapter4).toContain("label{lesson:PA-R04-wellbeing-r2}");
  expect(generatedChapter4).toContain("Romanization records speech only");

  const chapter3Atoms = new Set([
    "PA-GRAMMAR-TUSI-HO-03",
    "PA-ETYMON-QUESTION-K-03",
    "PA-ETYMON-FIRST-PERSON-M-03",
    "PA-ETYMON-NEGATIVE-NE-03",
    "PA-GRAMMAR-MAIN-HAAN-03",
    "PA-LEX-MAIN-03",
    "PA-LEX-THIK-03",
    "PA-PHRASE-HOW-ARE-YOU-03",
    "PA-PHRASE-I-AM-FINE-03",
    "PA-PHRASE-NO-PROBLEM-03",
    "PA-LEX-KIVEIN-03",
  ]);
  const report = measureContinuity(ordered);
  expect(report.reinforcement.filter((defect) => chapter3Atoms.has(defect.atom))).toEqual([]);
  // {45, 94, 141, 74} -> {54, 112, 157, 71}. Chapters 31-36 introduce 45 new atoms, and
  // the ones taught in the closing sessions have no room left after them for an R1 or R2
  // return, which is where the R1/R2/R3 growth comes from. R4 FALLS, from 74 to 71: the
  // new lessons retrieve mainu, madad and the wellbeing answers at long distance, which
  // is the window the earlier tranches were least able to reach.
  // #13068 inserted an 18-lesson recognition runway into Chapters 2-13 and gave
  // it a review layer: each runway lesson rehearses its three-lesson R1
  // neighbourhood, each early content lesson declares the letters its page
  // shows, and each later FORMATION lesson declares the recognition atom for the
  // same glyph. R1, R2 and R3 all fall BELOW their pre-runway values as a
  // result. R4 rises: 40 recognition atoms entered the corpus and 18 of them
  // still have no lesson 80-250 sessions later that puts their glyph back on the
  // page. That residue is named in BACKLOG.d as the next tranche's work.
  // Chapters 4 and 5 stopped being hand-written .tex and became generated from their
  // lessons. Migrating those ten lessons to schema v2 (and splitting two of them)
  // declared 25 knowledge atoms the corpus had been teaching in prose and counting
  // nowhere. Every window they do not close is now VISIBLE, which is why these totals
  // rise rather than fall: the numbers moved because the measurement reaches further,
  // not because reinforcement got worse. The serviced-debt assertions above still hold
  // exactly. The residue -- Chapter 4 and 5 atoms with no later lesson putting them
  // back in front of the reader -- is named in BACKLOG.d as the next tranche's work.
  expect(report.summary.missedByWindow).toEqual({ R1: 54, R2: 127, R3: 215, R4: 111 });
});

it("services Punjabi's three-field R4 debt without moving the boundary forward", () => {
  const ordered = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const lowerWindowBridgeIds = new Set([
    "PA-R23-three-no-model-r1",
    "PA-R26-three-repair-r2",
    "PA-R26-work-build-r3",
    "PA-R26-work-control-r3",
    "PA-R27-ear-mouth-r4",
    "PA-R27-nose-heart-r4",
    "PA-R28-form-supported-r3",
    "PA-R28-head-na-r4",
  ]);
  const lastR4Bridge = ordered.findIndex(
    (lesson) => lesson.realization.lessonId === "PA-R25-kin-eye-r4",
  );
  const orderedAtR4 = ordered.slice(0, lastR4Bridge + 1).filter(
    (lesson) => !lowerWindowBridgeIds.has(lesson.realization.lessonId),
  );
  const bridgeIds = [
    "PA-R24-know-think-r4",
    "PA-R24-understand-read-r4",
    "PA-R24-write-take-ask-r4",
    "PA-R24-help-like-r4",
    "PA-R25-drink-request-r4",
    "PA-R25-milk-bread-r4",
    "PA-R25-friend-family-r4",
    "PA-R25-kin-eye-r4",
  ];
  const bridge = bridgeIds.map((id) =>
    orderedAtR4.find((lesson) => lesson.realization.lessonId === id)!,
  );
  expect(bridge.map((lesson) => orderedAtR4.indexOf(lesson))).toEqual([158, 159, 160, 161, 162, 163, 164, 165]);
  expect(bridge.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 220)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("listening"))).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("speaking"))).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("reading"))).toBe(true);
  expect(bridge.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(bridge.every((lesson) => !lesson.body.includes("hl-writing-stage"))).toBe(true);
  expect(bridge.flatMap((lesson) => compileLessonActivities(lesson.blocks))).toHaveLength(17);

  const exposedByThreeFieldIntegration = [
    "PA-ETYMON-PASAND-SHINE",
    "PA-CONTRAST-JANA-JANNA",
    "PA-ETYMON-JANNA-KNOW",
    "PA-ETYMON-LAINA-LABH",
    "PA-ETYMON-LIKH-SCRATCH",
    "PA-ETYMON-MADAD-ARABIC",
    "PA-ETYMON-PAANI-DRINK",
    "PA-ETYMON-PARHNA-PATH",
    "PA-ETYMON-PUCHHNA-PRACH",
    "PA-ETYMON-SAMAJH-BUDH",
    "PA-ETYMON-SOCHNA-SHUC",
    "PA-GRAMMAR-DATIVE-LIKING",
    "PA-GRAMMAR-NOUN-PLUS-KARNA",
    "PA-LEX-JANNA",
    "PA-LEX-LAINA",
    "PA-LEX-LIKHNA",
    "PA-LEX-PAANI",
    "PA-LEX-PASAND",
    "PA-LEX-PUCHHNA",
    "PA-LEX-SAMAJHNA",
    "PA-LEX-SOCHNA",
    "PA-PHRASE-KIRPA-KARKE",
    "PA-SCRIPT-SUBJOINED-HA",
    "PA-LEX-MADAD-KARNA",
    "PA-LEX-PARHNA",
    "PA-SOUND-TONE-FALLING",
  ];
  expect(exposedByThreeFieldIntegration).toHaveLength(26);

  const movingBoundaryAtoms = [
    "PA-LEX-CHA",
    "PA-ETYMON-CHA-CHINESE",
    "PA-SOUND-TONE-HIGH-LEVEL",
    "PA-LEX-DUDH",
    "PA-ETYMON-DUDH-DUGDHA",
    "PA-LEX-ROTI",
    "PA-ETYMON-ROTI-UNKNOWN",
    "PA-LEX-DOST",
    "PA-ETYMON-DOST-CHOOSE",
    "PA-LEX-PARIVAR",
    "PA-ETYMON-PARIVAR-SURROUND",
    "PA-LEX-BHARA",
    "PA-ETYMON-BHARA-BROTHER",
    "PA-SOUND-TONE-LOW",
    "PA-LEX-BHAIN",
    "PA-ETYMON-BHAIN-BHAGA",
    "PA-LEX-AKKH",
    "PA-ETYMON-AKKH-EYE",
  ];
  expect(movingBoundaryAtoms).toHaveLength(18);

  const practised = new Set(
    bridge.flatMap((lesson) => lesson.frontmatter["practises.knowledge"] ?? []),
  );
  expect(
    [...exposedByThreeFieldIntegration, ...movingBoundaryAtoms].every((atom) => practised.has(atom)),
  ).toBe(true);

  const report = measureContinuity(orderedAtR4);
  const serviced = new Set([...exposedByThreeFieldIntegration, ...movingBoundaryAtoms]);
  expect(
    report.reinforcement.filter(
      (defect) => serviced.has(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);
  // R4 residue from the #13068 recognition runway; see the note above.
  // Chapters 4 and 5 stopped being hand-written .tex and became generated from their
  // lessons. Migrating those ten lessons to schema v2 (and splitting two of them)
  // declared 25 knowledge atoms the corpus had been teaching in prose and counting
  // nowhere. Every window they do not close is now VISIBLE, which is why these totals
  // rise rather than fall: the numbers moved because the measurement reaches further,
  // not because reinforcement got worse. The serviced-debt assertions above still hold
  // exactly. The residue -- Chapter 4 and 5 atoms with no later lesson putting them
  // back in front of the reader -- is named in BACKLOG.d as the next tranche's work.
  expect(report.summary.missedByWindow.R4).toBe(95);
});

it("services the exact Punjabi R1-R3 debt exposed by the R4 bridge", () => {
  const ordered = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bodyR4BridgeIds = new Set([
    "PA-R27-ear-mouth-r4",
    "PA-R27-nose-heart-r4",
    "PA-R28-form-supported-r3",
    "PA-R28-head-na-r4",
  ]);
  const lastLowerWindowBridge = ordered.findIndex(
    (lesson) => lesson.realization.lessonId === "PA-R26-work-control-r3",
  );
  const orderedBeforeBodyR4 = ordered.slice(0, lastLowerWindowBridge + 1).filter(
    (lesson) => !bodyR4BridgeIds.has(lesson.realization.lessonId),
  );
  const bridgeIds = [
    "PA-R23-three-no-model-r1",
    "PA-R26-three-repair-r2",
    "PA-R26-work-build-r3",
    "PA-R26-work-control-r3",
  ];
  const bridge = bridgeIds.map((id) =>
    orderedBeforeBodyR4.find((lesson) => lesson.realization.lessonId === id)!,
  );
  expect(bridge.map((lesson) => orderedBeforeBodyR4.indexOf(lesson))).toEqual([158, 167, 168, 169]);
  expect(bridge.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 220)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("reading"))).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(bridge.every((lesson) => lesson.body.includes("hl-writing-stage: controlled-composition"))).toBe(true);
  expect(bridge.flatMap((lesson) => compileLessonActivities(lesson.blocks))).toHaveLength(4);

  const servicedPairs = new Set([
    "R3|PA-FORM-WORK-DELAYED-ENTRY-01",
    "R3|PA-FORM-WORK-SUPPORTED-ENTRY-01",
    "R1|PA-FORM-THREE-NO-MODEL-01",
    "R2|PA-FORM-THREE-NO-MODEL-01",
    "R3|PA-FORM-WORK-AGREEMENT-01",
    "R3|PA-SCRIPT-AU-MATRA-01",
    "R2|PA-FORM-THREE-MIXED-REPAIR-01",
    "R2|PA-FORM-THREE-PLACEMENT-REPAIR-01",
    "R2|PA-FORM-THREE-SPACING-REPAIR-01",
    "R2|PA-FORM-THREE-SPELLING-REPAIR-01",
    "R3|PA-FORM-WORK-CUE-MAP-01",
    "R3|PA-FORM-WORK-JOB-01",
    "R3|PA-FORM-WORK-SPACING-01",
    "R3|PA-FORM-WORK-SPELLING-CHECK-01",
    "R3|PA-FORM-WORK-REPAIR-01",
    "R3|PA-FORM-WORK-NO-MODEL-01",
    "R3|PA-FORM-THREE-LABEL-ORDER-01",
    "R3|PA-FORM-THREE-CUE-SELECTION-01",
  ]);
  const report = measureContinuity(orderedBeforeBodyR4);
  const stillMissing = report.reinforcement.flatMap((defect) =>
    defect.missed
      .filter((window) => servicedPairs.has(`${window}|${defect.atom}`))
      .map((window) => `${window}|${defect.atom}`),
  );
  expect(stillMissing).toEqual([]);
  // Chapters 4 and 5 stopped being hand-written .tex and became generated from their
  // lessons. Migrating those ten lessons to schema v2 (and splitting two of them)
  // declared 25 knowledge atoms the corpus had been teaching in prose and counting
  // nowhere. Every window they do not close is now VISIBLE, which is why these totals
  // rise rather than fall: the numbers moved because the measurement reaches further,
  // not because reinforcement got worse. The serviced-debt assertions above still hold
  // exactly. The residue -- Chapter 4 and 5 atoms with no later lesson putting them
  // back in front of the reader -- is named in BACKLOG.d as the next tranche's work.
  expect(report.summary.missedByWindow).toEqual({ R1: 37, R2: 88, R3: 148, R4: 102 });

  const bodyBoundaryAtoms = new Set([
    "PA-LEX-KANN",
    "PA-CONTRAST-KANN-KARNA",
    "PA-LEX-MUNH",
    "PA-ETYMON-MUNH-MUKHA",
    "PA-LEX-NAKK",
    "PA-ETYMON-NAKK-NOSE",
    "PA-LEX-DIL",
    "PA-ETYMON-DIL-HEART",
  ]);
  expect(
    report.reinforcement
      .filter((defect) => defect.missed.includes("R4") && bodyBoundaryAtoms.has(defect.atom))
      .map((defect) => defect.atom)
      .sort(),
  ).toEqual([...bodyBoundaryAtoms].sort());
});

it("services the exact Punjabi body-word R4 debt exposed by the R1-R3 bridge", () => {
  const laterBridgeIds = new Set([
    "PA-R28-form-supported-r3",
    "PA-R28-head-na-r4",
  ]);
  const allLessons = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence));
  const lastBodyR4Bridge = allLessons.findIndex(
    (lesson) => lesson.realization.lessonId === "PA-R27-nose-heart-r4",
  );
  const ordered = allLessons
    .slice(0, lastBodyR4Bridge + 1)
    .filter((lesson) => !laterBridgeIds.has(lesson.realization.lessonId));
  const bridgeIds = [
    "PA-R27-ear-mouth-r4",
    "PA-R27-nose-heart-r4",
  ];
  const bridge = bridgeIds.map((id) =>
    ordered.find((lesson) => lesson.realization.lessonId === id)!,
  );
  expect(bridge.map((lesson) => ordered.indexOf(lesson))).toEqual([170, 171]);
  expect(bridge.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 210)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("listening"))).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("speaking"))).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("reading"))).toBe(true);
  expect(bridge.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(bridge.every((lesson) => !lesson.body.includes("hl-writing-stage"))).toBe(true);
  expect(bridge.every((lesson) => lesson.body.includes("does not award independent Gurmukhi writing evidence"))).toBe(true);
  expect(bridge.flatMap((lesson) => compileLessonActivities(lesson.blocks))).toHaveLength(4);

  const servicedAtoms = new Set([
    "PA-LEX-KANN",
    "PA-CONTRAST-KANN-KARNA",
    "PA-LEX-MUNH",
    "PA-ETYMON-MUNH-MUKHA",
    "PA-LEX-NAKK",
    "PA-ETYMON-NAKK-NOSE",
    "PA-LEX-DIL",
    "PA-ETYMON-DIL-HEART",
  ]);
  const practised = new Set(
    bridge.flatMap((lesson) => lesson.frontmatter["practises.knowledge"] ?? []),
  );
  expect([...servicedAtoms].every((atom) => practised.has(atom))).toBe(true);

  const report = measureContinuity(ordered);
  expect(
    report.reinforcement.filter(
      (defect) => servicedAtoms.has(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);
  // Chapters 4 and 5 stopped being hand-written .tex and became generated from their
  // lessons. Migrating those ten lessons to schema v2 (and splitting two of them)
  // declared 25 knowledge atoms the corpus had been teaching in prose and counting
  // nowhere. Every window they do not close is now VISIBLE, which is why these totals
  // rise rather than fall: the numbers moved because the measurement reaches further,
  // not because reinforcement got worse. The serviced-debt assertions above still hold
  // exactly. The residue -- Chapter 4 and 5 atoms with no later lesson putting them
  // back in front of the reader -- is named in BACKLOG.d as the next tranche's work.
  expect(report.summary.missedByWindow).toEqual({ R1: 37, R2: 88, R3: 150, R4: 97 });

  const before = measureContinuity(
    ordered.filter((lesson) => !bridgeIds.includes(lesson.realization.lessonId)),
  );
  const beforePairs = new Set(
    before.reinforcement.flatMap((defect) =>
      defect.missed.map((window) => `${window}|${defect.atom}`),
    ),
  );
  const afterPairs = new Set(
    report.reinforcement.flatMap((defect) =>
      defect.missed.map((window) => `${window}|${defect.atom}`),
    ),
  );
  expect([...afterPairs].filter((pair) => !beforePairs.has(pair)).sort()).toEqual([
    "R3|PA-FORM-THREE-SUPPORTED-01",
    "R3|PA-FORM-THREE-TWO-LINE-SUPPORTED-01",
    "R4|PA-ETYMON-SIR-HORN",
    "R4|PA-LEX-SIR",
    "R4|PA-SCRIPT-NA-01",
  ]);
});

it("services the exact Punjabi form and head-word debt exposed by the body R4 bridge", () => {
  const allLessons = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bridgeIds = [
    "PA-R28-form-supported-r3",
    "PA-R28-head-na-r4",
  ];
  const lastBridgeIndex = allLessons.findIndex(
    (lesson) => lesson.realization.lessonId === bridgeIds.at(-1),
  );
  const ordered = allLessons.slice(0, lastBridgeIndex + 1);
  const bridge = bridgeIds.map((id) =>
    ordered.find((lesson) => lesson.realization.lessonId === id)!,
  );
  expect(bridge.map((lesson) => ordered.indexOf(lesson))).toEqual([172, 173]);
  expect(bridge.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 210)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);

  const [supportedForm, headAndNa] = bridge;
  expect(supportedForm!.frontmatter.skills).toEqual(["reading", "writing"]);
  expect(supportedForm!.body).toContain("hl-writing-stage: guided-copy");
  expect(supportedForm!.body).toContain("does not award independent Punjabi writing evidence");
  expect(headAndNa!.frontmatter.skills).toEqual(["listening", "speaking", "reading"]);
  expect(headAndNa!.body).not.toContain("hl-writing-stage");
  expect(headAndNa!.body).toContain("does not award independent Gurmukhi writing evidence");
  expect(bridge.flatMap((lesson) => compileLessonActivities(lesson.blocks))).toHaveLength(4);

  // The three recognition atoms are the #13068 spaced review: this lesson puts
  // those glyphs back on the page inside their R3/R4 window.
  expect(supportedForm!.frontmatter["practises.knowledge"]).toEqual([
    "PA-FORM-THREE-TWO-LINE-SUPPORTED-01",
    "PA-FORM-THREE-SUPPORTED-01",
    "PA-SCRIPT-RECOG-BHA-01",
    "PA-SCRIPT-RECOG-BIHARI-01",
    "PA-SCRIPT-RECOG-TA-01",
    "PA-SCRIPT-RECOG-LAVA-01",
  ]);
  expect(headAndNa!.frontmatter["practises.knowledge"]).toEqual([
    "PA-LEX-SIR",
    "PA-ETYMON-SIR-HORN",
    "PA-SCRIPT-NA-01",
    // the sihari met by eye in Chapter 4, back on the page inside its window
    "PA-SCRIPT-RECOG-SIHARI-01",
  ]);

  const servicedPairs = new Set([
    "R3|PA-FORM-THREE-SUPPORTED-01",
    "R3|PA-FORM-THREE-TWO-LINE-SUPPORTED-01",
    "R4|PA-ETYMON-SIR-HORN",
    "R4|PA-LEX-SIR",
    "R4|PA-SCRIPT-NA-01",
  ]);
  const report = measureContinuity(ordered);
  const stillMissing = report.reinforcement.flatMap((defect) =>
    defect.missed
      .filter((window) => servicedPairs.has(`${window}|${defect.atom}`))
      .map((window) => `${window}|${defect.atom}`),
  );
  expect(stillMissing).toEqual([]);
  // Chapters 4 and 5 stopped being hand-written .tex and became generated from their
  // lessons. Migrating those ten lessons to schema v2 (and splitting two of them)
  // declared 25 knowledge atoms the corpus had been teaching in prose and counting
  // nowhere. Every window they do not close is now VISIBLE, which is why these totals
  // rise rather than fall: the numbers moved because the measurement reaches further,
  // not because reinforcement got worse. The serviced-debt assertions above still hold
  // exactly. The residue -- Chapter 4 and 5 atoms with no later lesson putting them
  // back in front of the reader -- is named in BACKLOG.d as the next tranche's work.
  expect(report.summary.missedByWindow).toEqual({ R1: 37, R2: 88, R3: 150, R4: 91 });

  const before = measureContinuity(
    ordered.filter((lesson) => !bridgeIds.includes(lesson.realization.lessonId)),
  );
  const beforePairs = new Set(
    before.reinforcement.flatMap((defect) =>
      defect.missed.map((window) => `${window}|${defect.atom}`),
    ),
  );
  const afterPairs = new Set(
    report.reinforcement.flatMap((defect) =>
      defect.missed.map((window) => `${window}|${defect.atom}`),
    ),
  );
  expect([...afterPairs].filter((pair) => !beforePairs.has(pair)).sort()).toEqual([
    "R3|PA-FORM-THREE-SELECTION-REPAIR-01",
    "R3|PA-FORM-THREE-SPELLING-REPAIR-01",
    "R4|PA-SCRIPT-II-MATRA-01",
    "R4|PA-SCRIPT-MA-01",
  ]);
});
