## HL-C371 — reading reach counts whitespace, which CJK does not use

`passageWordCount` splits a passage on whitespace and counts the runs that
contain a letter or a digit. For every track measured so far that is the right
unit. For Japanese and Chinese it is not a unit at all.

Japanese chapter 19's longest passage measures **6** against six lines, because
Japanese writes no spaces: each whole line counts as one. One of those lines,
**もうすこし、ゆっくりいってください**, is fourteen signs and four words, and the
measure sees a single token. The comma it contains is 、 (U+3001), which is not
whitespace either.

Two things follow, and neither is urgent while the pre-A1 parts ask for 1-3:

* **Japanese declares `unit: "words"`** and gets a number that is not words. It
  currently passes 1/1 because a six-token reading clears a three-word minimum
  by accident rather than by measurement.
* **Chinese declares `unit: "items"`**, which `reading-reach.ts` does not look
  at. The measure reports a word count against a part that never asked for one.

The fix is to make the count unit-aware rather than to change the passages:
count graphemes for a CJK track, and read `stimulusLength.unit` instead of
assuming words everywhere. Until then the CJK rows in
`npm run report:reading-reach` should be read as "has a passage at all" rather
than as a length.

Filed rather than fixed because it would change the measurement for two tracks
in the same change that first gives one of them a passage, and those are better
separated -- the floor for japanese/pre-A1 is recorded at 1/1 now and any
re-count has to keep it there or explain why.
