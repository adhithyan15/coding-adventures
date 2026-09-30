// Chapter payoff summaries keep the capital on the pronoun "I".
//
// A chapter's payoff summary is printed in the book under the chapter goal:
//
//   Complete the last lesson of chapter 138: I can say sells, pulls, ...
//
// The vocabulary-tranche generators built that line by splicing the chapter's
// canDo ("I can say ...") after the colon, and lowercased its first letter so it
// would read as the middle of a sentence. The rule is right for "Say ..." or
// "Name ...", and wrong for the pronoun: 2122 chapters across 21 tracks printed
// "...: i can say ...". The JSON was valid and every book compiled, so nothing
// noticed. This test does.
//
// The pattern is narrow on purpose. A bare "i" is a real word in several
// target languages (Italian "i", the plural "the"), so the test looks only
// at the English pronoun where the generator put it: first in the summary, or
// straight after a colon, and followed by one of the verbs a canDo opens with.

import { describe, expect, it } from "vitest";
import { loadTrackChapters } from "../src/loader.js";

const LOWERCASE_PRONOUN = /(?:^|: )i (?:can|am|will|know|understand)\b/;

describe("chapter payoff summaries", () => {
  it("recognises the defect it guards against, and leaves the corrected form alone", () => {
    expect(LOWERCASE_PRONOUN.test("Complete the last lesson of chapter 1: i can say x.")).toBe(true);
    expect(LOWERCASE_PRONOUN.test("Complete the last lesson of chapter 1: I can say x.")).toBe(false);
    expect(LOWERCASE_PRONOUN.test("Say i fiori and i libri.")).toBe(false);
  });

  it("never lowercase the pronoun I", () => {
    const offenders: string[] = [];
    for (const track of loadTrackChapters()) {
      for (const chapter of track.chapters ?? []) {
        const summary = chapter.payoff?.summary ?? "";
        if (LOWERCASE_PRONOUN.test(summary)) offenders.push(`${track.language} ${chapter.chapter}`);
      }
    }
    expect(offenders).toEqual([]);
  });
});
