import { describe, expect, it } from "vitest";
import {
  indicesByLanguage,
  languageFromPath,
  languagesOf,
  loadLessons,
  toLesson,
  nextDue,
  type Lesson,
} from "../src/lessons.ts";
import { buildPool } from "../src/interleave.ts";
import { parseLesson } from "@coding-adventures/human-language-data/src/parse.ts";

const SPANISH = `---
id: ES-C17-futuro
chapter: 17
type: word
headword: hablaré
gloss: the future
concept_tag: ES-FUTURE
prerequisites: [ES-C16-practice]
reviews_of: [ES-C16-practice, ES-C06-hablar]
---

# body
`;

const TAMIL = `---
id: TA-C06-dative-ukku
chapter: 6
type: word
headword: -உக்கு
gloss: to / for
concept_tag: TA-CASE-DATIVE
prerequisites: []
reviews_of: []
---

# body
`;

const NO_ID = `---
chapter: 1
type: word
headword: x
gloss: y
---

# body
`;

describe("languageFromPath", () => {
  it("pulls the track out of a curriculum path", () => {
    expect(
      languageFromPath("../../../../learning/human-languages/spanish/lessons/ES-C1.md"),
    ).toBe("spanish");
  });

  it("returns empty for a path that isn't a lesson", () => {
    expect(languageFromPath("../../elsewhere/notes.md")).toBe("");
    expect(languageFromPath("/human-languages/spanish/roadmap.md")).toBe("");
  });
});

describe("toLesson", () => {
  it("carries the id, concept and both graphs through", () => {
    const lesson = toLesson(parseLesson(SPANISH, "spanish"));
    expect(lesson).not.toBeNull();
    expect(lesson!.id).toBe("ES-C17-futuro");
    expect(lesson!.language).toBe("spanish");
    expect(lesson!.concept).toBe("ES-FUTURE");
    expect(lesson!.chapter).toBe(17);
    expect(lesson!.prerequisites).toEqual(["ES-C16-practice"]);
    expect(lesson!.reviewsOf).toEqual(["ES-C16-practice", "ES-C06-hablar"]);
  });

  it("skips a lesson with no id rather than inventing one", () => {
    expect(toLesson(parseLesson(NO_ID, "spanish"))).toBeNull();
  });

  it("handles a non-Latin headword unchanged", () => {
    const lesson = toLesson(parseLesson(TAMIL, "tamil"));
    expect(lesson!.headword).toBe("-உக்கு");
    expect(lesson!.id).toBe("TA-C06-dative-ukku");
  });
});

describe("loadLessons", () => {
  const sources = {
    "../../../../learning/human-languages/spanish/lessons/ES-C17-futuro.md": SPANISH,
    "../../../../learning/human-languages/tamil/lessons/TA-C06-dative-ukku.md": TAMIL,
    "../../../../learning/human-languages/spanish/lessons/broken.md": NO_ID,
    "../../somewhere/else.md": SPANISH,
  };

  it("parses every well-formed lesson and drops the rest", () => {
    const lessons = loadLessons(sources);
    expect(lessons.map((l) => l.id)).toEqual(["ES-C17-futuro", "TA-C06-dative-ukku"]);
  });

  it("is sorted by id, so indices are deterministic across builds", () => {
    const ids = loadLessons(sources).map((l) => l.id);
    expect([...ids].sort()).toEqual(ids);
  });
});

/** A Lesson with everything defaulted, so a test only states what it cares about. */
export function lesson(over: Partial<Lesson> & { id: string }): Lesson {
  return {
    language: "spanish",
    headword: "",
    gloss: "",
    type: "word",
    chapter: 1,
    concept: "",
    prerequisites: [],
    reviewsOf: [],
    romanization: "",
    script: "latin",
    etymologyHook: "",
    ...over,
  };
}

describe("grouping for interleaving", () => {
  const lessons: Lesson[] = [
    lesson({ id: "A1", language: "spanish", chapter: 1 }),
    lesson({ id: "B1", language: "tamil", chapter: 1 }),
    lesson({ id: "A2", language: "spanish", chapter: 2 }),
  ];

  it("lists the distinct languages, sorted", () => {
    expect(languagesOf(lessons)).toEqual(["spanish", "tamil"]);
  });

  it("groups indices by language, preserving order within each", () => {
    // spanish first (sorted): indices 0 and 2; tamil: index 1.
    expect(indicesByLanguage(lessons)).toEqual([[0, 2], [1]]);
  });

  it("produces groups the interleaver can round-robin over", () => {
    // Exercise the actual remap pickLesson relies on: pool entry →
    // groups[scriptIndex][letterIndex] → lesson index.
    const groups = indicesByLanguage(lessons);
    const pool = buildPool(groups.map((g) => g.length));
    const order = pool.map((e) => groups[e.scriptIndex]![e.letterIndex]!);
    // Round-robin: spanish(0), tamil(1), then spanish's second (2).
    expect(order).toEqual([0, 1, 2]);
    expect(order.map((i) => lessons[i]!.language)).toEqual([
      "spanish",
      "tamil",
      "spanish",
    ]);
  });
});

describe("nextDue", () => {
  // Three languages, one lesson each → the pool alternates across all three.
  const groups = [[0], [1], [2]];
  const pool = buildPool(groups.map((g) => g.length));
  const allDue = [
    { dueAtSession: 0 },
    { dueAtSession: 0 },
    { dueAtSession: 0 },
  ];

  it("advances the cursor so it never repeats the item just answered", () => {
    let cursor = -1;
    const picked: number[] = [];
    for (let i = 0; i < 3; i++) {
      const r = nextDue(groups, pool, allDue, 0, cursor);
      picked.push(r.index!);
      cursor = r.cursor;
    }
    // Every language in turn — the whole point of the cursor.
    expect(picked).toEqual([0, 1, 2]);
  });

  it("wraps around the pool", () => {
    const r = nextDue(groups, pool, allDue, 0, pool.length - 1);
    expect(r.index).toBe(0);
    expect(r.cursor).toBe(0);
  });

  it("skips items that are not yet due", () => {
    const schedule = [
      { dueAtSession: 99 }, // not due
      { dueAtSession: 0 }, // due
      { dueAtSession: 99 }, // not due
    ];
    expect(nextDue(groups, pool, schedule, 0, -1).index).toBe(1);
  });

  it("returns null when nothing is due, without looping forever", () => {
    const none = [{ dueAtSession: 5 }, { dueAtSession: 5 }, { dueAtSession: 5 }];
    expect(nextDue(groups, pool, none, 0, -1).index).toBeNull();
  });

  it("handles an empty pool", () => {
    expect(nextDue([], [], [], 0, -1)).toEqual({ index: null, cursor: -1 });
  });

  it("copes with ragged groups", () => {
    // spanish has 3 lessons, tamil 1 — buildPool goes ragged after round 1.
    const ragged = [[0, 2, 3], [1]];
    const raggedPool = buildPool(ragged.map((g) => g.length));
    const schedule = [
      { dueAtSession: 0 },
      { dueAtSession: 0 },
      { dueAtSession: 0 },
      { dueAtSession: 0 },
    ];
    let cursor = -1;
    const picked: number[] = [];
    for (let i = 0; i < 4; i++) {
      const r = nextDue(ragged, raggedPool, schedule, 0, cursor);
      picked.push(r.index!);
      cursor = r.cursor;
    }
    // Alternates while both have items, then drains the longer group.
    expect(picked).toEqual([0, 1, 2, 3]);
  });
});
