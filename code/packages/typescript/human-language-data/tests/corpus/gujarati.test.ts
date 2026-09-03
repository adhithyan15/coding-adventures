import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../src/activity.js";
import { measureContinuity, REINFORCEMENT_WINDOWS } from "../../src/continuity.js";
import { defaultCurriculumRoot, loadTrackChapters, loadTrackLessons } from "../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";

it("pins Gujarati continuity", () => expectLanguageContinuity("gujarati"));
it("pins Gujarati modality", () => expectLanguageModality("gujarati"));
it("pins Gujarati lesson-content budgets", () =>
  expectLanguageLessonBudgets("gujarati", {
    // HL-C286: 179 -> 228. Six chapters. Chapter 29 writes the six time words
    // chapter 28 left oral; chapter 30 is the fifth-return slab the previous
    // tranche filed rather than smuggled into a vocabulary chapter; chapters
    // 31-34 each teach five new pre-A1 headwords one lesson at a time, ear
    // first, and write exactly one of them. None declares an idiom, sense, or
    // culture claim, so only the lesson total moves.
    // 228 -> 263, and 15 -> 16 culture claims. Seven chapters, five lessons each,
    // closing the joining column the A1 inventory measured at 0 of 11. The single
    // culture claim is that mataf karo both apologises AND stops a stranger, which
    // is why one phrase closes a courtesy function and a repair strategy at once.
    lessons: 263,
    idioms: 12,
    senses: 6,
    cultureClaims: 16,
    unitPrefix: "GU",
  }));

it("keeps the Gujarati session map aligned with canonical chapter and lesson order", () => {
  const ordered = loadTrackLessons("gujarati").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const ledger = loadTrackChapters().find((track) => track.language === "gujarati");
  expect(ledger).toBeDefined();

  const markdown = readFileSync(
    join(defaultCurriculumRoot(), "gujarati", "session-map.md"),
    "utf8",
  );
  const inventory = markdown
    .split("## Canonical session inventory", 2)[1]
    ?.split("## Current boundary", 1)[0];
  expect(inventory).toBeDefined();

  const rows = [...inventory!.matchAll(
    /^\| (\d+(?:-\d+)?) \| (\d+) \| ([^|]+?) \| (.+) \|$/gm,
  )].map((match) => ({
    range: match[1]!,
    chapter: Number(match[2]),
    title: match[3]!.trim(),
    ids: [...match[4]!.matchAll(/`(GU-[A-Za-z0-9-]+)`/g)].map((id) => id[1]!),
  }));

  expect(rows.map(({ chapter, title }) => ({ chapter, title }))).toEqual(
    ledger!.chapters.map(({ chapter, title }) => ({ chapter, title })),
  );
  expect(rows.flatMap((row) => row.ids)).toEqual(
    ordered.map((lesson) => lesson.realization.lessonId),
  );

  let nextSession = 1;
  for (const row of rows) {
    const [startText, endText = startText] = row.range.split("-");
    const start = Number(startText);
    const end = Number(endText);
    expect(start).toBe(nextSession);
    expect(end - start + 1).toBe(row.ids.length);
    nextSession = end + 1;
  }
  expect(nextSession - 1).toBe(ordered.length);
  expect(new Set(rows.flatMap((row) => row.ids)).size).toBe(ordered.length);
});

it("pins Gujarati's meaning-first opening script spine", () => {
  const ordered = loadTrackLessons("gujarati").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const opening = ordered.slice(0, 11);
  expect(opening.map((lesson) => lesson.realization.lessonId)).toEqual([
    "GU-C01-namaste",
    "GU-W01-ha",
    "GU-W01-aa-matra",
    "GU-W01-aa",
    "GU-W01-na",
    "GU-W01-ma",
    "GU-W01-sa",
    "GU-W01-ta",
    "GU-W01-e-matra",
    "GU-W01-virama",
    "GU-W01-namaste-read",
  ]);
  expect(opening.every((lesson) => lesson.frontmatter.chapter === "1")).toBe(true);

  const meaningFirst = opening[0]!;
  expect(meaningFirst.realization.romanization).toBe("namaste");
  expect(meaningFirst.frontmatter.skills).toEqual(["listening", "speaking"]);
  expect(meaningFirst.body).not.toMatch(/\p{Script=Gujarati}/u);

  const courtesy = ordered.slice(11, 26);
  expect(courtesy.map((lesson) => lesson.realization.lessonId)).toEqual([
    "GU-W02-ra",
    "GU-W02-da",
    "GU-W02-va",
    "GU-W02-ya",
    "GU-W02-bha",
    "GU-W02-dha",
    "GU-W02-vocalic-r",
    "GU-C01-aabhaar",
    "GU-C01-aavjo",
    "GU-C01-haa-naa",
    "GU-W01-haa-guided-copy",
    "GU-W01-haa-delayed-copy",
    "GU-W01-haa-dictation",
    "GU-C01-saarun",
    "GU-C01-practice",
  ]);
  expect(courtesy.every((lesson) => lesson.frontmatter.chapter === "2")).toBe(true);

  const chapterSizes = new Map<string, number>();
  for (const lesson of ordered) {
    const chapter = lesson.frontmatter.chapter;
    chapterSizes.set(chapter, (chapterSizes.get(chapter) ?? 0) + 1);
  }
  // Chapters 35-41 are the joining tranche: seven chapters of exactly five,
  // one new item per lesson with the writing lesson third in every one.
  expect([...chapterSizes.entries()]).toEqual([
    ["1", 11],
    ["2", 15],
    ["3", 10],
    ["4", 5],
    ["5", 5],
    ["6", 5],
    ["7", 5],
    ["8", 10],
    ["9", 6],
    ["10", 5],
    ["11", 5],
    ["12", 2],
    ["13", 6],
    ["14", 4],
    ["15", 4],
    ["16", 4],
    ["17", 5],
    ["18", 4],
    ["19", 6],
    ["20", 6],
    ["21", 6],
    ["22", 5],
    ["23", 1],
    ["24", 8],
    ["25", 7],
    ["26", 7],
    ["27", 8],
    // HL-C271: chapter 28, "The Day and Its Times". Fourteen lessons, thirteen
    // of them ear-first: the eight new headwords are heard and said before any
    // of them is shown, and only two ever reach the page in this chapter.
    ["28", 14],
    // HL-C286. Chapter 29 writes six words already known by ear, so it teaches
    // no new headword at all. Chapter 30 introduces nothing: nine zero-new-atom
    // lessons returning the numbers and the fifteen core verbs at R4. Chapters
    // 31-34 are the vocabulary tranche -- eight lessons each, five of them one
    // new headword apiece, then an oral checkpoint, one word to the page, and
    // an R1 return that also carries a named distant band.
    ["29", 8],
    ["30", 9],
    ["31", 8],
    ["32", 8],
    ["33", 8],
    ["34", 8],
    ["35", 5],
    ["36", 5],
    ["37", 5],
    ["38", 5],
    ["39", 5],
    ["40", 5],
    ["41", 5],
  ]);
});

it("pins Gujarati's complete pre-A1 writing runway", () => {
  const gujarati = languageWritingStages("gujarati");
  expect(gujarati.defects).toEqual([]);
  expect(gujarati.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  expect(gujarati.validEvidence.map((entry) => entry.stage)).toEqual([
    // HL-C286: 158 -> 189 entries. Chapter 29 adds eight (five guided copies of
    // the time words, one delayed copy, the payoff's delayed copy, its R1
    // dictation), chapter 30 nine dictations, and chapters 31-34 two apiece.
    // The distant bands that return already-written words do so by COLD READING
    // rather than a second pen block, because a lesson may carry only one.
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "guided-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "guided-copy",
    "delayed-copy",
    "delayed-copy",
    "delayed-copy",
    "dictation-transcription",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "dictation-transcription",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "delayed-copy",
    "delayed-copy",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "guided-copy",
    "delayed-copy",
    "guided-copy",
    "dictation-transcription",
    "dictation-transcription",
    "delayed-copy",
    "guided-copy",
    "dictation-transcription",
    "dictation-transcription",
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "dictation-transcription",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "dictation-transcription",
    "guided-copy",
    "dictation-transcription",
    "guided-copy",
    "dictation-transcription",
    "guided-copy",
    "dictation-transcription",
    "guided-copy",
    "dictation-transcription",
    // 189 -> 197. The joining tranche adds eight: GU-W08-pha is the only new
    // LETTER in seven chapters and contributes an observe-trace and a guided
    // copy, and the six word-writing lessons -- ane, ke, kemke, jo, te, kyaan --
    // contribute one guided copy each. Every one of those six spends ZERO new
    // signs: the words this book most needed were never a writing problem.
    "observe-trace",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
  ]);
});

it("closes R3 for every Gujarati doorway form after the first name exchange", () => {
  const doorway = new Set([
    "GU-SCRIPT-JA-01",
    "GU-SCRIPT-O-MATRA-01",
    "GU-SCRIPT-ANUSVARA-01",
    "GU-SCRIPT-II-MATRA-01",
    "GU-SCRIPT-U-MATRA-01",
    "GU-SCRIPT-CHHA-01",
    "GU-SCRIPT-KA-01",
    "GU-SCRIPT-NNA-01",
    "GU-SCRIPT-SHA-01",
  ]);
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const checkpointIndex = ordered.findIndex(
    (lesson) => lesson.realization.lessonId === "GU-R04-doorway-nine-r2",
  );
  const checkpoint = ordered[checkpointIndex]!;
  expect(checkpoint.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(
    [...doorway].map((atom) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return checkpointIndex - introducedAt;
    }),
  ).toEqual([38, 37, 36, 35, 34, 33, 32, 31, 30]);

  const stillMissingR3 = measureContinuity(lessons).reinforcement.filter(
    (defect) => doorway.has(defect.atom) && defect.missed.includes("R3"),
  );
  expect(stillMissingR3).toEqual([]);
});

it("pins Gujarati R4 bridge A at positions 111 through 116", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bridgeIds = [
    "GU-R15-u-matra-r4",
    "GU-R15-chha-r4",
    "GU-R15-ka-r4",
    "GU-R15-nna-r4",
    "GU-R15-sha-r4",
    "GU-R15-name-exchange-r3",
  ];
  expect(ordered.slice(111, 117).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(111, 117).every((lesson) =>
      ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactR4 = [
    "GU-SCRIPT-U-MATRA-01",
    "GU-SCRIPT-CHHA-01",
    "GU-SCRIPT-KA-01",
    "GU-SCRIPT-NNA-01",
    "GU-SCRIPT-SHA-01",
  ];
  expect(
    exactR4.map((atom, offset) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 111 + offset - introducedAt;
    }),
  ).toEqual([81, 81, 81, 81, 81]);

  const stillMissingR4 = measureContinuity(ordered.slice(0, 117)).reinforcement.filter(
    (defect) => exactR4.includes(defect.atom) && defect.missed.includes("R4"),
  );
  expect(stillMissingR4).toEqual([]);

  const beforeBridge = measureContinuity(ordered.slice(0, 111));
  const afterBridge = measureContinuity(ordered.slice(0, 117));
  const priorTrackEnd = 110;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = afterBridge.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(201);
  expect(priorWindowMissesAfterBridge).toBe(188);
});

it("pins Gujarati R4 bridge B at positions 117 through 122", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bridgeIds = [
    "GU-R16-naam-r4",
    "GU-R16-maarun-r4",
    "GU-R16-chhe-r4",
    "GU-R16-my-name-is-r4",
    "GU-R16-anand-r4",
    "GU-R16-wellbeing-r3",
  ];
  expect(ordered.slice(117, 123).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(117, 123).every((lesson) =>
      ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactR4 = [
    "GU-CONCEPT-C02-NAAM-01",
    "GU-CONCEPT-C02-MAARUN-01",
    "GU-CONCEPT-C02-CHHE-01",
    "GU-CONCEPT-C02-MAARUNNAAMCHHE-01",
    "GU-CONCEPT-C02-ANAND-01",
  ];
  expect(
    exactR4.map((atom, offset) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 117 + offset - introducedAt;
    }),
  ).toEqual([61, 61, 61, 61, 61]);

  const wellbeingR3 = new Set([
    "GU-CONCEPT-C03-HUN-01",
    "GU-CONCEPT-C03-KEM-01",
    "GU-CONCEPT-C03-MAJAA-01",
    "GU-CONCEPT-C03-PRACTICE-01",
    "GU-CONCEPT-C03-TAMEKEMCHHO-01",
    "GU-CONCEPT-C03-VANDHONAHI-01",
  ]);
  const continuity = measureContinuity(ordered.slice(0, 123));
  expect(
    continuity.reinforcement.filter(
      (defect) => exactR4.includes(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);
  expect(
    continuity.reinforcement.filter(
      (defect) => wellbeingR3.has(defect.atom) && defect.missed.includes("R3"),
    ),
  ).toEqual([]);

  const beforeBridge = measureContinuity(ordered.slice(0, 117));
  const priorTrackEnd = 116;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = continuity.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(216);
  expect(priorWindowMissesAfterBridge).toBe(210);
});

it("pins Gujarati R4 bridge C at positions 123 through 128", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bridgeIds = [
    "GU-R17-you-r4",
    "GU-R17-shun-r4",
    "GU-R17-whats-your-name-r4",
    "GU-R17-introduction-r4",
    "GU-R17-hun-r4",
    "GU-R17-kem-r4",
  ];
  expect(ordered.slice(123, 129).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(123, 129).every((lesson) =>
      ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactR4 = [
    "GU-CONCEPT-C02-TUTAME-01",
    "GU-CONCEPT-C02-SHUN-01",
    "GU-CONCEPT-C02-TAMARUNNAAMSHUNCHHE-01",
    "GU-CONCEPT-C02-PRACTICE-01",
    "GU-CONCEPT-C03-HUN-01",
    "GU-CONCEPT-C03-KEM-01",
  ];
  expect(
    exactR4.map((atom, offset) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 123 + offset - introducedAt;
    }),
  ).toEqual([62, 62, 62, 61, 61, 61]);

  const continuity = measureContinuity(ordered.slice(0, 129));
  expect(
    continuity.reinforcement.filter(
      (defect) => exactR4.includes(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);

  const beforeBridge = measureContinuity(ordered.slice(0, 123));
  const priorTrackEnd = 122;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = continuity.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(231);
  expect(priorWindowMissesAfterBridge).toBe(228);
});

it("pins Gujarati R4 bridge D at positions 129 through 133", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bridgeIds = [
    "GU-R18-how-are-you-r4",
    "GU-R18-majaa-r4",
    "GU-R18-no-problem-r4",
    "GU-R18-wellbeing-r4",
    "GU-R18-farewell-r4",
  ];
  expect(ordered.slice(129, 134).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(129, 134).every((lesson) =>
      ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactR4 = [
    "GU-CONCEPT-C03-TAMEKEMCHHO-01",
    "GU-CONCEPT-C03-MAJAA-01",
    "GU-CONCEPT-C03-VANDHONAHI-01",
    "GU-CONCEPT-C03-PRACTICE-01",
    "GU-CONCEPT-C04-MALISHUN-01",
  ];
  expect(
    exactR4.map((atom, offset) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 129 + offset - introducedAt;
    }),
  ).toEqual([61, 61, 61, 61, 61]);

  const farewellR3 = [
    "GU-CONCEPT-C04-KAALE-01",
    "GU-CONCEPT-C04-PACHHA-01",
    "GU-CONCEPT-C04-PACHHAMALISHUN-01",
    "GU-CONCEPT-C04-PRACTICE-01",
  ];
  expect(
    farewellR3.map((atom) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 133 - introducedAt;
    }),
  ).toEqual([60, 59, 58, 57]);

  const continuity = measureContinuity(ordered.slice(0, 134));
  expect(
    continuity.reinforcement.filter(
      (defect) => exactR4.includes(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);
  expect(
    continuity.reinforcement.filter(
      (defect) => farewellR3.includes(defect.atom) && defect.missed.includes("R3"),
    ),
  ).toEqual([]);

  const beforeBridge = measureContinuity(ordered.slice(0, 129));
  const priorTrackEnd = 128;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = continuity.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(245);
  expect(priorWindowMissesAfterBridge).toBe(239);
});

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
  // The doorway assertion below is unchanged and still passes, which is the
  // property this test actually owns.
  expect(afterCheckpoint.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(364);
  expect(
    afterCheckpoint.reinforcement.filter(
      (defect) => doorway.includes(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);
});

it("closes Gujarati runway B at exact R1 and R2 positions", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  expect([
    ordered[153]?.realization.lessonId,
    ordered[158]?.realization.lessonId,
    ordered[164]?.realization.lessonId,
  ]).toEqual([
    "GU-C22-shaalaa",
    "GU-R23-route-three-r1",
    "GU-R23-shaalaa-rasto-r2",
  ]);
  expect(
    [ordered[158], ordered[164]].every(
      (lesson) => ((lesson?.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactReturns = [
    ["GU-SCRIPT-SHAHAR-01", 153, 2],
    ["GU-PERFORMANCE-ROUTE-THREE-FOUR-SKILL-01", 158, 2],
    ["GU-LEX-SHAALAA-01", 164, 12],
    ["GU-SCRIPT-SHAALAA-01", 164, 11],
    ["GU-LEX-RASTO-01", 164, 10],
    ["GU-SCRIPT-RASTO-01", 164, 9],
  ] as const;
  expect(
    exactReturns.map(([atom, returnAt]) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      expect(
        ((ordered[returnAt]?.frontmatter["practises.knowledge"] ?? []) as string[]),
      ).toContain(atom);
      return returnAt - introducedAt;
    }),
  ).toEqual(exactReturns.map(([, , distance]) => distance));

  const targets = new Set(exactReturns.map(([atom]) => atom));
  expect(
    measureContinuity(lessons).reinforcement.filter((defect) => targets.has(defect.atom)),
  ).toEqual([]);
});

it("pins Gujarati-owned objective activities", () => {
  const ids = loadTrackLessons("gujarati")
    .flatMap((lesson) => compileLessonActivities(lesson.blocks))
    .map((activity) => activity.id)
    .sort();
  expect(ids).toEqual([
    "GU-C01-aabhaar-script-retrieval",
    "GU-C01-aavjo-script-retrieval",
    "GU-C01-haa-naa-script-retrieval",
    "GU-C01-practice-four-consonants",
    "GU-C01-practice-two-consonants-one-sign",
    "GU-C01-practice-writing-payoff",
    "GU-C02-shun-da-retrieval",
    "GU-C02-tamarun-naam-shun-chhe-va-retrieval",
    "GU-C02-tu-tame-ra-retrieval",
    "GU-C03-hun-ya-retrieval",
    "GU-C03-kem-bha-retrieval",
    "GU-C03-majaa-vocalic-r-retrieval",
    "GU-C03-tame-kem-chho-dha-retrieval",
    "GU-C06-number-histories-be-source",
    "GU-C20-bajar-copy",
    "GU-C20-ghar-write",
    "GU-C20-hear-bajar-recall",
    "GU-C20-hear-ghar-recall",
    "GU-C20-hear-mandir-recall",
    "GU-C20-mandir-write",
    "GU-C20-places-three-payoff",
    "GU-C21-haath-write",
    "GU-C21-hear-haath-recall",
    "GU-C21-hear-paisa-recall",
    "GU-C21-paisa-write",
    "GU-C21-travel-five-payoff",
    "GU-C22-hear-rasto-recall",
    "GU-C22-hear-shaalaa-recall",
    "GU-C22-hear-shahar-recall",
    "GU-C22-rasto-copy",
    "GU-C22-route-three-payoff",
    "GU-C22-shaalaa-city-r1",
    "GU-C22-shaalaa-delayed",
    "GU-C22-shahar-copy",
    "GU-C23-dukaan-copy",
    "GU-C23-gaam-delayed",
    "GU-C23-hear-dukaan-recall",
    "GU-C23-hear-gaam-recall",
    "GU-C23-map-ten-payoff",
    "GU-C24-aaj-guided-copy",
    "GU-C24-aaj-reading",
    "GU-C24-day-parts-four-listening",
    "GU-C24-day-parts-four-speaking",
    "GU-C24-hear-aaj-recall",
    "GU-C24-hear-atyaare-recall",
    "GU-C24-hear-bapor-recall",
    "GU-C24-hear-divas-recall",
    "GU-C24-hear-mahino-recall",
    "GU-C24-hear-raat-recall",
    "GU-C24-hear-saanj-recall",
    "GU-C24-hear-savaar-recall",
    "GU-C24-raat-guided-copy",
    "GU-C24-raat-reading",
    "GU-C24-time-eight-listening",
    "GU-C24-time-eight-reading",
    "GU-C24-time-eight-speaking",
    "GU-C24-time-eight-writing",
    "GU-C25-atyaare-guided-copy",
    "GU-C25-atyaare-reading",
    "GU-C25-bapor-guided-copy",
    "GU-C25-bapor-reading",
    "GU-C25-divas-guided-copy",
    "GU-C25-divas-reading",
    "GU-C25-mahino-delayed-copy",
    "GU-C25-mahino-reading",
    "GU-C25-saanj-guided-copy",
    "GU-C25-saanj-reading",
    "GU-C25-savaar-guided-copy",
    "GU-C25-savaar-reading",
    "GU-C25-time-written-reading",
    "GU-C25-time-written-speaking",
    "GU-C25-time-written-writing",
    "GU-C27-bhaat-guided-copy",
    "GU-C27-bhaat-reading",
    "GU-C27-hear-bhaat-recall",
    "GU-C27-hear-daal-recall",
    "GU-C27-hear-keri-recall",
    "GU-C27-hear-shaak-recall",
    "GU-C27-hear-tel-recall",
    "GU-C27-plate-five-carryover",
    "GU-C27-plate-five-listening",
    "GU-C27-plate-five-speaking",
    "GU-C28-baari-guided-copy",
    "GU-C28-baari-reading",
    "GU-C28-baari-time-r3",
    "GU-C28-hear-baari-recall",
    "GU-C28-hear-baarnun-recall",
    "GU-C28-hear-chaavi-recall",
    "GU-C28-hear-divo-recall",
    "GU-C28-hear-khurshi-recall",
    "GU-C28-house-five-carryover",
    "GU-C28-house-five-listening",
    "GU-C28-house-five-speaking",
    "GU-C29-hear-aakaash-recall",
    "GU-C29-hear-chandra-recall",
    "GU-C29-hear-nadi-recall",
    "GU-C29-hear-sooraj-recall",
    "GU-C29-hear-varsaad-recall",
    "GU-C29-nadi-guided-copy",
    "GU-C29-nadi-reading",
    "GU-C29-sky-five-carryover",
    "GU-C29-sky-five-listening",
    "GU-C29-sky-five-speaking",
    "GU-C30-hear-chhokri-recall",
    "GU-C30-hear-chhokro-recall",
    "GU-C30-hear-kaagal-recall",
    "GU-C30-hear-maanas-recall",
    "GU-C30-hear-pustak-recall",
    "GU-C30-kaagal-guided-copy",
    "GU-C30-kaagal-map-r4",
    "GU-C30-kaagal-nadi-r2",
    "GU-C30-kaagal-reading",
    "GU-C30-people-five-carryover",
    "GU-C30-people-five-listening",
    "GU-C30-people-five-speaking",
    "GU-R03-doorway-three-r1-dictation",
    "GU-R03-doorway-three-r1-reading",
    "GU-R04-doorway-nine-r2-dictation",
    "GU-R04-doorway-nine-r2-reading",
    "GU-R04-first-four-r1-dictation",
    "GU-R04-first-four-r1-reading",
    "GU-R05-second-four-r1-dictation",
    "GU-R05-second-four-r1-doorway-three-r2-dictation",
    "GU-R05-second-four-r1-doorway-three-r2-reading",
    "GU-R05-second-four-r1-reading",
    "GU-R06-first-four-r1-dictation",
    "GU-R06-first-four-r1-reading",
    "GU-R07-second-four-r1-dictation",
    "GU-R07-second-four-r1-reading",
    "GU-R13-doorway-nine-r3-dictation",
    "GU-R13-doorway-nine-r3-reading",
    "GU-R15-chha-r4-dictation",
    "GU-R15-chha-r4-reading",
    "GU-R15-ka-r4-dictation",
    "GU-R15-ka-r4-reading",
    "GU-R15-name-exchange-r3-production",
    "GU-R15-name-exchange-r3-reading",
    "GU-R15-nna-r4-dictation",
    "GU-R15-nna-r4-reading",
    "GU-R15-sha-r4-dictation",
    "GU-R15-sha-r4-reading",
    "GU-R15-u-matra-r4-dictation",
    "GU-R15-u-matra-r4-reading",
    "GU-R16-anand-r4-listening",
    "GU-R16-anand-r4-writing",
    "GU-R16-chhe-r4-listening",
    "GU-R16-chhe-r4-writing",
    "GU-R16-maarun-r4-listening",
    "GU-R16-maarun-r4-writing",
    "GU-R16-my-name-is-r4-listening",
    "GU-R16-my-name-is-r4-writing",
    "GU-R16-naam-r4-listening",
    "GU-R16-naam-r4-writing",
    "GU-R16-wellbeing-r3-listening",
    "GU-R16-wellbeing-r3-writing",
    "GU-R17-hun-r4-listening",
    "GU-R17-hun-r4-writing",
    "GU-R17-introduction-r4-listening",
    "GU-R17-introduction-r4-writing",
    "GU-R17-kem-r4-listening",
    "GU-R17-kem-r4-writing",
    "GU-R17-shun-r4-listening",
    "GU-R17-shun-r4-writing",
    "GU-R17-whats-your-name-r4-listening",
    "GU-R17-whats-your-name-r4-writing",
    "GU-R17-you-r4-listening",
    "GU-R17-you-r4-writing",
    "GU-R18-farewell-r4-listening",
    "GU-R18-farewell-r4-writing",
    "GU-R18-how-are-you-r4-listening",
    "GU-R18-how-are-you-r4-writing",
    "GU-R18-majaa-r4-listening",
    "GU-R18-majaa-r4-writing",
    "GU-R18-no-problem-r4-listening",
    "GU-R18-no-problem-r4-writing",
    "GU-R18-wellbeing-r4-listening",
    "GU-R18-wellbeing-r4-writing",
    "GU-R19-doorway-nine-r4-dictation",
    "GU-R19-doorway-nine-r4-reading",
    "GU-R21-travel-five-recall",
    "GU-R23-map-ten-recall",
    "GU-R23-route-three-r1-listening",
    "GU-R23-route-three-r1-reading",
    "GU-R23-route-three-r1-speaking",
    "GU-R23-route-three-r1-writing",
    "GU-R23-shaalaa-rasto-r2-listening",
    "GU-R23-shaalaa-rasto-r2-reading",
    "GU-R23-shaalaa-rasto-r2-speaking",
    "GU-R23-shaalaa-rasto-r2-writing",
    "GU-R24-map-ten-r3-listening",
    "GU-R24-map-ten-r3-speaking",
    "GU-R24-map-ten-r3-writing",
    "GU-R24-time-eight-r1-listening",
    "GU-R24-time-eight-r1-speaking",
    "GU-R24-time-eight-r1-writing",
    "GU-R25-time-written-r1-listening",
    "GU-R25-time-written-r1-writing",
    "GU-R26-aavvun-khaavun-r4-listening",
    "GU-R26-aavvun-khaavun-r4-writing",
    "GU-R26-hovun-javun-r4-listening",
    "GU-R26-hovun-javun-r4-writing",
    "GU-R26-jovun-jaanvun-r4-listening",
    "GU-R26-jovun-jaanvun-r4-writing",
    "GU-R26-levun-puchhvun-r4-listening",
    "GU-R26-levun-puchhvun-r4-writing",
    "GU-R26-madad-gamvun-paani-r4-listening",
    "GU-R26-madad-gamvun-paani-r4-writing",
    "GU-R26-numbers-r4-listening",
    "GU-R26-numbers-r4-writing",
    "GU-R26-time-written-r2-listening",
    "GU-R26-time-written-r2-map-writing",
    "GU-R26-time-written-r2-writing",
    "GU-R26-vanchvun-lakhvun-r4-listening",
    "GU-R26-vanchvun-lakhvun-r4-writing",
    "GU-R26-vicharvun-samajvun-r4-listening",
    "GU-R26-vicharvun-samajvun-r4-writing",
    "GU-R27-plate-five-r1-distant",
    "GU-R27-plate-five-r1-listening",
    "GU-R27-plate-five-r1-speaking",
    "GU-R27-plate-five-r1-writing",
    "GU-R28-house-five-r1-distant",
    "GU-R28-house-five-r1-listening",
    "GU-R28-house-five-r1-plate-r2",
    "GU-R28-house-five-r1-speaking",
    "GU-R28-house-five-r1-writing",
    "GU-R29-sky-five-r1-distant",
    "GU-R29-sky-five-r1-house-r2",
    "GU-R29-sky-five-r1-listening",
    "GU-R29-sky-five-r1-speaking",
    "GU-R29-sky-five-r1-writing",
    "GU-R30-people-five-r1-distant",
    "GU-R30-people-five-r1-listening",
    "GU-R30-people-five-r1-speaking",
    "GU-R30-people-five-r1-travel-r4",
    "GU-R30-people-five-r1-writing",
    "GU-W01-aa-matra-observe-check",
    "GU-W01-ha-observe-check",
    "GU-W01-haa-delayed-copy-check",
    "GU-W01-haa-delayed-copy-vocalic-r-retrieval",
    "GU-W01-haa-dictation-answer",
    "GU-W01-haa-guided-copy-check",
    "GU-W01-haa-guided-copy-dha-retrieval",
    "GU-W02-bha-recall",
    "GU-W02-da-recall",
    "GU-W02-dha-recall",
    "GU-W02-ra-recall",
    "GU-W02-va-recall",
    "GU-W02-vocalic-r-recall",
    "GU-W02-ya-recall",
    "GU-W03-anusvara-recall",
    "GU-W03-chha-recall",
    "GU-W03-ii-matra-recall",
    "GU-W03-ja-recall",
    "GU-W03-ka-recall",
    "GU-W03-nna-recall",
    "GU-W03-o-matra-recall",
    "GU-W03-sha-doorway-dictation",
    "GU-W03-sha-doorway-reading",
    "GU-W03-sha-recall",
    "GU-W03-u-matra-recall",
    "GU-W04-i-matra-recall",
    "GU-W04-independent-a-recall",
    "GU-W04-lla-recall",
    "GU-W04-tha-recall",
    "GU-W05-ba-recall",
    "GU-W05-cha-recall",
    "GU-W05-la-recall",
    "GU-W05-pa-recall",
    "GU-W06-ga-recall",
    "GU-W06-independent-e-recall",
    "GU-W06-independent-u-recall",
    "GU-W06-kha-recall",
    "GU-W07-ddha-recall",
    "GU-W07-independent-ii-recall",
    "GU-W07-tta-recall",
    "GU-W07-uu-matra-recall",
    "GU-W20-gha-copy",
    "GU-W21-ai-matra-copy",
  ]);
});
