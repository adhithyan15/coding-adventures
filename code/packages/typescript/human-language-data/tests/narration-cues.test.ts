// The cue vocabulary and splitter, at their new home.
//
// `narration-cues.ts` was split out of `narration.ts` so the modality rules can
// read a lesson the way the narrator does without importing the narrator (which
// imports modality). Two promises come with the move, and this file holds them:
//
//   1. nothing that imported these names from `narration.ts` sees a difference —
//      the re-exports are the very same bindings, not copies that could drift;
//   2. the splitter still answers the one question the modality rule asks of it:
//      which parts of a paragraph are prose the narrator says plainly, and which
//      are cues, and of those which ones it defers.
//
// The behaviour itself is covered in depth by `narration.test.ts`,
// `delivery-cue.test.ts` and `cue-action-classification.test.ts`.

import { describe, expect, it } from "vitest";
import * as cues from "../src/narration-cues.js";
import * as narration from "../src/narration.js";

describe("narration.ts re-exports the cue module unchanged", () => {
  it.each([
    "isManualCueAction",
    "MANUAL_CUE_ACTIONS",
    "parseNarrationCue",
    "PROMPT_RESPONSE_SECONDS",
    "SPOKEN_CUE_ACTIONS",
    "splitNarrationCues",
  ] as const)("%s is the same binding", (name) => {
    expect(narration[name]).toBe(cues[name]);
  });
});

describe("splitNarrationCues, as the modality rule reads it", () => {
  it("separates prose from cues, in order", () => {
    const parts = cues.splitNarrationCues("[PAUSE 3s] Read **नमस्ते**. [YOU READ: **नमस्ते**] Then say it.");
    expect(parts.map((part) => ("text" in part ? "text" : part.cue.kind))).toEqual([
      "pause",
      "text",
      "prompt",
      "text",
    ]);
  });

  it("marks a reading cue deferred and a recall spoken", () => {
    const [read] = cues.splitNarrationCues("[YOU READ: **নাম**]");
    const [recall] = cues.splitNarrationCues("[YOU RECALL: say *āmi*]");
    expect(read && "cue" in read && read.cue.kind === "prompt" && read.cue.spoken).toBe(false);
    expect(recall && "cue" in recall && recall.cue.kind === "prompt" && recall.cue.spoken).toBe(true);
  });

  it("keeps a bracket that is not a cue as prose", () => {
    expect(cues.splitNarrationCues("a [bonjour] gloss")).toEqual([{ text: "a [bonjour] gloss" }]);
  });

  it("gives every unscored prompt the duration estimator's response window", () => {
    const [say] = cues.splitNarrationCues("[YOU SAY: hola]");
    expect(say && "cue" in say && say.cue.kind === "prompt" && say.cue.responseSeconds).toBe(
      cues.PROMPT_RESPONSE_SECONDS,
    );
  });
});
