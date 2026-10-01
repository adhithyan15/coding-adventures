// Chapter titles and goals never name a bare article.
//
// The vocabulary-tranche generators build a chapter's title and its canDo goal
// from the glosses of its five words. Each gloss is cut at its first comma or
// bracket, and the cut-off part becomes the word's name:
//
//   "a companion, a friend"      ->  "A companion"
//   "a sack"                     ->  "A sack"
//   "a (girl's) friend"          ->  "A"            <- the defect
//
// A gloss that brackets its own noun leaves only the article. Hindi chapter
// 254 shipped as "A, A companion, A relative, A mother-in-law, A father-in-law",
// its goal read "I can say a, a companion, ...", and three lessons asked the
// learner to "say the Hindi for a". The JSON was valid and the book compiled,
// so nothing noticed. This test does.
//
// It looks at the two places the cut-off name surfaces in chapter metadata:
// the comma-separated title, and the list after "I can say" in the goal.

import { describe, expect, it } from "vitest";
import { loadTrackChapters } from "../src/loader.js";

const ARTICLES = new Set(["a", "an", "the"]);

/** The comma-separated parts of a title that are only an article. */
function stubTitleParts(title: string): string[] {
  return title
    .split(",")
    .map((part) => part.trim())
    .filter((part) => ARTICLES.has(part.toLowerCase()));
}

/** The items of an "I can say x, y and z." goal that are only an article. */
function stubGoalItems(canDo: string): string[] {
  const match = /^I can say (.*)\.$/.exec(canDo.trim());
  if (!match) return [];
  return match[1]!
    .split(/, | and /)
    .map((item) => item.trim())
    .filter((item) => ARTICLES.has(item.toLowerCase()));
}

describe("chapter titles and goals", () => {
  it("recognises the defect it guards against, and leaves the corrected form alone", () => {
    expect(stubTitleParts("A, A companion, A relative")).toEqual(["A"]);
    expect(stubTitleParts("A female friend, A companion, A relative")).toEqual([]);
    expect(stubGoalItems("I can say a, a companion and a relative.")).toEqual(["a"]);
    expect(stubGoalItems("I can say a female friend, a companion and a relative.")).toEqual([]);
    expect(stubGoalItems("I can read a timetable and a notice.")).toEqual([]);
  });

  it("never name a bare article", () => {
    const offenders: string[] = [];
    for (const track of loadTrackChapters()) {
      for (const chapter of track.chapters ?? []) {
        const stubs = [...stubTitleParts(chapter.title ?? ""), ...stubGoalItems(chapter.canDo ?? "")];
        if (stubs.length > 0) offenders.push(`${track.language} ${chapter.chapter}: ${chapter.title}`);
      }
    }
    expect(offenders).toEqual([]);
  });
});
