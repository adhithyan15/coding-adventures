import type { ExamInventory } from "../src/exam-inventory.js";
import { parseLesson } from "../src/parse.js";

export function fixtureLesson(id: string, introduces: string[]) {
  return parseLesson(
    `---
schema_version: 2
id: ${id}
spine_node: HELLO
sequence: 10
chapter: 1
type: grammar
headword: prueba
gloss: a fixture
concept_tag: ES-TEST
prerequisites: []
duration:
  max_seconds: 60
requires:
  knowledge: []
introduces:
  knowledge: [${introduces.join(", ")}]
practises:
  knowledge: []
skills: [reading]
modes: [interpretive]
strands: [language-focus]
register: neutral
variety: general
---

# prueba

## Warm-up

[PAUSE 2s] Recall it.
`,
    "spanish",
  );
}

export const COMPLETE_SCOPE: ExamInventory["scope"] = {
  "communicative-functions": { status: "complete", source: "fixture", note: "fixture" },
  grammar: { status: "complete", source: "fixture", note: "fixture" },
  "phonology-orthography": { status: "complete", source: "fixture", note: "fixture" },
  lexicon: { status: "complete", source: "fixture", note: "fixture" },
};

export const FIXTURE: ExamInventory = {
  version: 1,
  language: "spanish",
  level: "A1",
  about: "fixture",
  source: "fixture",
  scope: COMPLETE_SCOPE,
  probeSemantics: "fixture",
  points: [
    { id: "P-1", category: "Uno", label: "both atoms present", probe: ["ES-A", "ES-B"] },
    { id: "P-2", category: "Uno", label: "one atom missing", probe: ["ES-A", "ES-MISSING"] },
    { id: "P-3", category: "Dos", label: "nothing corresponds", probe: null },
  ],
};
