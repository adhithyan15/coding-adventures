---
category: Testing & coverage
---

# A chapter title that lands at a review's edge is spoken, so the drivable word scan judges it

**What went wrong.** The Tamil B1 vocabulary generator closes every group of about eight chapters with a review whose opening line names the group by its first and last chapter titles: "Looking back: from Stocks and Brokers to Plans on Paper — a review of the words from …". That line is narrated at the wheel. `tests/drivable-instructions.test.ts` runs a plain word scan over every spoken opening, question and title, and refuses hand verbs (write, copy, point, record …) and written answer forms (in writing, on paper …) unless an allowlist entry explains them. The chapter title "Plans on Paper" was fine as a book heading, but once it sat at the end of a review group, "on paper" became a spoken phrase and the test failed. Titles in the middle of a group (such as "Making a Point") never reach the review line, so the same words are harmless there; which titles are spoken depends on how the groups fall.

**The fix.** The chapter was retitled "Designs and Property" (its canDo already said "designs and property"), and the builder (`ta_b1_common.add_reviews`) now asserts that each review's spoken heading, span and prompt cues contain none of the scan's hand verbs or written-form phrases, naming the title to change.

**Do differently.** When a generator interpolates authored text into a line that is narrated, run the narration scans on that interpolated line before generating, not only on the authored fields. A title or gloss that reads well on a page can become an instruction once it is spoken.
