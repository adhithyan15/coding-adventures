import { describe, expect, it } from "vitest";
import {
  acceptedMeanings,
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
});
