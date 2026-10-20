import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../../src/loader.js";

const root = join(defaultCurriculumRoot(), "sanskrit", "mocks", "c1");
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
  "<!-- speaker: Arjuna -->",
  "<!-- speaker: Devayani -->",
  "<!-- speaker: Director -->",
  "<!-- speaker: Gauri -->",
  "<!-- speaker: IndustryRepresentative -->",
  "<!-- speaker: Interviewer -->",
  "<!-- speaker: Journalist -->",
  "<!-- speaker: Lecturer -->",
  "<!-- speaker: Malavika -->",
  "<!-- speaker: Mayura -->",
  "<!-- speaker: Moderator -->",
  "<!-- speaker: Narrator -->",
  "<!-- speaker: Sarala -->",
  "<!-- speaker: Vinoda -->",
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
  ["### Part 1 — dense public text", "### Part 2", 650, 800],
  ["### Part 2 — professional text", "### Part 3", 700, 850],
  ["### Part 3 — academic text", "### Part 4", 700, 880],
  ["### Part 4 — literary text", "## Listening", 750, 970],
] as const;

const listeningParts = [
  ["### Part 1 — unscripted discussion", "### Part 2", 925, 1120],
  ["### Part 2 — lecture", "### Part 3", 1110, 1280],
  ["### Part 3 — interview", "### Part 4", 1018, 1200],
  ["### Part 4 — narrative", undefined, 1018, 1200],
] as const;

describe("Sanskrit C1 timed mock source pair", () => {
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
        "## Reading — 75 minutes", "## Listening — 50 minutes",
        "## Writing — 75 minutes", "## Speaking — 22 minutes",
      ]) expect(paper).toContain(heading);
      expect(paper).toContain(`${form}-examiner-script.md`);
      expect(normalized(paper)).toMatch(/लेखनपरीक्षायाः पूर्वं .*पठनश्रवण/u);
      expect(normalized(paper)).toMatch(/पञ्चदश निमेषाः (?:सज्जतायै|सिद्धतायै)/u);
      expect(paper.match(/उदाहरणम् \(अङ्करहितम्\)/gu)?.length).toBeGreaterThanOrEqual(2);
      expect(renderedLearnerProse(paper)).not.toMatch(/[A-Za-z]{2,}/u);
      expect(paper).not.toMatch(/Hindi|Bengali|Malayalam/u);

      const reading = between(paper, "## Reading", "## Listening");
      expect(numbered(reading)).toBe(36);
      for (const [heading, next, minimum, maximum] of readingParts) {
        const part = between(paper, heading, next);
        expect(numbered(part), heading).toBe(9);
        const source = between(part, "#### Source", "#### Questions");
        expect(devanagariWords(source), `${heading} minimum`).toBeGreaterThanOrEqual(minimum);
        expect(devanagariWords(source), `${heading} maximum`).toBeLessThanOrEqual(maximum);
        expect(source).toContain("ऽ");
      }
      const receptiveQuestions = between(paper, "## Reading", "## Writing")
        .match(/^\d+\..*$/gmu) ?? [];
      expect(receptiveQuestions).toHaveLength(64);
      for (const question of receptiveQuestions) {
        expect(question).toMatch(/[\u0900-\u097F]/u);
        expect(question).toMatch(/^\d+\..* A .* \/ B .* \/ C /u);
      }

      const listening = between(paper, "## Listening", "## Writing");
      expect(numbered(listening)).toBe(28);
      expect(listening).not.toContain("recording master");
      expect(listening).not.toContain("mock-1-l01");
      expect(listening).not.toContain("mock-2-l01");

      expect(examiner).toContain("Do not give this file to the learner");
      const normalizedExaminer = normalized(examiner);
      expect(normalizedExaminer).toContain("trained speaker of the living spoken register");
      expect(normalizedExaminer).toContain("160–185 words per minute");
      expect(normalizedExaminer.toLowerCase()).toContain(
        "play every listening part once after one unscored orienting preview",
      );
      const recording = between(examiner, "## Listening recording master", "## Speaking");
      for (const [heading, next, minimum, maximum] of listeningParts) {
        const part = between(recording, heading, next);
        const script = between(part, "#### Scored spoken script");
        expect(spokenText(script), `${heading} speaker metadata`).not.toContain("<!-- speaker:");
        expect(script).not.toMatch(/^\*\*[\u0900-\u097F][^*\n]*:\*\*/gmu);
        if (heading.includes("discussion") || heading.includes("interview")) {
          expect(script).toContain("<!-- speaker:");
        }
        expect(devanagariWords(part), `${heading} excludes preview and markers`)
          .toBeGreaterThan(spokenDevanagariWords(script));
        expect(spokenDevanagariWords(script), `${heading} spoken minimum`).toBeGreaterThanOrEqual(minimum);
        expect(spokenDevanagariWords(script), `${heading} spoken maximum`).toBeLessThanOrEqual(maximum);
      }
      expect(normalizedExaminer).toContain("documented regional Sanskrit pronunciation tradition A");
      expect(normalizedExaminer).toContain("documented regional Sanskrit pronunciation tradition B");
      expect(normalizedExaminer).toContain("Before any recording is made");
      expect(normalizedExaminer).toContain("two distinct actual documented traditions");
      expect(normalizedExaminer).toContain("actual tradition names");
      expect(normalizedExaminer).toContain("speaker provenance and reviewer confirmation");
      expect(normalizedExaminer.toLowerCase()).toContain(
        "record the complete speaking paper for independent scoring",
      );

      const writing = between(paper, "## Writing", "## Speaking");
      expect(numbered(writing)).toBe(2);
      expect(writing).toContain("१८०–२२०");
      expect(writing).toContain("३००–३८०");
      expect(writing).toContain("बाह्यसन्धिं");
      expect(writing).not.toMatch(/model answer|copyable answer model/iu);
      expect(devanagariWords(between(
        writing, "### Part 1 — source synthesis", "#### Task",
      ))).toBeGreaterThanOrEqual(450);
      expect(devanagariWords(between(
        writing, "### Part 1 — source synthesis", "#### Task",
      ))).toBeLessThanOrEqual(650);
      const extendedBrief = between(writing, "### Part 2 — extended response");
      expect(devanagariWords(extendedBrief)).toBeGreaterThanOrEqual(90);
      expect(devanagariWords(extendedBrief)).toBeLessThanOrEqual(150);

      const speaking = between(paper, "## Speaking");
      expect(numbered(speaking)).toBe(8);
      expect(speaking).toContain("२७०–३००");
      expect(speaking).toContain("३००–३६०");
      expect(speaking).toContain("३३०–४२०");
      expect(speaking).not.toMatch(/model answer|copyable answer model/iu);
      expect(speaking).not.toMatch(/लिखत(?:\s|[।,:;])/u);
      const presentation = between(
        speaking, "### Part 1 — source-based presentation", "#### Task",
      );
      expect(devanagariWords(presentation)).toBeGreaterThanOrEqual(350);
      expect(devanagariWords(presentation)).toBeLessThanOrEqual(500);
      const collaboration = between(
        speaking, "### Part 2 — collaborative synthesis", "### Part 3",
      );
      expect(devanagariWords(collaboration)).toBeGreaterThanOrEqual(60);
      expect(devanagariWords(collaboration)).toBeLessThanOrEqual(100);

      const speakingMaster = between(examiner, "## Speaking");
      expect(speakingMaster.match(/^#### Challenge prompt \d$/gmu)?.length).toBe(6);
      for (let index = 1; index <= 6; index += 1) {
        const next = index === 6
          ? "#### End challenge prompts"
          : `#### Challenge prompt ${index + 1}`;
        const prompt = between(speakingMaster, `#### Challenge prompt ${index}`, next);
        expect(devanagariWords(prompt), `${form} challenge ${index} 20-second minimum`)
          .toBeGreaterThanOrEqual(62);
        expect(devanagariWords(prompt), `${form} challenge ${index} 75-second maximum`)
          .toBeLessThanOrEqual(200);
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
        ["Reading", "Listening", 36, 8], ["Listening", "Writing", 28, 6],
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
      expect(key).toContain("speaker provenance and reviewer confirmation");
      expect(key).toContain("once after one unscored orienting preview");
      expect(key).toContain("documented alternatives used consistently receive full credit");
    }
  });

  it("pins one source-question-key relationship in every receptive part", () => {
    const regressions = [
      {
        form: "mock-1", skill: "reading", item: 1, answer: "A",
        question: "1. लेखस्य मुख्यं मतं किम्? A यन्त्रपद्धतिः पारदर्शितापुनर्विचाराभ्यां सह प्रयोज्या / B सर्वः सङ्गणकीयक्रमो निषेध्यः / C शीघ्रतैव न्यायस्य प्रमाणम्",
        evidence: "परन्तु एते लाभाः तदैव सार्वजनिकाः यदा मानानि प्रकाशितानि, दत्तांशवैषम्यं परीक्षितं, मानवीयः पुनर्विचारः वास्तविकः, निजता सीमिता, परिवर्तननियमश्च पूर्वनिर्धारितः।",
      },
      {
        form: "mock-1", skill: "reading", item: 10, answer: "B",
        question: "10. प्रथमचरणस्य मुख्यं लक्ष्यं किम्? A सर्वसङ्ग्रहस्य प्रचारः / B उत्तरदायिनीं पुनरुपयोगयोग्यां पद्धतिं परीक्षितुम् / C केवलं लक्ष्यसङ्ख्यां पूरयितुम्",
        evidence: "प्रथमचरणस्य लक्ष्यं विशालप्रदर्शनं न, पुनरुपयोगयोग्यां उत्तरदायिनीं पद्धतिं परीक्षितुम्।",
      },
      {
        form: "mock-1", skill: "reading", item: 19, answer: "C",
        question: "19. अध्ययनस्य मुख्यः प्रश्नः कः? A सौरफलकस्य वर्णः / B ग्रामाणां नामानि / C ऊर्जासुलभता कार्यं लाभञ्च केषां कथं परिवर्तयति",
        evidence: "ते तु पृच्छन्ति यत् ऊर्जा कस्य कार्यं कथं परिवर्तयति, नवव्ययो भारश्च केषु पततः।",
      },
      {
        form: "mock-1", skill: "reading", item: 28, answer: "A",
        question: "28. सूचीलेखस्य दोषः कथं प्रथमं प्रकाशितः? A पटे सङ्घस्य नाम दृश्यते स्म / B गोविन्दः सूचीं लिखितवान् / C अध्यक्षः दिनाङ्कं परिवर्तितवान्",
        evidence: "दक्षिणकोणे वर्णस्याधः लघ्वक्षरैः “स्मृतिपथनाट्यसङ्घः” इति दृश्यते स्म। सूचीलेखः तत्क्षणमेव असत्यो जातः;",
      },
      {
        form: "mock-1", skill: "listening", item: 1, answer: "A",
        question: "1. आरम्भे नगरस्य मुख्यप्रस्तावः कः? A रात्रौ नूतनं लोकयानमार्गं परीक्षितुम् / B सर्वाणि यानानि निःशुल्कानि कर्तुम् / C दिवामार्गं निराकर्तुम्",
        evidence: "अतः परिवहनविभागः षड्वारपरीक्षणं प्रस्तावयति—नगरमध्ये आरभ्य चिकित्सालयम्, उद्योगक्षेत्रं, उत्तरप्रतिवेशञ्च योजयन् लघुमार्गः।",
      },
      {
        form: "mock-1", skill: "listening", item: 8, answer: "B",
        question: "8. व्याख्यानस्य मुख्यः सिद्धान्तः कः? A अङ्कीयघटिका सदैव अशुद्धा / B सार्वजनिकघटिका समयमात्रं न, सामाजिकसमन्वयमपि निर्माति / C पुरातनघटिका न संस्कार्या",
        evidence: "अतः सार्वजनिकसमयः प्राकृतिकः प्रवाहः न, संस्थाभिः साधनैश्च रचितः समन्वयः।",
      },
      {
        form: "mock-1", skill: "listening", item: 15, answer: "C",
        question: "15. बीजभाण्डारस्य प्रारम्भिकं लक्ष्यं किमासीत्? A केवलं विदेशे बीजविक्रयः / B प्रत्येकं बीजं संग्रहालये स्थापयितुम् / C स्थानीयविविधतां कृषकप्रवेशञ्च रक्षितुम्",
        evidence: "अतः स्थानीयविविधतां कृषकप्रवेशञ्च एकत्र रक्षितुं भाण्डारः कृतः।",
      },
      {
        form: "mock-1", skill: "listening", item: 22, answer: "A",
        question: "22. कथयिता तन्तुरूपाणि प्रथमं कथं मन्यते स्म? A केवलं सुन्दरालङ्काररूपेण / B कुटुम्बस्य ऋणलेखरूपेण / C यन्त्रदोषरूपेण",
        evidence: "अहं तान् केवलं सुन्दरालङ्कारान् अमन्ये।",
      },
      {
        form: "mock-2", skill: "reading", item: 5, answer: "B",
        question: "5. अपीलस्य उचितं प्रयोजनं किम्? A अप्रियं परिणामं पुनः परीक्षितुम् / B प्रक्रियादोषं समयेन सुधारयितुम् / C प्रत्येकं मतं प्रकाशयितुम्",
        evidence: "अपीलः प्रक्रियादोषस्य उपचारः, राजकीयपराजयस्य द्वितीयावसरः न।",
      },
      {
        form: "mock-2", skill: "reading", item: 13, answer: "B",
        question: "13. प्रादेशिकरूपयोः पुरातनदिशा किमर्थं तस्थौ? A अनुवादकौ कार्यं त्यक्तवन्तौ / B मूलसंशोधनस्य सूचना सर्वरूपेषु न प्राप्ता / C यन्त्रं सर्वरूपाणि नाशितवत्",
        evidence: "सम्पादनव्यवस्था केवलं मूलरूपस्य परिवर्तनं सूचितवती।",
      },
      {
        form: "mock-2", skill: "reading", item: 25, answer: "A",
        question: "25. “श्रमनिमेष” मापनस्य मुख्यसीमा का? A मानसिकसावधानतां संज्ञानभारञ्च पूर्णं न गृह्णाति / B जलपूरणं कदापि न गणयति / C समयं सर्वथा न मापयति",
        evidence: "जलपूरणस्य दश निमेषाः सरलतया गण्यन्ते; रात्रौ वृद्धस्य स्थितिं चिन्तयतः मानसिकसावधानता न गण्यते।",
      },
      {
        form: "mock-2", skill: "reading", item: 32, answer: "B",
        question: "32. कमलायाः स्मृतिः कथं प्रयुज्यते? A पक्षिसंख्यायाः अन्तिमप्रमाणरूपेण / B पुरातनजलमार्गशोधनस्य कारणरूपेण / C पुस्तिकानाशस्य कारणरूपेण",
        evidence: "कमलायाः सरोवरस्मृतिः पक्षिसंख्यायाः प्रत्यक्षप्रमाणं न, तथापि पुरातनजलमार्गं शोधयितुं कारणं ददाति।",
      },
      {
        form: "mock-2", skill: "listening", item: 4, answer: "A",
        question: "4. छात्रप्रतिनिधिः कं मध्यममार्गं सूचयति? A द्वौ नियतसायंकालौ आरक्षणेन सह परीक्षितुम् / B सर्वरात्रं प्रतिदिनमुद्घाटयितुम् / C केवलं प्रातःकालं वर्धयितुम्",
        evidence: "द्वौ नियतसायंकालौ षट्सप्ताहं परीक्षेम।",
      },
      {
        form: "mock-2", skill: "listening", item: 12, answer: "A",
        question: "12. सम्भावनाछाया कदा भ्रामिका भवेत्? A वर्गान्तरस्य अर्थः न प्रकाशितः चेत् / B वर्णः अस्ति चेत् / C मानचित्रं कागदे मुद्रितम् चेत्",
        evidence: "वर्गान्तरस्य अर्थः शीर्षके स्पष्टः भवेत्।",
      },
      {
        form: "mock-2", skill: "listening", item: 17, answer: "C",
        question: "17. मिश्रनलिकादोषं निवारयितुं मुख्योपायः कः? A केवलं विज्ञापनम् / B जलस्य सुगन्धः / C भिन्नवर्णनलिका परीक्षणं लेखबद्धनिरीक्षणञ्च",
        evidence: "पुनरुपयोगनलिका विशिष्टवर्णा, भिन्नमुखी, स्पष्टचिह्नयुक्ता च भवति। स्थापने द्वितीयः अभियन्ता संयोजनं परीक्षते। दाबपरीक्षा तथा प्रतिप्रवाहनिरोधकः भवतः। प्रत्येकं निरीक्षणं दिनाङ्केन, स्थानेन, परीक्षकेन च लिख्यते।",
      },
      {
        form: "mock-2", skill: "listening", item: 23, answer: "A",
        question: "23. वृद्धाभिनेतुः कथने मुख्यपरिवर्तनं किम्? A हास्यदृश्यं पूर्वप्रतिबन्धस्य सङ्केतरूपेण परिवर्तितम् / B नाटकं कदापि नाभिनीतम् / C गीतं विदेशात् आगतम्",
        evidence: "शान्ता अवदत् यत् पूर्वप्रतिबन्धकाले अधिकारी प्रत्यक्षराजनीतिकवाक्यं निरुध्यते स्म। अभिनेतारः करसङ्ग्राहकस्य छत्रं वारं वारं पतति इति हास्यं योजितवन्तः। दर्शकाः जानन्ति स्म यत् छत्रं शासनस्य दुर्बलाधिकारं सूचयति।",
      },
    ] as const;

    for (const regression of regressions) {
      const paper = read(`${regression.form}.md`);
      const source = regression.skill === "reading"
        ? paper
        : read(`${regression.form}-examiner-script.md`);
      const key = between(
        read(`${regression.form}-answer-key.md`),
        `## ${regression.skill === "reading" ? "Reading" : "Listening"}`,
        `## ${regression.skill === "reading" ? "Listening" : "Writing"}`,
      );
      expect(paper).toContain(regression.question);
      expect(normalized(source)).toContain(normalized(regression.evidence));
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
    expect(rubric).toContain("180–220 word source synthesis");
    expect(compact).toContain("300–380 word extended response");
    expect(compact).toContain("270–300 second source-based presentation");
    expect(compact).toContain("300–360 second collaborative synthesis");
    expect(compact).toContain("330–420 second challenge and defence");
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
