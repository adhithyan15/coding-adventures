import { describe, expect, it } from "vitest";
import { validateCurriculum } from "../src/curriculum.js";
import { parseLesson } from "../src/parse.js";
import type { CurriculumSpine, LanguageRegistry, Taxonomy } from "../src/types.js";

const registry: LanguageRegistry = {
  version: 1,
  languages: [{ id: "test", name: "Test", family: "Test", script: "latin", status: "active", bridges: [] }],
};
const taxonomy: Taxonomy = {
  version: 1,
  concepts: { "GREETING-HELLO": { family: "GREETING", gloss: "hello", core: true } },
};
const spine: CurriculumSpine = {
  version: 1,
  stages: ["pre-A1"],
  nodes: [{ id: "HELLO", stage: "pre-A1", canDo: "I can greet someone.", prerequisites: [], core: true, concepts: ["GREETING-HELLO"] }],
};
const source = (id: string, type: string, concept: string, prerequisites: string[]) => `---
id: ${id}
chapter: 1
type: ${type}
headword: hello
gloss: hello
concept_tag: ${concept}
prerequisites: [${prerequisites.join(", ")}]
est_minutes: 4
reviews_of: []
---

# A real body
`;

const sourceV2 = (
  id: string,
  sequence: number,
  prerequisites: string[],
  requires: string[],
  introduces: string[],
  practises: string[],
) => `---
schema_version: 2
id: ${id}
spine_node: HELLO
sequence: ${sequence}
chapter: 1
type: word
headword: hello
gloss: hello
concept_tag: GREETING-HELLO
prerequisites: [${prerequisites.join(", ")}]
duration:
  max_seconds: 120
requires:
  knowledge: [${requires.join(", ")}]
introduces:
  knowledge: [${introduces.join(", ")}]
practises:
  knowledge: [${practises.join(", ")}]
skills: [listening, speaking]
modes: [interpretive, interpersonal]
strands: [meaning-input, meaning-output]
register: neutral
variety: general
reviews_of: []
---

# A real body

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[] -->

Recall what you know.

## You'll want to know
<!-- hl-knowledge: introduces=[${introduces.join(", ")}]; assesses=[] -->

Meet the new material.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[${practises.join(", ")}] -->

Say hello.

## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[${practises.join(", ")}] -->

Say hello once.
`;

describe("curriculum prerequisite validation", () => {
  it("rejects an unknown lesson prerequisite", () => {
    const lessons = [parseLesson(source("A", "word", "GREETING-HELLO", ["MISSING"]), "test")];
    expect(validateCurriculum({ registry, taxonomy, spine, lessons }).map((issue) => issue.code))
      .toContain("unknown-lesson-prerequisite");
  });

  it("rejects a prerequisite cycle", () => {
    const lessons = [
      parseLesson(source("A", "word", "GREETING-HELLO", ["B"]), "test"),
      parseLesson(source("B", "practice", "TEST-PRACTICE", ["A"]), "test"),
    ];
    expect(validateCurriculum({ registry, taxonomy, spine, lessons }).map((issue) => issue.code))
      .toContain("lesson-prerequisite-cycle");
  });
});

describe("schema-v2 lesson validation", () => {
  it("accepts a dependency-ordered transitive knowledge chain", () => {
    const lessons = [
      parseLesson(sourceV2("A", 10, [], [], ["TEST-LEX-HELLO"], ["TEST-LEX-HELLO"]), "test"),
      parseLesson(
        sourceV2(
          "B",
          20,
          ["A"],
          ["TEST-LEX-HELLO"],
          ["TEST-GRAMMAR-GREETING"],
          ["TEST-LEX-HELLO", "TEST-GRAMMAR-GREETING"],
        ),
        "test",
      ),
      parseLesson(
        sourceV2(
          "C",
          30,
          ["B"],
          ["TEST-LEX-HELLO", "TEST-GRAMMAR-GREETING"],
          [],
          ["TEST-LEX-HELLO"],
        ),
        "test",
      ),
    ];
    expect(validateCurriculum({ registry, taxonomy, spine, lessons }).filter((issue) => issue.level === "error"))
      .toEqual([]);
  });

  it("rejects required and practised knowledge that is not available", () => {
    const lessons = [
      parseLesson(
        sourceV2(
          "A",
          10,
          [],
          ["TEST-LEX-FUTURE"],
          ["TEST-LEX-HELLO"],
          ["TEST-GRAMMAR-FUTURE"],
        ),
        "test",
      ),
    ];
    const codes = validateCurriculum({ registry, taxonomy, spine, lessons }).map((issue) => issue.code);
    expect(codes).toContain("schema-v2-knowledge-not-closed");
    expect(codes).toContain("schema-v2-practice-before-introduction");
  });

  it("rejects block assessments before their atoms reach the block frontier", () => {
    const premature = sourceV2(
      "A",
      10,
      [],
      [],
      ["TEST-LEX-HELLO"],
      ["TEST-LEX-HELLO"],
    ).replace(
      "<!-- hl-knowledge: introduces=[]; assesses=[] -->",
      "<!-- hl-knowledge: introduces=[]; assesses=[TEST-LEX-HELLO] -->",
    );
    const codes = validateCurriculum({
      registry,
      taxonomy,
      spine,
      lessons: [parseLesson(premature, "test")],
    }).map((issue) => issue.code);
    expect(codes).toContain("schema-v2-block-knowledge-not-closed");
  });

  it("rejects assessed atoms omitted from lesson-level practice declarations", () => {
    const undeclared = sourceV2("A", 10, [], [], ["TEST-LEX-HELLO"], ["TEST-LEX-HELLO"])
      .replace(
        "assesses=[TEST-LEX-HELLO]",
        "assesses=[TEST-LEX-HELLO, TEST-LEX-UNDECLARED]",
      );
    const codes = validateCurriculum({
      registry,
      taxonomy,
      spine,
      lessons: [parseLesson(undeclared, "test")],
    }).map((issue) => issue.code);
    expect(codes).toContain("schema-v2-block-undeclared-assessment");
    expect(codes).toContain("schema-v2-block-knowledge-not-closed");
  });

  it("requires every schema-v2 body block to declare its knowledge boundary", () => {
    const missing = sourceV2("A", 10, [], [], ["TEST-LEX-HELLO"], ["TEST-LEX-HELLO"])
      .replace("<!-- hl-knowledge: introduces=[]; assesses=[] -->\n", "");
    expect(validateCurriculum({
      registry,
      taxonomy,
      spine,
      lessons: [parseLesson(missing, "test")],
    }).map((issue) => issue.code)).toContain("schema-v2-missing-block-knowledge");
  });

  it("requires production and recall blocks to name what they assess", () => {
    const empty = sourceV2("A", 10, [], [], ["TEST-LEX-HELLO"], ["TEST-LEX-HELLO"])
      .replaceAll("assesses=[TEST-LEX-HELLO]", "assesses=[]");
    const codes = validateCurriculum({
      registry,
      taxonomy,
      spine,
      lessons: [parseLesson(empty, "test")],
    }).map((issue) => issue.code);
    expect(codes).toContain("schema-v2-empty-block-assessment");
    expect(codes).toContain("schema-v2-block-assessment-missing");
  });

  it("requires block introductions to exactly account for lesson introductions", () => {
    const base = sourceV2("A", 10, [], [], ["TEST-LEX-HELLO"], ["TEST-LEX-HELLO"]);
    const missing = base.replace(
      "introduces=[TEST-LEX-HELLO]",
      "introduces=[]",
    );
    expect(validateCurriculum({
      registry,
      taxonomy,
      spine,
      lessons: [parseLesson(missing, "test")],
    }).map((issue) => issue.code)).toContain("schema-v2-block-introduction-missing");

    const undeclaredAndDuplicate = base.replace(
      "introduces=[]; assesses=[]",
      "introduces=[TEST-LEX-HELLO, TEST-LEX-UNDECLARED]; assesses=[]",
    );
    const codes = validateCurriculum({
      registry,
      taxonomy,
      spine,
      lessons: [parseLesson(undeclaredAndDuplicate, "test")],
    }).map((issue) => issue.code);
    expect(codes).toContain("schema-v2-block-undeclared-introduction");
    expect(codes).toContain("schema-v2-duplicate-block-introduction");
  });

  it("rejects a malformed authored block-knowledge directive", () => {
    const malformed = sourceV2("A", 10, [], [], ["TEST-LEX-HELLO"], ["TEST-LEX-HELLO"])
      .replace(
        "<!-- hl-knowledge: introduces=[]; assesses=[] -->",
        "<!-- hl-knowledge: assesses=[] -->",
      );
    expect(validateCurriculum({
      registry,
      taxonomy,
      spine,
      lessons: [parseLesson(malformed, "test")],
    }).map((issue) => issue.code)).toContain("schema-v2-invalid-block-knowledge");
  });

  it("rejects malformed duration, coverage, sequence, and body blocks", () => {
    const malformed = sourceV2("A", 10, [], [], ["TEST-LEX-HELLO"], [])
      .replace("sequence: 10", "sequence: 0")
      .replace("max_seconds: 120", "max_seconds: 300")
      .replace("skills: [listening, speaking]", "skills: [guessing]")
      .replace("## Warm-up", "## Surprise")
      .replace("## Wrap-up Recall", "## Guided Practice");
    const lessons = [parseLesson(malformed, "test")];
    const codes = validateCurriculum({ registry, taxonomy, spine, lessons }).map((issue) => issue.code);
    expect(codes).toEqual(expect.arrayContaining([
      "schema-v2-invalid-sequence",
      "schema-v2-invalid-duration",
      "schema-v2-duration-budget",
      "schema-v2-unknown-coverage",
      "schema-v2-first-block",
      "schema-v2-last-block",
      "schema-v2-unknown-block",
    ]));
  });

  it("rejects a prerequisite that appears later in the authored sequence", () => {
    const lessons = [
      parseLesson(sourceV2("A", 20, ["B"], ["TEST-LEX-B"], ["TEST-LEX-A"], []), "test"),
      parseLesson(sourceV2("B", 30, [], [], ["TEST-LEX-B"], []), "test"),
    ];
    expect(validateCurriculum({ registry, taxonomy, spine, lessons }).map((issue) => issue.code))
      .toContain("schema-v2-prerequisite-order");
  });

  it("rejects an unknown authored schema version", () => {
    const lesson = parseLesson(
      source("A", "word", "GREETING-HELLO", []).replace("est_minutes: 4", "schema_version: 9"),
      "test",
    );
    expect(validateCurriculum({ registry, taxonomy, spine, lessons: [lesson] }).map((issue) => issue.code))
      .toContain("unknown-lesson-schema-version");
  });
});
