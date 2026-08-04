import { describe, expect, it } from "vitest";
import {
  activityAnswerIsCorrect,
  activityContractErrors,
  compileLessonActivities,
  normalizeActivityResponse,
} from "../src/activity.js";
import { parseBodyBlocks } from "../src/parse.js";

const DIRECTIVE = `<!-- hl-activity: {"id":"ES-G01-class-count","kind":"text","assesses":["ES-GRAMMAR-NOUN-GENDER"],"prompt":"How many noun classes does Spanish keep?","answer":"two","accepted":["2"],"feedback":{"correct":"Right: masculine and feminine.","incorrect":"Spanish keeps two noun classes."},"response_seconds":8} -->`;

describe("typed lesson activities", () => {
  it("parses compact JSON beside block knowledge and omits it from learner copy", () => {
    const [block] = parseBodyBlocks([
      "## Wrap-up Recall",
      "<!-- hl-knowledge: introduces=[]; assesses=[ES-GRAMMAR-NOUN-GENDER] -->",
      DIRECTIVE,
      "",
      "Recall the two classes.",
    ].join("\n")).blocks;

    expect(block?.markdown).toBe("Recall the two classes.");
    expect(block?.activityDirectiveErrors).toBeUndefined();
    expect(block?.activities).toEqual([{
      id: "ES-G01-class-count",
      kind: "text",
      assesses: ["ES-GRAMMAR-NOUN-GENDER"],
      prompt: "How many noun classes does Spanish keep?",
      answer: "two",
      accepted: ["2"],
      feedback: {
        correct: "Right: masculine and feminine.",
        incorrect: "Spanish keeps two noun classes.",
      },
      responseSeconds: 8,
    }]);
  });

  it("compiles answer variants once and matches normalized runtime responses", () => {
    const blocks = parseBodyBlocks([
      "## Wrap-up Recall",
      "<!-- hl-knowledge: introduces=[]; assesses=[ES-GRAMMAR-NOUN-GENDER] -->",
      DIRECTIVE,
      "Recall the two classes.",
    ].join("\n")).blocks;
    const [activity] = compileLessonActivities(blocks);

    expect(activity).toMatchObject({
      id: "ES-G01-class-count",
      blockIndex: 0,
      blockType: "recall",
      acceptedResponses: ["two", "2"],
    });
    expect(activityAnswerIsCorrect("  TWO  ", activity!)).toBe(true);
    expect(activityAnswerIsCorrect("three", activity!)).toBe(false);
    expect(normalizeActivityResponse("It’s  TWO")).toBe("it's two");
  });

  it("flags malformed, misplaced, and ambiguous authored contracts", () => {
    const [misplaced] = parseBodyBlocks([
      "## Wrap-up Recall",
      "<!-- hl-knowledge: introduces=[]; assesses=[ES-GRAMMAR-NOUN-GENDER] -->",
      "Learner copy first.",
      DIRECTIVE,
    ].join("\n")).blocks;
    expect(misplaced?.activityDirectiveErrors).toEqual([
      "activity directives must follow the first-line hl-knowledge directive before learner copy",
    ]);
    expect(misplaced?.markdown).toContain("hl-activity");

    const [malformed] = parseBodyBlocks([
      "## Wrap-up Recall",
      "<!-- hl-knowledge: introduces=[]; assesses=[ES-GRAMMAR-NOUN-GENDER] -->",
      "<!-- hl-activity: {not-json} -->",
      "Learner copy.",
    ].join("\n")).blocks;
    expect(malformed?.activityDirectiveErrors).toEqual(["contains invalid JSON"]);

    const parsed = parseBodyBlocks([
      "## Wrap-up Recall",
      "<!-- hl-knowledge: introduces=[]; assesses=[ES-GRAMMAR-NOUN-GENDER] -->",
      DIRECTIVE,
      "Learner copy.",
    ].join("\n")).blocks[0]!.activities![0]!;
    expect(activityContractErrors({ ...parsed, accepted: ["TWO"] })).toContain(
      "answer and accepted variants must resolve to unique responses",
    );
  });
});
