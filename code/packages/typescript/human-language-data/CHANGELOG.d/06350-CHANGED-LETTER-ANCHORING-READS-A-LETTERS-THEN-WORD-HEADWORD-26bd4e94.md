### Changed — letter anchoring reads a letters-then-word headword, and never counts the tatweel (HL-C443)

`writtenLettersOf` now counts the leading letters of a writing lesson whose
headword names its letters and then the word they build: Arabic "ا م — سلام" or
Tamil "ி, நன்றி". Before this change those lessons were invisible to the
measure, so the letters they write counted as unwritten. A headword that opens
with a word ("ஏழு ௭") is still not a letter set.

ARABIC TATWEEL (U+0640), the joining stroke, is no longer counted as a letter.

Measured effect on arabic: unwritten 18 → 14 before any new content. Cold went
5 → 7 and builds-toward 4 → 5, because lessons the measure could not see are now
counted. Arabic chapters 46-49 then take arabic's unwritten count to 0. The
figure-target pin moves from AR 3 to 15.
