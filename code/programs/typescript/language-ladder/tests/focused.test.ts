import { describe, expect, it } from "vitest";
import {
  acceptedMeanings,
  activityAnswerIsCorrect,
  focusedActivity,
  focusedCheckKind,
  meaningAnswerIsCorrect,
  normalizeFocusedAnswer,
} from "../src/focused";

describe("focused retrieval checks", () => {
  it("accepts a full gloss or one top-level alternative", () => {
    expect(acceptedMeanings("hello / peace (formal note)")).toEqual([
      "hello peace",
      "hello",
      "peace",
    ]);
    expect(meaningAnswerIsCorrect("Peace", "hello / peace (formal note)")).toBe(true);
    expect(meaningAnswerIsCorrect("goodbye", "hello / peace (formal note)")).toBe(false);
  });

  it("normalizes case, punctuation, apostrophes, and accents", () => {
    expect(normalizeFocusedAnswer("  YOU’RE-wélcome! ")).toBe("youre welcome");
  });

  it("uses objective meaning checks only for lexical lessons", () => {
    expect(focusedCheckKind({ type: "word", gloss: "hello" })).toBe("meaning");
    expect(focusedCheckKind({ type: "phrase", gloss: "my name is" })).toBe("meaning");
    expect(focusedCheckKind({ type: "writing", gloss: "the first joined shape" }))
      .toBe("self-check");
  });

  it("prefers an authored recall activity over lexical inference or self-confirmation", () => {
    const activity = {
      id: "ES-G01-count",
      kind: "text" as const,
      assesses: ["ES-GRAMMAR-NOUN-GENDER"],
      prompt: "How many classes?",
      answer: "two",
      accepted: ["2"],
      feedback: { correct: "Right.", incorrect: "Try again." },
      responseSeconds: 8,
      blockIndex: 3,
      blockType: "recall" as const,
      blockTitle: "Wrap-up Recall",
      acceptedResponses: ["two", "2"],
    };
    const lesson = { type: "grammar", gloss: "noun gender", activities: [activity] };
    expect(focusedCheckKind(lesson)).toBe("activity");
    expect(focusedActivity(lesson)).toBe(activity);
    expect(activityAnswerIsCorrect(" 2 ", activity)).toBe(true);
  });
});
