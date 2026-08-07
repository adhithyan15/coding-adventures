import { describe, expect, it } from "vitest";
import type { LanguageCurriculum } from "@coding-adventures/human-language-data/src/types.ts";
import type { Lesson } from "../src/lessons";
import {
  LEARN_PROGRESS_SCHEMA_VERSION,
  LEARN_PROGRESS_STORAGE_KEY,
  completeFrontierLesson,
  eligibleReviewGrid,
  fromSavedLearnProgress,
  loadLearnProgress,
  mixedReviewReady,
  saveLearnProgress,
  toSavedLearnProgress,
} from "../src/learnprogress";

const curriculum = (language: string): LanguageCurriculum => ({
  version: 1,
  language,
  path: [{
    id: `${language}-path`,
    spine_node: "GREET",
    lessons: [`${language}-a`, `${language}-b`, `${language}-support`],
    before: [],
    inline: [`${language}-extension`],
    after: [],
  }],
  spine: { GREET: { segments: [`${language}-path`], omits: [], relocates: {} } },
  extensions: [{
    id: `${language}-extension`,
    stage: "pre-A1",
    kind: "supporting",
    category: "grammar",
    canDo: "I can use the support step.",
    prerequisites: [],
    lessons: [`${language}-support`],
  }],
});

const lesson = (id: string, language: string, concept: string): Lesson => ({
  id,
  language,
  headword: id,
  gloss: `meaning ${id}`,
  type: "word",
  chapter: 1,
  concept,
  prerequisites: [],
  reviewsOf: [],
  roots: [],
  romanization: id,
  script: "latin",
  etymologyHook: "",
  body: "",
  activities: [],
  estMinutes: 3,
});

describe("per-language Learn progress", () => {
  it("keeps only prerequisite-safe prefixes from untrusted saved ids", () => {
    const fa = curriculum("fa");
    const restored = fromSavedLearnProgress({
      version: LEARN_PROGRESS_SCHEMA_VERSION,
      completed: [["fa", ["fa-a", "fa-support"]], ["unknown", ["x"]]],
    }, [fa]);
    expect([...restored.get("fa") ?? []]).toEqual(["fa-a"]);
    expect(restored.has("unknown")).toBe(false);
  });

  it("completes only the current local frontier", () => {
    const fa = curriculum("fa");
    const skipped = completeFrontierLesson(new Map(), fa, "fa-b");
    expect(skipped.changed).toBe(false);

    const first = completeFrontierLesson(new Map(), fa, "fa-a");
    expect(first.changed).toBe(true);
    expect([...first.completion.get("fa") ?? []]).toEqual(["fa-a"]);

    const second = completeFrontierLesson(first.completion, fa, "fa-b");
    expect([...second.completion.get("fa") ?? []]).toEqual(["fa-a", "fa-b"]);
  });

  it("admits only independently completed prefixes to mixed review", () => {
    const fa = curriculum("fa");
    const ur = curriculum("ur");
    const lessons = [
      lesson("fa-a", "fa", "HELLO"),
      lesson("fa-b", "fa", "THANKS"),
      lesson("fa-support", "fa", ""),
      lesson("ur-a", "ur", "HELLO"),
      lesson("ur-b", "ur", "THANKS"),
      lesson("ur-support", "ur", ""),
    ];
    const completion = new Map<string, ReadonlySet<string>>([
      ["fa", new Set(["fa-a", "fa-b"])],
      ["ur", new Set(["ur-b"])],
    ]);
    expect(eligibleReviewGrid(lessons, [fa, ur], ["ur", "fa"], completion)
      .map((cell) => cell.lesson.id)).toEqual(["fa-a", "fa-b"]);
  });

  it("waits for two visually distinct eligible answers before mixing", () => {
    const shared = lesson("fa-a", "fa", "HELLO");
    const identical = { ...lesson("ur-a", "ur", "HELLO"), headword: shared.headword };
    expect(mixedReviewReady([
      { concept: "HELLO", language: "fa", lesson: shared },
      { concept: "HELLO", language: "ur", lesson: identical },
    ])).toBe(false);
    expect(mixedReviewReady([
      { concept: "HELLO", language: "fa", lesson: shared },
      { concept: "THANKS", language: "ur", lesson: lesson("ur-b", "ur", "THANKS") },
    ])).toBe(true);
  });

  it("round-trips stable ids and fails closed on corrupt storage", () => {
    const fa = curriculum("fa");
    const completion = new Map<string, ReadonlySet<string>>([
      ["fa", new Set(["fa-a", "fa-b"])],
    ]);
    const saved = toSavedLearnProgress(completion, [fa]);
    expect(saved.completed).toEqual([["fa", ["fa-a", "fa-b"]]]);

    const data = new Map<string, string>();
    const store = {
      getItem: (key: string) => data.get(key) ?? null,
      setItem: (key: string, value: string) => void data.set(key, value),
    };
    expect(saveLearnProgress(store, completion, [fa])).toBe(true);
    expect([...loadLearnProgress(store, [fa]).get("fa") ?? []]).toEqual(["fa-a", "fa-b"]);
    data.set(LEARN_PROGRESS_STORAGE_KEY, "not-json");
    expect(loadLearnProgress(store, [fa])).toEqual(new Map());
  });
});
