import { describe, expect, it } from "vitest";
import {
  LANGUAGE_CURRICULA,
  LANGUAGE_ORDER,
  SPINE_NODES,
  curriculumForLanguage,
  mappedLessonIds,
  mixedCurriculumFrontier,
  spineNodeById,
} from "../src/curriculum";

describe("per-language shared-spine maps", () => {
  it("bundles one complete map for every active language", () => {
    expect(LANGUAGE_CURRICULA.map((curriculum) => curriculum.language)).toEqual(LANGUAGE_ORDER);
    // Derived from the registry: registering a track must not require editing
    // this file. The assertion above already pins that the two agree in order.
    expect(LANGUAGE_CURRICULA).toHaveLength(LANGUAGE_ORDER.length);
    for (const curriculum of LANGUAGE_CURRICULA) {
      expect(Object.keys(curriculum.spine)).toEqual(SPINE_NODES.map((node) => node.id));
    }
  });

  it("keeps repeated local visits and explicit relocations", () => {
    const spanish = curriculumForLanguage("spanish")!;
    expect(spanish.spine["SPINE-MEET-GREET"]?.segments.length).toBeGreaterThan(1);
    expect(spanish.spine["SPINE-TAKE-LEAVE"]?.relocates["GREETING-GOODNIGHT"])
      .toBe("SPINE-TIME-OF-DAY");
  });

  it("resolves a shared ability by its stable node id", () => {
    expect(spineNodeById("SPINE-MEET-GREET")?.canDo).toContain("greeting");
    expect(spineNodeById("MISSING")).toBeUndefined();
  });

  it("makes Persian and Urdu script introduction an inline local extension", () => {
    for (const language of ["persian", "urdu"]) {
      const curriculum = curriculumForLanguage(language)!;
      const script = curriculum.extensions.find((extension) => extension.category === "script");
      expect(script?.kind).toBe("required");
      expect(script?.lessons).toHaveLength(1);
      const segment = curriculum.path.find((item) => item.inline.includes(script!.id));
      expect(segment?.spine_node).toBe("SPINE-MEET-GREET");
    }
  });

  it("exposes only mapped lessons for a selected mix", () => {
    const ids = mappedLessonIds(["persian", "urdu"]);
    expect(ids).toEqual(new Set([
      "FA-C01-salam",
      "FA-C01-mamnoon",
      "FA-C01-bale",
      "FA-C01-na",
      "FA-C02-esm-e-man",
      "FA-C03-shoma-to",
      "FA-C03-chist",
      "FA-C03-esm-e-shoma-chist",
      "FA-C03-khoshvaghtam",
      "FA-C03-practice",
      "FA-C04-hal",
      "FA-C04-chetor",
      "FA-C04-hal-e-shoma-chetor-ast",
      "FA-C04-khub",
      "FA-C04-khubam",
      "FA-C04-practice",
      "FA-C05-khoda",
      "FA-C05-hafez",
      "FA-C05-khodahafez",
      "FA-C05-practice",
      "UR-C01-salam",
      "UR-C01-shukriya",
      "UR-C01-ji-han",
      "UR-C01-nahin",
      "UR-C02-mera-naam",
      "UR-C03-aap-tum-tu",
      "UR-C03-kya",
      "UR-C03-aap-ka-naam-kya-hai",
      "UR-C03-khushi-hui",
      "UR-C03-practice",
      "UR-C04-kaise-kaisi",
      "UR-C04-aap-kaise-hain",
      "UR-C04-main-hun",
      "UR-C04-thik",
      "UR-C04-main-thik-hun",
      "UR-C04-practice",
      "UR-C05-khuda",
      "UR-C05-hafiz",
      "UR-C05-khuda-hafiz",
      "UR-C05-practice",
          // Persian chapter 6 and Urdu chapter 6, the core-verb tranches. Both attach to
      // SPINE-SAY-WHAT-I-DO, so they are mapped and must appear here.
      "FA-C06-budan",
      "FA-C06-raftan",
      "FA-C06-amadan",
      "FA-C06-goftan",
      "FA-C06-danestan",
      // Chapters 7-8: the eight core verbs (HL-C46). Persian's three verb shapes —
      // compound, -idan with a predictable stem, and inherited with a stem you must
      // be told — are what these two chapters exist to separate.
      "FA-C07-fekr-kardan",
      "FA-C07-fahmidan",
      "FA-C07-khandan",
      "FA-C07-neveshtan",
      "FA-C08-gereftan",
      "FA-C08-porsidan",
      "FA-C08-komak-kardan",
      "FA-C08-dust-dashtan",
      // Vocabulary wave 5: chapters 9-11, 12 pre-A1 nouns closing the last unrealized
      // pre-A1 spine node (SPINE-POLITE-REQUEST-REPAIR).
      "FA-C09-ab",
      "FA-C09-chay",
      "FA-C09-kelid",
      "FA-C09-nan",
      "FA-C10-baradar",
      "FA-C10-dokhtar",
      "FA-C10-madar",
      "FA-C10-pedar",
      "FA-C11-cheshm",
      "FA-C11-dast",
      "FA-C11-pa",
      "FA-C11-zaban",
      // Vocabulary wave 6, round 2: chapters 12-14, 14 more pre-A1 nouns.
      "FA-C12-nam",
      "FA-C12-del",
      "FA-C12-dar",
      "FA-C12-ketab",
      "FA-C13-aseman",
      "FA-C13-khorshid",
      "FA-C13-mah",
      "FA-C13-setare",
      "FA-C13-baran",
      "FA-C14-khahar",
      "FA-C14-pesar",
      "FA-C14-mard",
      "FA-C14-zan",
      "FA-C14-dust",
      "FA-W15-alef",
      "FA-W15-lam",
      "FA-W15-sin",
      "FA-W15-mim",
      "FA-W15-joining",
      "FA-W15-be",
      "FA-W15-te-nun",
      "FA-W15-he",
      "FA-W15-vav",
      "FA-C15-practice",
      "UR-C06-hona",
      "UR-C06-jana",
      "UR-C06-ana",
      "UR-C06-bolna",
      "UR-C06-janna",
      // Chapters 7-8: the same eight core verbs, reached through Urdu's own
      // Persian/Arabic layer — madad is Arabic, pasand Persian, and the nastaliq
      // script makes that lineage visible on the page.
      "UR-C07-sochna",
      "UR-C07-samajhna",
      "UR-C07-parhna",
      "UR-C07-likhna",
      "UR-C08-lena",
      "UR-C08-puchhna",
      "UR-C08-madad",
      "UR-C08-pasand",
      // Vocabulary wave 4: chapters 9-12, 13 pre-A1 nouns realizing the last three
      // unrealized spine nodes (EXCHANGE-NAMES, CHECK-WELLBEING, POLITE-REQUEST-REPAIR).
      "UR-C09-dost",
      "UR-C09-khandan",
      "UR-C09-bhai",
      "UR-C09-bahan",
      "UR-C10-aankh",
      "UR-C10-kaan",
      "UR-C10-naak",
      "UR-C10-munh",
      "UR-C11-dil",
      "UR-C12-pani",
      "UR-C12-doodh",
      "UR-C12-chai",
      "UR-C12-roti",
      // Vocabulary wave 6, round 2: chapters 13-15, 13 more pre-A1 nouns.
      "UR-C13-lal",
      "UR-C13-safed",
      "UR-C13-kala",
      "UR-C13-nila",
      "UR-C14-qamiz",
      "UR-C14-juta",
      "UR-C14-topi",
      "UR-C14-koat",
      "UR-C15-barish",
      "UR-C15-dhoop",
      "UR-C15-hawa",
      "UR-C15-garmi",
      "UR-C15-sardi",
]));
  });

  it("computes independent next steps before grouping shareable abilities", () => {
    const progress = new Map<string, ReadonlySet<string>>([
      ["persian", new Set(["FA-C01-salam"])],
      ["urdu", new Set()],
    ]);
    const frontier = mixedCurriculumFrontier(["persian", "urdu"], progress);
    expect(frontier.steps.map((step) => [step.language, step.lessonId])).toEqual([
      ["persian", "FA-C01-mamnoon"],
      ["urdu", "UR-C01-salam"],
    ]);
    expect(frontier.bySpineNode.get("SPINE-COURTESY-THANK")?.map((step) => step.language))
      .toEqual(["persian"]);
    expect(frontier.bySpineNode.get("SPINE-MEET-GREET")?.map((step) => step.language))
      .toEqual(["urdu"]);
  });
});
