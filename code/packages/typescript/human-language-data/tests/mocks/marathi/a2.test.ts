import { createHash } from "node:crypto";
import { existsSync, lstatSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";
import { listMockAssessmentStatuses } from "../../../src/mock-assessment.js";

const curriculumRoot = defaultCurriculumRoot();
const root = join(curriculumRoot, "marathi", "mocks", "a2");
const read = (name: string) => readFileSync(join(root, name), "utf8");
const numberedItems = (text: string, start: string, end?: string) => {
  const from = text.indexOf(start);
  const to = end === undefined ? text.length : text.indexOf(end, from + start.length);
  return text.slice(from, to).match(/^\d+\./gmu)?.length ?? 0;
};
const devanagariWords = (text: string) => text.match(/[\u0900-\u097F]+/gu)?.length ?? 0;
const between = (text: string, start: string, end: string) =>
  text.slice(text.indexOf(start) + start.length, text.indexOf(end, text.indexOf(start) + start.length));
const part = (text: string, partId: string) => between(
  text,
  `<!-- mock-part:${partId} -->`,
  `<!-- /mock-part:${partId} -->`,
);

const partIds = [
  "a2-reading-notices-and-correspondence",
  "a2-reading-connected-everyday-text-one",
  "a2-reading-connected-everyday-text-two",
  "a2-listening-announcements",
  "a2-listening-everyday-conversations",
  "a2-listening-connected-account",
  "a2-writing-functional-response",
  "a2-writing-connected-message-or-description",
  "a2-speaking-personal-interview",
  "a2-speaking-prepared-account",
  "a2-speaking-information-exchange-role-play",
] as const;

describe("Marathi A2 timed mock pair", () => {
  it("binds two complete forms to every published A2 task-shape part", () => {
    expect(listMockAssessmentStatuses(curriculumRoot).find(
      (status) => status.language === "marathi" && status.level === "A2",
    )).toMatchObject({
      requiredMocks: 2,
      completeMocks: 2,
      missingMocks: [],
      blockedBy: [],
    });

    for (const form of [1, 2]) {
      const manifest = JSON.parse(read(`mock-${form}-manifest.json`)) as {
        language: string;
        level: string;
        mockId: string;
        sections: Array<{ skill: string; minutes: number; partIds: string[] }>;
      };
      expect(manifest).toMatchObject({
        language: "marathi",
        level: "A2",
        mockId: `a2-mock-${form}`,
      });
      expect(manifest.sections.map(({ skill, minutes }) => ({ skill, minutes }))).toEqual([
        { skill: "reading", minutes: 30 },
        { skill: "listening", minutes: 25 },
        { skill: "writing", minutes: 30 },
        { skill: "speaking", minutes: 12 },
      ]);
      expect(manifest.sections.flatMap((section) => section.partIds)).toEqual(partIds);
    }
  });

  it("owns distinct timed papers, keys, examiner scripts, manifests, and one rubric", () => {
    const names = [
      "mock-1.md",
      "mock-2.md",
      "mock-1-answer-key.md",
      "mock-2-answer-key.md",
      "mock-1-examiner-script.md",
      "mock-2-examiner-script.md",
      "mock-1-manifest.json",
      "mock-2-manifest.json",
      "rubric.md",
    ];
    expect(names.every((name) => existsSync(join(root, name)))).toBe(true);

    for (const form of [1, 2]) {
      const paper = read(`mock-${form}.md`);
      expect(paper).toContain("## Reading — 30 minutes");
      expect(paper).toContain("## Listening — 25 minutes");
      expect(paper).toContain("## Writing — 30 minutes");
      expect(paper).toContain("## Speaking — 12 minutes");
      expect(paper).toContain("पाच मिनिटांच्या तयारी");
      expect(paper).toMatch(/[\u0900-\u097F]/u);
      expect(numberedItems(paper, "## Reading", "## Listening")).toBe(24);
      expect(numberedItems(paper, "## Listening", "## Writing")).toBe(20);
      expect(numberedItems(paper, "## Writing", "## Speaking")).toBe(2);
      expect(numberedItems(paper, "## Speaking")).toBe(3);

      const script = read(`mock-${form}-examiner-script.md`);
      expect(script).toContain("110–130 words per minute");
      expect(script).toContain("two plays");
      expect(script).toContain("five-minute preparation");
      expect(script).toContain("exactly 90 seconds per play");
      expect(script).toContain("exactly 120 seconds per play");
      expect(script).toContain("10–45-second stimulus window");
      expect(script).toContain("combined 120–180-second response window");
      expect(script).toContain("allow a 120–180-second response window");

      const readingRanges = [
        ["a2-reading-notices-and-correspondence", 150, 220],
        ["a2-reading-connected-everyday-text-one", 180, 260],
        ["a2-reading-connected-everyday-text-two", 220, 270],
      ] as const;
      for (const [partId, minimum, maximum] of readingRanges) {
        const stimulus = between(part(paper, partId), "<!-- stimulus-start -->", "<!-- stimulus-end -->");
        expect(devanagariWords(stimulus), `${partId} source words`).toBeGreaterThanOrEqual(minimum);
        expect(devanagariWords(stimulus), `${partId} source words`).toBeLessThanOrEqual(maximum);
      }
      for (const [partId, minimum, maximum] of [
        ["a2-writing-functional-response", 35, 60],
        ["a2-writing-connected-message-or-description", 45, 75],
        ["a2-speaking-prepared-account", 15, 35],
        ["a2-speaking-information-exchange-role-play", 20, 40],
      ] as const) {
        expect(devanagariWords(part(paper, partId)), `${partId} prompt words`).toBeGreaterThanOrEqual(minimum);
        expect(devanagariWords(part(paper, partId)), `${partId} prompt words`).toBeLessThanOrEqual(maximum);
      }

      for (const leakedSupportDirection of ["Read the notices", "Listen to", "Answer six", "Role-play"]) {
        expect(paper).not.toContain(leakedSupportDirection);
      }
    }
    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
    expect(read("mock-1-examiner-script.md")).not.toBe(read("mock-2-examiner-script.md"));
  });

  it("pins every objective mapping and the complete taught-evidence boundary for scored content", () => {
    const expected = {
      1: [
        "1 B; 2 A; 3 B; 4 B; 5 B; 6 A; 7 A; 8 B.",
        "9 B; 10 A; 11 B; 12 B; 13 A; 14 B; 15 C; 16 A.",
        "17 B; 18 B; 19 B; 20 A; 21 C; 22 A; 23 B; 24 B.",
        "1 B; 2 C; 3 B; 4 B; 5 A; 6 B; 7 A.",
        "8 C; 9 A; 10 B; 11 B; 12 B; 13 A; 14 B.",
        "15 B; 16 A; 17 B; 18 B; 19 A; 20 B.",
      ],
      2: [
        "1 B; 2 A; 3 B; 4 C; 5 A; 6 B; 7 C; 8 B.",
        "9 B; 10 A; 11 B; 12 A; 13 B; 14 B; 15 B; 16 A.",
        "17 B; 18 A; 19 B; 20 B; 21 B; 22 A; 23 B; 24 B.",
        "1 B; 2 B; 3 B; 4 C; 5 B; 6 A; 7 A.",
        "8 C; 9 C; 10 A; 11 B; 12 A; 13 B; 14 B.",
        "15 A; 16 A; 17 B; 18 B; 19 A; 20 B.",
      ],
    } as const;
    for (const form of [1, 2] as const) {
      const key = read(`mock-${form}-answer-key.md`);
      for (const row of expected[form]) expect(key).toContain(row);
    }
    expect(read("mock-1-answer-key.md")).toContain("ask both times and the fare");
    expect(read("mock-1-examiner-script.md")).toContain("पन्नास रुपये");
    expect(read("mock-2-answer-key.md")).toContain("ask what to\nbring");

    const lessonDir = join(curriculumRoot, "marathi", "lessons");
    // Only numbered chapters already taught by the A2 gate may establish a
    // scored token. Later chapters and non-chapter support files cannot make
    // an otherwise untaught A2 word look source-bounded.
    const chapterNames = readdirSync(lessonDir).filter((name) => name.startsWith("MR-C"));
    const a2ChapterNames = chapterNames.filter((name) => {
      const match = /^MR-C(\d+)-[a-z0-9-]+\.md$/u.exec(name);
      expect(match, `invalid Marathi chapter filename: ${name}`).not.toBeNull();
      return Number(match![1]) <= 275;
    });
    expect(a2ChapterNames.length).toBeGreaterThan(0);
    const lessonCorpus = a2ChapterNames.map((name) => {
      const lessonPath = join(lessonDir, name);
      expect(lstatSync(lessonPath).isFile(), `regular Marathi lesson: ${name}`).toBe(true);
      return readFileSync(lessonPath, "utf8");
    }).join("\n");
    const lessonTokens = new Set(lessonCorpus.match(/[\u0900-\u097F]+/gu) ?? []);
    const scoredArtifacts = [1, 2].flatMap((form) => [
      read(`mock-${form}.md`),
      read(`mock-${form}-examiner-script.md`),
    ]).join("\n");
    const scoredTokens = new Set(scoredArtifacts.match(/[\u0900-\u097F]+/gu) ?? []);
    // Pin the complete learner-facing token inventory, not just a selected
    // vocabulary sample. Any new or substituted Devanagari surface form must
    // change this digest and receive a fresh source-boundary review.
    const scoredTokenInventory = [...scoredTokens].sort();
    expect(scoredTokenInventory).toHaveLength(855);
    expect(createHash("sha256").update(scoredTokenInventory.join("\n"), "utf8").digest("hex"))
      .toBe("610769072fa2a583185781fb22763e0f131b6495b57f7aa5da787ffaf4ad29fe");
    // This inventory spans the answer-bearing nouns and the open-task topic
    // roots in every reading, listening, writing, and speaking part. Each root
    // must be visible both in the scored pair and in a lesson published before
    // this assessment; the deny-list covers reviewed derivations which cannot
    // be justified merely by sharing a shorter taught prefix.
    for (const taughtScoredRoot of [
      "फलाट", "पाणी", "दुकान", "बस", "गाडी", "गाव", "पाऊस",
      "बाजार", "शाळा", "पुस्तक", "खोली", "गोष्ट", "प्रवास", "देश",
      "इतिहास", "संगीत", "आरोग्य", "वही", "पेन", "दिवा", "कपाट",
      "पेटी", "माहिती", "पैसे", "खिडकी", "छत्री", "कपडे", "फळ",
      "सायकल", "पिशवी", "झाड", "कार्यक्रम", "घर", "काम", "ठिकाण",
      "वेळ", "माणूस", "मदत", "भाजी", "भाकरी",
      "बाग", "नदी", "कुटुंब", "मित्र",
      "बटाटा", "गाजर", "टपाल", "मंदिर", "कार्यालय", "साडी", "दूध",
      "चहा", "शनिवार", "मीरा", "अरुण", "स्टेशन", "शिक्षक",
      "स्वच्छ", "धन्यवाद",
    ]) {
      expect(scoredArtifacts, `scored use of ${taughtScoredRoot}`).toContain(taughtScoredRoot);
      expect(lessonTokens.has(taughtScoredRoot), `standalone prior lesson token for ${taughtScoredRoot}`).toBe(true);
    }
    for (const untaught of [
      "तांदूळ", "हातमोजे", "पुनर्वापर", "साखळी", "टोमॅटो", "बियाणे",
      "डॉक्टर", "तात्पुरत", "परिचित", "परिचय", "अपुर", "विषयानुसार",
      "एकावेळी", "सव्वा", "साडे", "मेज", "मजकूर", "संभाषण", "सेकंद",
      "साफ", "क्रमांक", "रेष", "पंच्याऐंशी", "किलोमीटर", "रोमन", "लिपी",
      "सोप", "परीक्षक", "रेल्वे", "ज्ञान", "तिकीट", "जागा", "व्यक्ती",
      "अडचण", "मैदान", "वाहन", "कागद", "जेवण", "यंत्र", "उद्देश",
      "वक्ता", "संदेश", "रद्द", "चौकशी", "वार्षिक", "स्नेहसंमेलन",
      "नुकता", "वस्तू", "कृती", "अंतर", "उरले", "घोषणा", "छोटा",
      "नागरिक", "प्रत्येक", "केवळ", "सगळे", "सर्व", "आजचा",
    ]) {
      expect(scoredArtifacts).not.toContain(untaught);
    }
    expect(scoredTokens.has("प्रत"), "untaught standalone प्रत").toBe(false);
  });

  it("pins one visible evidence span per part and the project-only evidence boundary", () => {
    const rubric = read("rubric.md");
    for (const partId of partIds) {
      const open = `<!-- mock-part:${partId} -->`;
      const close = `<!-- /mock-part:${partId} -->`;
      for (const name of ["mock-1.md", "mock-2.md", "mock-1-answer-key.md", "mock-2-answer-key.md", "rubric.md"]) {
        const artifact = read(name);
        expect(artifact.split(open)).toHaveLength(2);
        expect(artifact.split(close)).toHaveLength(2);
        expect(artifact.indexOf(close)).toBeGreaterThan(artifact.indexOf(open) + open.length);
      }
    }
    expect(rubric).toContain("60/100 on **every** paper");
    expect(rubric).toContain("no compensatory total");
    expect(rubric).toContain("documented regional Marathi form");
    expect(rubric).toContain("does not establish exam readiness");
    expect(rubric).toContain("recording provenance");
    expect(rubric).toContain("independent human validation");
    expect(rubric).toContain("standard setting");
    expect(rubric).toContain("learner attainment");
  });
});
