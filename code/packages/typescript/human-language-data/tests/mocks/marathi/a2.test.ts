import { createHash } from "node:crypto";
import { existsSync, lstatSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  defaultCurriculumRoot,
  loadCurriculumSpine,
  loadTrackLanguageCurriculum,
  loadTrackLessons,
} from "../../../src/loader.js";
import { lessonsUpToLevel } from "../../../src/levels.js";
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

interface BoundaryLesson {
  id: string;
  forms: string[];
}

interface A2Boundary {
  language: string;
  level: string;
  maxRegularChapter: number;
  firstExcludedChapter: number;
  grammarLessons: BoundaryLesson[];
  functionalLessons: BoundaryLesson[];
}

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
    expect(read("mock-1-examiner-script.md")).toContain("जाण्याची बस सकाळी आठ वाजता आहे");
    expect(read("mock-1-examiner-script.md")).toContain("ती सहा वाजता आहे");
    expect(read("mock-1-examiner-script.md")).toContain("पन्नास रुपये");
    expect(read("mock-1-examiner-script.md")).toContain("order the learner asks them");
    expect(read("mock-1-examiner-script.md")).not.toMatch(/पाचव्या\s+पाचव्या/u);
    expect(read("mock-1-examiner-script.md")).not.toMatch(/पिशवी\s+आम्ही\s+चहा|चहा\s+चहाच्या/u);
    expect(read("mock-2-answer-key.md")).toContain("ask what to\nbring");
    expect(read("mock-2-examiner-script.md")).toContain("reply **वही**");
    expect(read("mock-2-answer-key.md")).toContain("which days and what time the room opens");
    expect(read("mock-2-examiner-script.md")).toContain("ती नऊ वाजता उघडी आहे");
    expect(read("mock-2-examiner-script.md")).toContain("पुस्तक दोन आठवड्यांत परत द्या");

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
    const boundary = JSON.parse(readFileSync(
      join(curriculumRoot, "marathi", "assessment-boundaries", "a2.json"),
      "utf8",
    )) as A2Boundary;
    const scoredArtifactBodies = [1, 2].flatMap((form) => [
      read(`mock-${form}.md`),
      read(`mock-${form}-answer-key.md`),
      read(`mock-${form}-examiner-script.md`),
    ]);
    const scoredArtifacts = scoredArtifactBodies.join("\n");

    // The next Marathi rung owns these constructions. A token appearing in
    // an earlier vocabulary lesson is not enough to make its later inflected
    // or connective use A2 evidence.
    for (const [label, laterRungSurface] of [
      ["ongoing past", /[\u0900-\u097F]+त\s+हो(?:तो|ती|ता|ते|त्या|तं)/u],
      ["causal -mule", /[\u0900-\u097F]+मुळे/u],
      ["conditional -lyas", /[\u0900-\u097F]+ल्यास/u],
      ["B1 sequence connector", /त्यानंतर/u],
      ["B1 result connector", /म्हणून/u],
      ["C278 while surface", /(?:असताना|[\u0900-\u097F]+ताना)/u],
      ["C278 past habit", /(?:जायचो|जायची|जायचे)/u],
      ["C279 conjunctive -un", /(?:करून|घेऊन|वाचून|बसून|पाहून|खाऊन|पिऊन|जाऊन|येऊन|निघून|ठेवून|देऊन|बोलून|चालून|लिहून|ऐकून)/u],
      ["C279 season noun", /ऋतू/u],
      ["C280 opinion frame", /मला\s+वाटतं\s+की/u],
      ["C280 result frame", /त्यामुळे/u],
      ["C281 obligation", /[\u0900-\u097F]+ला\s+पाहिजे/u],
      ["C281 prohibition", /नये/u],
      ["C281 knowledge noun", /ज्ञान/u],
      ["C282 directions", /(?:कसं\s+जायचं|किती\s+दूर|उजवीकडे|डावीकडे|समोर)/u],
      ["C283 lost surface", /हरवल/u],
      ["C283 missed surface", /चुकल/u],
      ["C283 ticket surface", /तिकीट/u],
      ["C284 emotion noun", /(?:भीती|ध्येय|राग)/u],
      ["C284 dative want frame", /[\u0900-\u097F]+ायचं\s+आहे/u],
      ["C285 comparison", /(?:पेक्षा|सर्वात)/u],
      ["C285 duration frame", /(?:पासून|पर्यंत|दिवसभर)/u],
      ["C286 otherwise", /नाहीतर/u],
      ["C287 dependency", /अवलंबून/u],
      ["C287 counterfactual", /[\u0900-\u097F]+ल[ोाीे]\s+अस(?:तो|ता|ती|ते)/u],
      ["unowned future passive", /[\u0900-\u097F]+(?:ला|ली|ले|लं|ल्या)\s+जा(?:ईल|तील)/u],
      ["C277 future or plan copula", /(?:असेल|असतील|आहोत)/u],
      ["later inclusive pronoun", /(?:^|[^\u0900-\u097F])आपण(?:$|[^\u0900-\u097F])/u],
      ["later habitual copula", /(?:असते|असतात)/u],
      ["unowned impersonal ability", /(?:घेता|ठेवता|वाचता|बोलता)\s+(?:ये|यावं)/u],
      ["unowned ability combination", /असू\s+शकत/u],
      ["unowned prescriptive or subjunctive", /(?:घ्यावी|द्यावे|यावं|घालावी|आणावी|करावा|आणावं|द्यावी|भेटावं|बोलावं|करावी|द्यावं|ठेवावी|मागावी)/u],
      ["unowned relative or result participle", /[\u0900-\u097F]+ले(?:ला|ली|लेले|लं|ल्या|ल्यां|लेल्या)/u],
      ["unowned agreeing future participle", /[\u0900-\u097F]+णार(?:ा|ी|े|्या)/u],
      ["C280 demonstrative discourse form", /असं/u],
      ["later concessive or conditional", /(?:^|[^\u0900-\u097F])तरी(?:$|[^\u0900-\u097F])/u],
      ["unowned before-time nominal", /वाजण्यापूर्वी/u],
      ["unowned nominalized decision", /[\u0900-\u097F]+ण्याचं(?:\s+[\u0900-\u097F]+){0,3}\s+ठरवल/u],
      ["untaught instead construction", /ऐवजी/u],
      ["later negative conjunctive", /(?:^|[^\u0900-\u097F])न\s+[\u0900-\u097F]+ता(?:$|[^\u0900-\u097F])/u],
      ["later first-time adverb", /पहिल्यांदा/u],
      ["unowned attributive postposition", /[\u0900-\u097F]+वरील/u],
      ["later distributive source", /महिन्यातून/u],
      ["later reciprocal", /एकमेकांना/u],
      ["unowned feminine friend", /मैत्रीण/u],
      ["unowned bicycle-ride compound", /सायकलफेरी/u],
      ["later verbal-adjective or deontic family", /[\u0900-\u097F]+ायच[\u0900-\u097F]*/u],
      ["later list noun", /यादी/u],
      ["later collective numeral", /दोघ/u],
      ["later reflexive pronoun", /स्वतः/u],
      ["unowned partitive direction", /पैकी/u],
      ["unowned abstract cleanliness noun", /स्वच्छत/u],
      ["untaught father noun", /बाबा/u],
      ["unowned attention collocation", /(?:लक्ष\s+द्या|लक्षात\s+ठेवा)/u],
      ["awkward late-notice collocation", /उशीर\s+घरी\s+सांगेन/u],
      ["later use verb", /वापर/u],
      ["unowned summon verb", /बोलव/u],
      ["later drive verb", /चालव/u],
      ["later sentence noun", /वाक्य/u],
      ["later negative future copula", /नसेल/u],
      ["later finish verb", /संप(?:ेल|लं)/u],
      ["later depart verb", /सुट(?:ेल|णार)/u],
      ["later appear verb", /दिसली/u],
      ["later start surface", /(?:^|[^\u0900-\u097F])सुरू(?:$|[^\u0900-\u097F])/u],
      ["later previous surface", /मागच्या/u],
      ["later nice adjective", /(?:^|[^\u0900-\u097F])छान(?:$|[^\u0900-\u097F])/u],
      ["later all adjective", /सगळी/u],
      ["later complete adjective", /(?:^|[^\u0900-\u097F])पूर्ण(?:$|[^\u0900-\u097F])/u],
      ["later new adjective", /नवीन/u],
      ["later ready adjective", /(?:^|[^\u0900-\u097F])तयार(?:$|[^\u0900-\u097F])/u],
      ["later en-route locative", /वाटेत/u],
      ["later attributive last surface", /(?:^|[^\u0900-\u097F])गेल्या(?:$|[^\u0900-\u097F])/u],
      ["unowned productive dative infinitive", /[\u0900-\u097F]+ायला/u],
      ["unowned reading noun", /वाचन/u],
      ["unowned market-held sense", /बाजार[^\n.]{0,80}भर(?:तो|ला|ेल)/u],
      ["later people noun", /(?:^|[^\u0900-\u097F])लोक(?:ांना|ांनी)?(?:$|[^\u0900-\u097F])/u],
      ["later change noun", /(?:^|[^\u0900-\u097F])बदल(?:$|[^\u0900-\u097F])/u],
      ["broken possessive agreement", /गावाचे\s+आणि\s+शहराची\s+माणसं/u],
      ["later quantity word", /अनेक/u],
      ["later manner adverb", /शांतपणे/u],
      ["later reflexive possessive", /आपल(?:ा|ी|े|्या)/u],
      ["later about postposition", /बद्दल/u],
      ["untaught feminine teacher noun", /शिक्षिका/u],
      ["untaught student noun", /विद्यार्थ/u],
      ["unowned choice verb", /निवड[\u0900-\u097F]*/u],
      ["untaught wait idiom", /वाट\s+पाह/u],
      ["later attributive locative", /[\u0900-\u097F]+मधील|(?:शहर|गाव|शाळा|घर|पुस्तक|बाजार|प्रवास)ातील/u],
    ] as const) {
      expect(scoredArtifacts, `later-rung scored surface: ${label}`).not.toMatch(laterRungSurface);
    }
    for (const laterRungCondition of [
      "पाऊस पडत असेल तर",
      "हवामान चांगलं असेल तर",
      "पाऊस आला तर",
    ]) {
      expect(scoredArtifacts, `later-rung scored condition: ${laterRungCondition}`)
        .not.toContain(laterRungCondition);
    }
    expect(scoredArtifacts.match(/[\u0900-\u097F]+/gu) ?? [], "B1 conditional token")
      .not.toContain("तर");
    expect(scoredArtifacts.match(/[\u0900-\u097F]+/gu) ?? [], "B1 condition introducer")
      .not.toContain("जर");

    // Reject close reuse from the complete excluded B1 grammar/story seam,
    // including the rain account and direction payoff that exposed the leak.
    // Four-word spans catch distinctive copied phrasing; the sole allow-list
    // entry is the earlier C160-owned future question repeated at C277.
    const excludedB1Sources = readdirSync(lessonDir).filter((name) => {
      const chapter = Number(/^MR-C(\d+)-/u.exec(name)?.[1]);
      return chapter >= boundary.firstExcludedChapter && chapter <= 287;
    });
    expect(excludedB1Sources.length).toBeGreaterThan(0);
    const excludedB1Spans = new Set(
      excludedB1Sources.flatMap((name) => {
        const words = readFileSync(join(lessonDir, name), "utf8")
          .match(/[\u0900-\u097F]+/gu) ?? [];
        return words.slice(0, -3).map((_, index) => words.slice(index, index + 4).join(" "));
      }),
    );
    const permittedRepeatedSpans = new Set(["तुम्ही काय करणार आहात"]);
    const scoredSpans = scoredArtifactBodies.flatMap((body) => {
      const words = body.match(/[\u0900-\u097F]+/gu) ?? [];
      return words.slice(0, -3).map((_, index) => words.slice(index, index + 4).join(" "));
    });
    const unapprovedRepeatedSpans = scoredSpans
      .filter((span) => excludedB1Spans.has(span) && !permittedRepeatedSpans.has(span));
    expect(unapprovedRepeatedSpans, "no unapproved four-word scored span may reuse the excluded B1 seam")
      .toEqual([]);

    // Lexical rewrites must not preserve the excluded C279 story's distinctive
    // structure. Scope the fingerprint to form 1's first connected account so
    // ordinary A2 words elsewhere cannot hide a semantic source copy.
    const formOneConnectedAccount = read("mock-1.md").match(
      /<!-- mock-part:a2-reading-connected-everyday-text-one -->([\s\S]*?)<!-- \/mock-part:a2-reading-connected-everyday-text-one -->/u,
    )?.[1] ?? "";
    for (const [label, signals] of [
      ["rainy village setting", [/पावसाळ|पाऊस/u, /गाव/u]],
      ["rail-and-book journey", [/(?:गाडी|स्टेशन)/u, /पुस्तक/u]],
      ["mother's courtyard story meal", [/आई/u, /चहा/u, /अंगण/u, /(?:गोष्ट|भाकरी)/u]],
    ] as const) {
      expect(signals.every((signal) => signal.test(formOneConnectedAccount)), `no C279 semantic reuse: ${label}`)
        .toBe(false);
    }

    // Every content root in the independently structured replacement account
    // is tied to an explicit regular lesson at or before C275. Inflection is
    // exercised by the already-owned A2 grammar; new lexical content may not
    // enter merely because the whole-file token digest changed.
    for (const [root, owner, headword] of [
      ["शनिवार", "MR-C125-sanivar", "शनिवार"], ["सकाळ", "MR-C66-sakal", "सकाळ"],
      ["मित्र", "MR-C11-mitra", "मित्र"], ["मोठ", "MR-C49-motha", "मोठा"],
      ["शाळा", "MR-C94-shala", "शाळा"], ["मराठी", "MR-C05-mi-marathi-bolto", "मराठी"],
      ["पुस्तक", "MR-C93-pustak", "पुस्तक"], ["कार्यक्रम", "MR-C263-karyakram", "कार्यक्रम"],
      ["बसस्थानक", "MR-C271-basasthanak", "बसस्थानक"], ["बस", "MR-C114-bas", "बस"],
      ["चहा", "MR-C10-chaha", "चहा"], ["खिडकी", "MR-C89-khidki", "खिडकी"],
      ["पाणी", "MR-C10-paani", "पाणी"], ["दुकान", "MR-C113-dukan", "दुकान"],
      ["नाव", "MR-C02-naav", "नाव"], ["खोली", "MR-C41-kholi", "खोली"],
      ["गोष्ट", "MR-C120-gosta", "गोष्ट"], ["प्रवास", "MR-C119-pravas", "प्रवास"],
      ["इतिहास", "MR-C250-itihas", "इतिहास"], ["संगीत", "MR-C120-sangit", "संगीत"],
      ["आरोग्य", "MR-C169-arogya", "आरोग्य"], ["शिक्षक", "MR-C77-shiksak", "शिक्षक"],
      ["भात", "MR-C85-bhat", "भात"], ["बाग", "MR-C128-bag", "बाग"],
      ["गाण", "MR-C79-gane", "गाणे"], ["माहिती", "MR-C191-mahiti", "माहिती"],
      ["आई", "MR-C74-ai", "आई"], ["वही", "MR-C93-vahi", "वही"],
      ["पेन", "MR-C93-pen", "पेन"], ["संग्रहालय", "MR-C245-sangrahalay", "संग्रहालय"],
      ["नकाशा", "MR-C94-nakasha", "नकाशा"], ["कपड", "MR-C115-kapde", "कपडे"],
      ["चित्र", "MR-C95-chitra", "चित्र"], ["नाण", "MR-C182-nane", "नाणे"],
      ["घर", "MR-C41-ghar", "घर"], ["दिवस", "MR-C66-divas", "दिवस"],
      ["मुल", "MR-C74-mulga", "मुलगा"],
    ] as const) {
      const ownerChapter = Number(owner.match(/^MR-C(\d+)-/u)?.[1]);
      expect(ownerChapter, `${owner} chapter`).toBeLessThanOrEqual(boundary.maxRegularChapter);
      const ownerLesson = readFileSync(join(lessonDir, `${owner}.md`), "utf8");
      expect(ownerLesson, `${owner} identity`).toContain(`id: ${owner}`);
      const authoredHeadword = /^headword:\s*(.+)$/mu.exec(ownerLesson)?.[1].replaceAll('"', "") ?? "";
      expect(authoredHeadword, `${owner} headword owns ${root}`).toContain(headword);
      expect(formOneConnectedAccount, `replacement account uses ${root}`).toContain(root);
    }

    // The language-owned boundary contract is the authority for the A2
    // grammar and functional-language forms used by these papers. Bind every
    // owner in that contract to at least one visible assessed form, and bind
    // each selected form back to both the declaration and its pre-gate lesson.
    expect(boundary).toMatchObject({
      language: "marathi",
      level: "A2",
      maxRegularChapter: 275,
      firstExcludedChapter: 276,
    });
    const assessedBoundaryForms = new Map<string, string[]>([
      ["MR-C159-gelo", ["गेलो", "गेला", "आलो", "आली"]],
      ["MR-C159-tyane-kela", ["त्याने", "केलं", "खाल्ली", "पाहिला"]],
      ["MR-C159-hota", ["होता", "होती", "होते"]],
      ["MR-C160-jain", ["जाईल", "जाऊ", "जातील"]],
      ["MR-C160-janar-ahe", ["जाणार आहे", "येणार नाही"]],
      ["MR-C32-nako", ["नको"]],
      ["MR-C34-kaaran", ["कारण"]],
      ["MR-C111-adhi", ["आधी"]],
      ["MR-C111-nantar", ["नंतर"]],
      ["MR-C171-mag", ["मग"]],
      ["MR-C171-sevti", ["शेवटी"]],
    ]);
    const boundaryOwners = [
      ...boundary.grammarLessons,
      ...boundary.functionalLessons,
    ];
    const lessons = loadTrackLessons("marathi");
    const curriculum = loadTrackLanguageCurriculum("marathi");
    expect(curriculum, "Marathi curriculum must load").toBeDefined();
    const lessonById = new Map(lessons.map((lesson) => [
      lesson.realization.lessonId,
      lesson,
    ]));
    const a2LessonIds = new Set(lessonsUpToLevel(
      lessons,
      curriculum ? [curriculum] : [],
      loadCurriculumSpine(),
      "A2",
    ).map((lesson) => lesson.realization.lessonId));
    expect([...assessedBoundaryForms.keys()].sort())
      .toEqual(boundaryOwners.map(({ id }) => id).sort());
    for (const owner of boundaryOwners) {
      const chapter = Number(owner.id.match(/^MR-C(\d+)-/u)?.[1]);
      expect(chapter, `${owner.id} chapter`).toBeLessThanOrEqual(boundary.maxRegularChapter);
      const ownedLesson = lessonById.get(owner.id);
      expect(ownedLesson, `${owner.id} must resolve`).toBeDefined();
      expect(Number(ownedLesson?.frontmatter.chapter), `${owner.id} authored chapter`)
        .toBeLessThanOrEqual(boundary.maxRegularChapter);
      expect(a2LessonIds, `${owner.id} must remain in the A2 lesson slice`)
        .toContain(owner.id);
      const authoredLesson = readFileSync(
        join(curriculumRoot, "marathi", "lessons", `${owner.id}.md`),
        "utf8",
      );
      for (const form of assessedBoundaryForms.get(owner.id) ?? []) {
        expect(owner.forms, `${owner.id} declares ${form}`).toContain(form);
        expect(authoredLesson, `${owner.id} teaches ${form}`).toContain(form);
        expect(scoredArtifacts, `A2 mocks visibly assess ${form}`).toContain(form);
      }
    }

    const scoredTokens = new Set(scoredArtifacts.match(/[\u0900-\u097F]+/gu) ?? []);
    // Pin the complete learner-facing token inventory, not just a selected
    // vocabulary sample. Any new or substituted Devanagari surface form must
    // change this digest and receive a fresh source-boundary review.
    const scoredTokenInventory = [...scoredTokens].sort();
    expect(scoredTokenInventory).toHaveLength(724);
    expect(createHash("sha256").update(scoredTokenInventory.join("\n"), "utf8").digest("hex"))
      .toBe("b1e8aac7966dec4bdb71e4e0ab4c35e17aa156b0ff6efd815bdefd3beaa459f4");
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
