// ---------------------------------------------------------------------------
// HL41: family and neighbour equivalents.
//
// These tests pin the three promises the panel makes:
//
//   - validation refuses data that would print wrong (a form in the wrong
//     script, a language outside the track's set, a paradigm where one form
//     belongs, a panel on a non-vocabulary lesson);
//   - the panel marks ONLY a judged `sameRoot: true`, so an unjudged pair is
//     never presented as related;
//   - only the book's view of a lesson carries the panel. The authored lesson,
//     its duration inputs and its source hash are untouched.
// ---------------------------------------------------------------------------
import { describe, expect, it } from "vitest";
import {
  equivalentsPanelMarkdown,
  validateEquivalents,
  withEquivalentsPanels,
  type ComparisonSets,
  type LessonEquivalents,
} from "../src/equivalents.js";
import { parseLesson } from "../src/parse.js";
import type { LanguageRegistry } from "../src/types.js";

const registry: LanguageRegistry = {
  version: 1,
  languages: [
    { id: "tamil", name: "Tamil", family: "Dravidian", script: "tamil", status: "active", bridges: [] },
    { id: "kannada", name: "Kannada", family: "Dravidian", script: "kannada", status: "active", bridges: [] },
    { id: "telugu", name: "Telugu", family: "Dravidian", script: "telugu", status: "active", bridges: [] },
    { id: "hindi", name: "Hindi", family: "Indo-Aryan", script: "devanagari", status: "active", bridges: [] },
    { id: "spanish", name: "Spanish", family: "Romance", script: "latin", status: "active", bridges: [] },
  ],
};

const sets: ComparisonSets = {
  version: 1,
  sets: { tamil: { family: ["kannada", "telugu"], neighbours: ["hindi"] } },
};

function lesson(id: string, type = "word"): ReturnType<typeof parseLesson> {
  return parseLesson(
    `---
schema_version: 2
id: ${id}
spine_node: HELLO
sequence: 10
chapter: 1
type: ${type}
headword: போ
romanization: pō
gloss: to go
concept_tag: VERB-GO
prerequisites: []
duration:
  max_seconds: 120
requires:
  knowledge: []
introduces:
  knowledge: []
practises:
  knowledge: []
skills: [reading]
modes: [interpretive]
strands: [meaning-input]
register: neutral
variety: general
---

# போ — to go

## Warm-up

[PAUSE 2s] Recall it.

## You'll want to know

**போ** (*pō*) means "go".

## Wrap-up Recall

[PAUSE 3s] Say it.
`,
    "tamil",
  );
}

const po: LessonEquivalents = {
  lesson: "TA-T1-po",
  english: "to go",
  equivalents: [
    { language: "kannada", form: "ಹೋಗು", romanization: "hōgu", sameRoot: true },
    { language: "telugu", form: "వెళ్ళు", romanization: "veḷḷu" },
    { language: "hindi", form: "जाना", romanization: "jānā", sameRoot: false },
  ],
  source: "test",
};

function codes(equivalents: LessonEquivalents[], lessons = [lesson("TA-T1-po")]): string[] {
  return validateEquivalents({ registry, sets, lessons, equivalents }).map((issue) => issue.code);
}

describe("validateEquivalents", () => {
  it("accepts a well-formed panel", () => {
    expect(codes([po])).toEqual([]);
  });

  it("refuses a lesson that does not exist", () => {
    expect(codes([{ ...po, lesson: "TA-T1-missing" }])).toEqual(["equivalents-unknown-lesson"]);
  });

  it("refuses a panel on a lesson that is not vocabulary", () => {
    expect(codes([po], [lesson("TA-T1-po", "grammar")])).toContain("equivalents-not-vocabulary");
  });

  it("refuses a language outside the track's comparison set", () => {
    const spanish = { ...po, equivalents: [{ language: "spanish", form: "ir" }] };
    expect(codes([spanish])).toEqual(["equivalents-language"]);
  });

  it("refuses languages out of comparison-set order", () => {
    const swapped = { ...po, equivalents: [po.equivalents[1]!, po.equivalents[0]!] };
    expect(codes([swapped])).toEqual(["equivalents-order"]);
  });

  it("refuses a form written in another language's script", () => {
    const wrong = { ...po, equivalents: [{ language: "kannada", form: "వెళ్ళు", romanization: "veḷḷu" }] };
    expect(codes([wrong])).toEqual(["equivalents-script"]);
  });

  it("refuses a paradigm where one form belongs", () => {
    const paradigm = { ...po, equivalents: [{ language: "hindi", form: "आना / आता", romanization: "ānā" }] };
    expect(codes([paradigm])).toEqual(["equivalents-form"]);
  });

  it("refuses a list where a word belongs", () => {
    const months = { ...po, equivalents: [{ language: "hindi", form: "जनवरी फ़रवरी मार्च अप्रैल मई", romanization: "janvarī" }] };
    expect(codes([months])).toEqual(["equivalents-form"]);
  });

  it("requires a romanization for a non-Latin form", () => {
    const bare = { ...po, equivalents: [{ language: "kannada", form: "ಹೋಗು" }] };
    expect(codes([bare])).toEqual(["equivalents-romanization"]);
  });

  it("reports malformed data instead of throwing", () => {
    const broken = { ...po, equivalents: "ಹೋಗು" } as unknown as LessonEquivalents;
    expect(codes([broken])).toEqual(["equivalents-shape"]);
    const nulls = { ...po, equivalents: [null] } as unknown as LessonEquivalents;
    expect(codes([nulls])).toEqual(["equivalents-shape"]);
  });

  it("refuses a pipe, which would split the panel's table row", () => {
    expect(codes([{ ...po, english: "go | come" }])).toEqual(["equivalents-pipe"]);
  });

  it("skips a malformed entry in the book instead of crashing", () => {
    const authored = lesson("TA-T1-po");
    const broken = [null, { ...po, english: 7 }] as unknown as LessonEquivalents[];
    expect(withEquivalentsPanels([authored], broken, sets, registry)).toEqual([authored]);
    expect(codes([null] as unknown as LessonEquivalents[])).toEqual(["equivalents-shape"]);
  });

  it("refuses two owner files for one lesson", () => {
    expect(codes([po, po])).toEqual(["equivalents-duplicate"]);
  });
});

describe("the panel", () => {
  it("marks only a judged same root, labels the neighbour, and ends in English", () => {
    const markdown = equivalentsPanelMarkdown(po, sets.sets.tamil!, registry);
    expect(markdown.split("\n")).toEqual([
      "**In the family, and next door**",
      "",
      "| | word | same root |",
      "|---|---|---|",
      "| Kannada | ಹೋಗು (*hōgu*) | yes |",
      "| Telugu | వెళ్ళు (*veḷḷu*) | |",
      "| Hindi (neighbour) | जाना (*jānā*) | |",
      "| English | to go | |",
    ]);
  });

  it("drops the same-root column when nothing is marked", () => {
    const unjudged = { ...po, equivalents: po.equivalents.map(({ sameRoot: _, ...rest }) => rest) };
    expect(equivalentsPanelMarkdown(unjudged, sets.sets.tamil!, registry)).toContain("| | word |\n|---|---|");
  });

  it("is added to the book's view only, after the first teaching block", () => {
    const authored = lesson("TA-T1-po");
    const [booked] = withEquivalentsPanels([authored], [po], sets, registry);
    expect(booked!.blocks[0]!.markdown).toBe(authored.blocks[0]!.markdown);
    expect(booked!.blocks[1]!.markdown).toContain("In the family, and next door");
    expect(authored.blocks[1]!.markdown).not.toContain("In the family");
    expect(booked!.sourceHash).toBe(authored.sourceHash);
    expect(booked!.body).toBe(authored.body);
  });

  it("leaves every lesson alone when there are no comparison sets", () => {
    const authored = lesson("TA-T1-po");
    expect(withEquivalentsPanels([authored], [po], undefined, registry)).toEqual([authored]);
  });
});
