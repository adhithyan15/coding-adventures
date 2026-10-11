### Fixed — narration reads one pronunciation per headword

`pairRomanization` paired every chapter pair whose romanization was not
already in the span, so a headword two lessons romanize differently was read
with both: Arabic ch2's ي became "ي (yāʾ) (-ī)" once the "my" lesson declared
`-ī`, and Chinese 好 read "好 (hǎo) (hào)". Now only the first pair per headword
applies, and `narrateLesson` puts the lesson's own pair before the chapter
glossary, so a lesson reads the headword the way it teaches it. A headword
already followed by one of its known readings in brackets ("ي (yāʾ)") is left
alone; a bracketed gloss ("("goodbye")") is not a reading and still gets the
pronunciation.

The "is the reading already here?" test was a substring test on the text
built so far, so a short reading was suppressed by any word containing it:
"Say it: iṭ … ই" never paired ই with *i*, ও never got *o*, and "kuṭumba" hid
कुटुंब's *kuṭumb*. It is now a whole-word test (still on the text built so
far, so a compound's inserted reading keeps covering its parts: مع السلامة
(maʿa s-salāma) does not also become مع (maʿa)). Many single-letter vowels and
short words across the Bengali, Arabic, Tamil and other tracks are now read
with their pronunciation for the first time.
