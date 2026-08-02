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
