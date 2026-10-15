import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "sanskrit", "mocks", "c2");
const read = (name: string) => readFileSync(join(root, name), "utf8");

function between(text: string, start: string, end?: string): string {
  const from = text.indexOf(start);
  expect(from, start).toBeGreaterThanOrEqual(0);
  const to = end === undefined ? text.length : text.indexOf(end, from + start.length);
  expect(to, end).toBeGreaterThan(from);
  return text.slice(from, to);
}

const numbered = (text: string) => text.match(/^\d+\./gmu)?.length ?? 0;
const devanagariWords = (text: string) => text.match(/[\u0900-\u097F]+/gu)?.length ?? 0;
const speakerMetadata = [
  "<!-- speaker: Archivist -->", "<!-- speaker: Doctor -->",
  "<!-- speaker: DrummerOne -->", "<!-- speaker: DrummerTwo -->",
  "<!-- speaker: Guard -->", "<!-- speaker: Lecturer -->",
  "<!-- speaker: Moderator -->", "<!-- speaker: Narrator -->",
  "<!-- speaker: Officer -->", "<!-- speaker: Parent -->",
  "<!-- speaker: Presenter -->", "<!-- speaker: Priest -->",
  "<!-- speaker: Principal -->", "<!-- speaker: Producer -->",
  "<!-- speaker: access -->", "<!-- speaker: actor -->",
  "<!-- speaker: anagha -->", "<!-- speaker: curator -->",
  "<!-- speaker: dancer -->", "<!-- speaker: digital -->",
  "<!-- speaker: dyer -->", "<!-- speaker: finance -->",
  "<!-- speaker: guard -->", "<!-- speaker: lecturer -->",
  "<!-- speaker: moderator -->", "<!-- speaker: narrator -->",
  "<!-- speaker: presenter -->", "<!-- speaker: student -->",
] as const;
const spokenText = (text: string) => speakerMetadata
  .reduce((spoken, metadata) => spoken.replaceAll(metadata, ""), text);
const spokenDevanagariWords = (text: string) => devanagariWords(spokenText(text)
  .replace(/^\*\*[^*\n]+:\*\*\s*/gmu, ""));
const normalized = (text: string) => text.replace(/\s+/gu, " ");
const renderedLearnerProse = (text: string) => text.split("\n")
  .filter((line) => !line.startsWith("#"))
  .join("\n")
  .replace(/\[`mock-[12]-examiner-script\.md`\]\(mock-[12]-examiner-script\.md\)/gu, "");

const readingParts = [
  ["### Part 1", "### Part 2", 950, 1200],
  ["### Part 2", "### Part 3", 1000, 1250],
  ["### Part 3", "### Part 4", 1000, 1250],
  ["### Part 4", undefined, 1050, 1300],
] as const;

const listeningParts = [
  ["### Part 1", "### Part 2", 990, 1600],
  ["### Part 2", "### Part 3", 1155, 1800],
  ["### Part 3", "### Part 4", 1155, 1800],
  ["### Part 4", undefined, 1155, 1800],
] as const;

describe("Sanskrit C2 timed mock source pair", () => {
  it("owns two independent Sanskrit-only learner, confidential, key, and rubric bundles", () => {
    for (const name of [
      "mock-1.md", "mock-2.md", "mock-1-examiner-script.md",
      "mock-2-examiner-script.md", "mock-1-answer-key.md",
      "mock-2-answer-key.md", "rubric.md",
    ]) expect(existsSync(join(root, name)), name).toBe(true);

    for (const form of ["mock-1", "mock-2"]) {
      const paper = read(`${form}.md`);
      const examiner = read(`${form}-examiner-script.md`);
      for (const heading of [
        "## Reading — 90 minutes", "## Listening — 60 minutes",
        "## Writing — 90 minutes", "## Speaking — 25 minutes",
      ]) expect(paper).toContain(heading);
      expect(paper).toContain(`${form}-examiner-script.md`);
      expect(normalized(paper)).toMatch(/लेखनपरीक्षायाः पूर्वं .*पठनश्रवण/u);
      expect(normalized(paper)).toMatch(/पञ्चदश निमेषाः (?:सज्जतायै|सिद्धतायै)/u);
      expect(paper.match(/उदाहरणम् \(अङ्करहितम्\)/gu)?.length).toBeGreaterThanOrEqual(2);
      expect(renderedLearnerProse(paper)).not.toMatch(/[A-Za-z]{2,}/u);
      expect(paper).not.toMatch(/Hindi|Bengali|Malayalam/u);

      const reading = between(paper, "## Reading", "## Listening");
      expect(numbered(reading)).toBe(40);
      for (const [heading, next, minimum, maximum] of readingParts) {
        const part = between(reading, heading, next);
        expect(numbered(part), heading).toBe(10);
        const source = between(part, "#### Source", "#### Questions");
        expect(devanagariWords(source), `${heading} minimum`).toBeGreaterThanOrEqual(minimum);
        expect(devanagariWords(source), `${heading} maximum`).toBeLessThanOrEqual(maximum);
        expect(source).toContain("ऽ");
      }

      const questions = between(paper, "## Reading", "## Writing").match(/^\d+\..*$/gmu) ?? [];
      expect(questions).toHaveLength(72);
      for (const question of questions) {
        expect(question).toMatch(/[\u0900-\u097F]/u);
        expect(question).toMatch(/^\d+\..* A .* \/ B .* \/ C /u);
      }

      const listening = between(paper, "## Listening", "## Writing");
      expect(numbered(listening)).toBe(32);
      expect(listening).not.toContain("recording master");
      expect(listening).not.toContain(`${form}-l01`);

      expect(examiner).toContain("Do not give this file to the learner");
      const normalizedExaminer = normalized(examiner);
      expect(normalizedExaminer).toContain("trained speaker of the living spoken register");
      expect(normalizedExaminer).toContain("165–200 words per minute");
      expect(normalizedExaminer.toLowerCase()).toContain(
        "play every listening part once after one unscored orienting preview",
      );
      const recording = between(examiner, "## Listening recording master", "## Speaking");
      for (const [heading, next, minimum, maximum] of listeningParts) {
        const part = between(recording, heading, next);
        const script = between(part, "#### Scored spoken script");
        expect(spokenText(script), `${heading} speaker metadata`).not.toContain("<!-- speaker:");
        expect(script).not.toMatch(/^\*\*[\u0900-\u097F][^*\n]*:\*\*/gmu);
        if (heading.includes("Part 1") || heading.includes("Part 4")) {
          expect(script).toContain("<!-- speaker:");
        }
        expect(devanagariWords(part), `${heading} excludes preview and markers`)
          .toBeGreaterThan(spokenDevanagariWords(script));
        expect(spokenDevanagariWords(script), `${heading} spoken minimum`)
          .toBeGreaterThanOrEqual(minimum);
        expect(spokenDevanagariWords(script), `${heading} spoken maximum`)
          .toBeLessThanOrEqual(maximum);
      }
      expect(normalizedExaminer).toContain("two distinct actual documented traditions");
      expect(normalizedExaminer).toContain("actual tradition names");
      expect(normalizedExaminer).toContain("speaker provenance");
      expect(normalizedExaminer).toContain("reviewer confirmation");
      expect(normalizedExaminer).toContain("consent");
      expect(normalizedExaminer).toContain("recording date");
      expect(normalizedExaminer.toLowerCase()).toContain(
        "record the complete speaking paper for independent scoring",
      );

      const writing = between(paper, "## Writing", "## Speaking");
      expect(numbered(writing)).toBe(2);
      expect(writing).toContain("२२०–२८०");
      expect(writing).toContain("४२०–५२०");
      expect(writing).toContain("बाह्यसन्धिं");
      expect(writing).not.toMatch(/model answer|copyable answer model/iu);
      const synthesisSources = between(writing, "### Part 1", "#### Task");
      expect(devanagariWords(synthesisSources)).toBeGreaterThanOrEqual(750);
      expect(devanagariWords(synthesisSources)).toBeLessThanOrEqual(1000);
      const extendedBrief = between(writing, "### Part 2");
      expect(devanagariWords(extendedBrief)).toBeGreaterThanOrEqual(100);
      expect(devanagariWords(extendedBrief)).toBeLessThanOrEqual(180);

      const speaking = between(paper, "## Speaking");
      expect(numbered(speaking)).toBe(8);
      expect(speaking).toContain("३३०–३६०");
      expect(speaking).toContain("३६०–४५०");
      expect(speaking).toContain("३९०–४८०");
      expect(speaking).not.toMatch(/model answer|copyable answer model/iu);
      expect(speaking).not.toMatch(/लिखत(?:\s|[।,:;])/u);
      const presentation = between(speaking, "### Part 1", "#### Task");
      expect(devanagariWords(presentation)).toBeGreaterThanOrEqual(550);
      expect(devanagariWords(presentation)).toBeLessThanOrEqual(750);
      const negotiation = between(speaking, "### Part 2", "### Part 3");
      expect(devanagariWords(negotiation)).toBeGreaterThanOrEqual(80);
      expect(devanagariWords(negotiation)).toBeLessThanOrEqual(130);

      const speakingMaster = between(examiner, "## Speaking");
      expect(speakingMaster.match(/^#### Challenge prompt \d$/gmu)?.length).toBe(6);
      for (let index = 1; index <= 6; index += 1) {
        const next = index === 6
          ? "#### End challenge prompts"
          : `#### Challenge prompt ${index + 1}`;
        const prompt = between(speakingMaster, `#### Challenge prompt ${index}`, next);
        expect(devanagariWords(prompt), `${form} challenge ${index} 20-second minimum`)
          .toBeGreaterThanOrEqual(55);
        expect(devanagariWords(prompt), `${form} challenge ${index} 90-second maximum`)
          .toBeLessThanOrEqual(300);
      }
    }

    expect(read("mock-1.md")).not.toBe(read("mock-2.md"));
    expect(read("mock-1-examiner-script.md")).not.toBe(read("mock-2-examiner-script.md"));
  });

  it("keys every objective item without omissions or predictable labels", () => {
    for (const name of ["mock-1-answer-key.md", "mock-2-answer-key.md"]) {
      const key = read(name);
      expect(key).toMatch(/\[`?rubric\.md`?\]\(rubric\.md\)/u);
      expect(key).toContain("examiner-script.md)");
      for (const [skill, next, count, minimumPerLabel] of [
        ["Reading", "Listening", 40, 10], ["Listening", "Writing", 32, 8],
      ] as const) {
        const answers = between(key, `## ${skill}`, `## ${next}`)
          .match(/\b\d{1,2} [ABC](?=;|\.)/gu) ?? [];
        expect(answers.map((answer) => Number.parseInt(answer, 10))).toEqual(
          Array.from({ length: count }, (_, index) => index + 1),
        );
        const labels = answers.map((answer) => answer.at(-1));
        for (const label of ["A", "B", "C"]) {
          expect(labels.filter((candidate) => candidate === label).length, `${name} ${skill} ${label}`)
            .toBeGreaterThanOrEqual(minimumPerLabel);
        }
      }
      expect(key).toContain("## Writing");
      expect(key).toContain("## Speaking");
      expect(key).toContain("speaker provenance");
      expect(key).toContain("reviewer confirmation");
      expect(key).toContain("once after one unscored orienting preview");
      expect(key).toContain("documented alternatives used consistently receive full credit");
    }
  });

  it("pins one source-question-key relationship in every Mock 1 receptive part", () => {
    const regressions = [
      {
        skill: "reading", item: 1, answer: "A",
        question: "1. लेखस्य मुख्यं मतं किम्? A मानचित्रं उपयोगि, परन्तु तस्य निर्माणसीमाः निर्णयप्रयोजनञ्च प्रकाशनीयम् / B सर्वे तापसंवेदकाः निषिद्धाः / C विमानस्थानकस्य सङ्ख्यैव पर्याप्ता",
        evidence: "परन्तु मानचित्रं प्रदेशस्य प्रतिरूपं, प्रदेशः स्वयं न।",
      },
      {
        skill: "reading", item: 11, answer: "B",
        question: "11. “निष्क्रियभवनम्” इति पदं स्रोतः किमर्थं प्रश्नयति? A छदिर्न भग्ना / B अनौपचारिकाः वर्तमानोपयोगाः तेन लुप्यन्ते / C भवनं नद्यां नास्ति",
        evidence: "औपचारिकभाटकाभावः सामाजिकजीवनाभावं न सिद्धयति।",
      },
      {
        skill: "reading", item: 21, answer: "B",
        question: "21. देवदत्तस्य “जीवनस्य बीमा” इति रूपकस्य आशयः कः? A सर्वबीजस्य विक्रयः / B अग्न्यादिनाशात् ज्ञानस्य बहुस्थानीयप्रतिरक्षणम् / C कृषकस्य रोगबीमा",
        evidence: "प्रतिकृतयः बहुषु स्थानेषु रक्षिताः स्युः",
      },
      {
        skill: "reading", item: 31, answer: "A",
        question: "31. कथयितुः प्रथमसंशयः कः? A निश्चितसीमा कस्य दिनस्येति / B मानचित्रस्य मूल्यं किमिति / C कुमारस्य नाम किमिति",
        evidence: "सीमा कस्य दिनस्य?",
      },
      {
        skill: "listening", item: 1, answer: "A",
        question: "1. सभायाः तात्कालिकविषयः कः? A वर्षाकाले विद्यालयाङ्गणस्य सामुदायिकाश्रयरूपेण उपयोगः / B नूतनविद्यालयनिर्माणम् / C क्रीडापरीक्षानिरसनम्",
        evidence: "नगरस्य प्रस्तावः अस्ति यत् विद्यालयस्य आच्छादितम् अङ्गणं षड्रात्रयः सामुदायिकाश्रयरूपेण परीक्ष्येत।",
      },
      {
        skill: "listening", item: 9, answer: "B",
        question: "9. वक्तुः मुख्यः सिद्धान्तः कः? A जलवितरणं केवलं नलिकासम्बद्धम् / B सङ्कटकाले न्याय्यवितरणं पूर्वघोषितमूल्यनिर्णयं अपेक्षते / C अधिकमूल्यं सर्वदा श्रेष्ठम्",
        evidence: "जलं नलिकासु प्रवहति, निर्णयस्तु मूल्येषु प्रवहति।",
      },
      {
        skill: "listening", item: 17, answer: "B",
        question: "17. कथयिता पुरातनं रेडियोयन्त्रं प्रथमं कथं मन्यते स्म? A पारिवारिकविवादस्य मूलम् / B केवलं मौनकाष्ठपेटिका / C अत्याधुनिकयन्त्रम्",
        evidence: "बाल्ये अहं तं केवलं मौनकाष्ठपेटिकाममन्ये।",
      },
      {
        skill: "listening", item: 25, answer: "A",
        question: "25. कार्यक्रमस्य केन्द्रविषयः कः? A ग्रामोत्सवढोलस्य ध्वनिसंग्रहणविवादः / B नूतनवाद्यविक्रयः / C नृत्यप्रतियोगिता",
        evidence: "अद्य “श्रुतिसञ्चयः” कार्यक्रमे वयं दक्षिणपर्वतस्य वर्षारम्भोत्सवे वादितस्य महाढोलस्य ध्वनिसंग्रहं चर्चयामः।",
      },
    ] as const;

    const paper = read("mock-1.md");
    const examiner = read("mock-1-examiner-script.md");
    const keyText = read("mock-1-answer-key.md");
    for (const regression of regressions) {
      const key = between(
        keyText,
        `## ${regression.skill === "reading" ? "Reading" : "Listening"}`,
        `## ${regression.skill === "reading" ? "Listening" : "Writing"}`,
      );
      expect(paper).toContain(regression.question);
      expect(normalized(regression.skill === "reading" ? paper : examiner))
        .toContain(normalized(regression.evidence));
      expect(key).toMatch(new RegExp(
        `\\b${regression.item} ${regression.answer}(?=;|\\.)`, "u",
      ));
      if (regression.skill === "listening") {
        expect(normalized(paper)).not.toContain(normalized(regression.evidence));
      }
    }
  });

  it("pins one source-question-key relationship in every Mock 2 receptive part", () => {
    const regressions = [
      {
        skill: "reading", item: 6, answer: "C",
        question: "6. नूतनमानवीयपरीक्षानियमः किं करोति? A सर्वं विवेकं निषेधति / B यन्त्रनिर्णयं अपरिवर्तनीयं करोति / C विवेकस्य कारणं प्रमाणञ्च लेखबद्धं करोति",
        evidence: "नूतननियमे परिवर्तनस्य कारणवर्गः, स्थलीयप्रमाणम्, उत्तरदायी अभियन्ता, अपेक्षितहानिश्च लेख्यन्ते।",
      },
      {
        skill: "reading", item: 14, answer: "A",
        question: "14. ईशानी किमर्थं सरासरीध्वनिमात्रं न मन्यते? A सा अल्पकालिकनिद्राभङ्गघटनाः स्थानप्रतिध्वनिञ्च गोपयति / B मापकः मूल्यवान् / C निवासिनः ध्वनिं न शृण्वन्ति",
        evidence: "सरासरी अङ्कः अल्पकालिकं पात्रपातं, वाहनशृङ्गं, समूहहासञ्च लोपयति, यैः निद्रा भग्ना भवति।",
      },
      {
        skill: "reading", item: 26, answer: "C",
        question: "26. वैज्ञानिकपुनरुत्पाद्यता सहमतेः स्थानं किमर्थं न गृह्णीयात्? A सर्वज्ञानं गोपनीयम् / B सॉफ्टवेयरं नास्ति / C चित्रणानुमतिः सर्वजालपुनरुपयोगानुमतिः न भवति",
        evidence: "स्वामी चित्रणाय अनुमतिं ददाति इति सर्वजालप्रकाशनाय अनुमतिं ददाति न।",
      },
      {
        skill: "reading", item: 38, answer: "B",
        question: "38. बालकस्य तालं कथयिता कथं व्याख्यायति? A गौरवस्य विनाशः / B स्मृतिनादस्य वर्तमानसामुदायिकजीवने पुनः प्रवेशः / C समारोहस्य समाप्तिः",
        evidence: "सः नादं संग्रहालयस्य काचात् नगरस्य शरीरे पुनः आनयत्।",
      },
      {
        skill: "listening", item: 4, answer: "A",
        question: "4. छात्रप्रतिनिधेः मध्यमप्रस्तावः कः? A द्वे विलम्बितयाने सीमितसप्ताहप्रयोगे चालयितुम् / B सर्वरात्रं सेवा दातुम् / C दिवासेवां निरोद्धुम्",
        evidence: "सोमबुधशुक्रवासरे अष्टवादने तथा नववादने याने चालयाम। अष्टसप्ताहानन्तरं पश्यामः।",
      },
      {
        skill: "listening", item: 12, answer: "C",
        question: "12. वक्ता सम्भावनाविस्तारं कथं प्रस्तुतुमिच्छति? A एकं निश्चितबिन्दुम् / B केवलं विशेषज्ञसूत्रम् / C निर्णयसीमया सह बहुविकल्पविस्तारम्",
        evidence: "बहुषु युक्तकल्पेषु सङ्कटं सीमायाः उपरि चेत् क्रिया दृढा। केषुचित् एव चेत् पूर्वसावधानी तथा शीघ्रनूतनमापनम्।",
      },
      {
        skill: "listening", item: 21, answer: "A",
        question: "21. बालिकायाः प्रश्नः कं परिवर्तनं जनयति? A बीजप्रत्यावर्तनस्य अर्थं फलोत्पादनात् ज्ञानवितरणपर्यन्तं विस्तारयति / B पेटिकां पिधाति / C उत्सवं निरस्यति",
        evidence: "प्रत्यर्पणं केवलं वस्तु न, ज्ञानं तथा स्थितिं कथयितुमपि स्यात्।",
      },
      {
        skill: "listening", item: 31, answer: "A",
        question: "31. स्वीकृतमध्यममार्गः कः? A मूलं सीमितप्रयोगे, प्रतिरूपं नियमितनाट्ये, उभयोः अभिलेखः / B सर्वं पिधातुम् / C मूलं प्रतिरात्रं प्रयोजयितुम्",
        evidence: "मध्यमस्थिरं वर्षे एकवारं संयुक्तनियमे सीमितप्रयोगः। नियमितनाट्याय भारगतिसमीपप्रतिरूपम्। मूलप्रतिरूपयोः चित्रध्वनिगतिक्रमः।",
      },
    ] as const;

    const paper = read("mock-2.md");
    const examiner = read("mock-2-examiner-script.md");
    const keyText = read("mock-2-answer-key.md");
    for (const regression of regressions) {
      const key = between(
        keyText,
        `## ${regression.skill === "reading" ? "Reading" : "Listening"}`,
        `## ${regression.skill === "reading" ? "Listening" : "Writing"}`,
      );
      expect(paper).toContain(regression.question);
      expect(normalized(regression.skill === "reading" ? paper : examiner))
        .toContain(normalized(regression.evidence));
      expect(key).toMatch(new RegExp(
        `\\b${regression.item} ${regression.answer}(?=;|\\.)`, "u",
      ));
      if (regression.skill === "listening") {
        expect(normalized(paper)).not.toContain(normalized(regression.evidence));
      }
    }
  });

  it("pins Sanskrit-specific scoring while keeping readiness evidence unclaimed", () => {
    const rubric = read("rubric.md");
    const compact = normalized(rubric);
    expect(rubric).toMatch(/60\/100 on (?:\*\*)?every(?:\*\*)? paper/u);
    expect(rubric).toContain("220–280 word three-source synthesis");
    expect(compact).toContain("420–520 word contrasting extended text");
    expect(compact).toContain("330–360 second synthesis or mediation presentation");
    expect(compact).toContain("360–450 second multiparty negotiation");
    expect(compact).toContain("390–480 second sustained challenge, defence, and reformulation");
    expect(rubric).toContain("external sandhi");
    expect(compact).toContain("word-final visarga or anusvara");
    expect(compact).toContain("continuous śirorekhā");
    expect(compact).toContain("documented pronunciation tradition");
    expect(compact).toContain("Vedic accent and morphology remain outside");
    expect(compact).toContain("qualified Sanskrit recordings");
    expect(compact).toContain("independent human review");
    expect(compact).toContain("standard setting");
    expect(compact).toContain("Book-only validation cannot establish");
    expect(compact).toContain("not an external CEFR qualification");
    expect(compact).toContain("`assessment.json` is active as a source-artifact contract");
    expect(existsSync(join(root, "..", "..", "assessment.json"))).toBe(true);
  });
});
