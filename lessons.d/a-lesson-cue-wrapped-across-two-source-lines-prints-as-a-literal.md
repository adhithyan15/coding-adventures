---
category: Repo policy / workflow reminders
---

# A lesson cue wrapped across two source lines prints as a literal bracket in the book

**What went wrong.** While turning bare "Write …" instructions in drivable
lessons into `[YOU WRITE: …]` cues (issue #12070), the first draft wrapped long
cues at eighty columns, the way lesson prose is wrapped:

    4. [YOU WRITE: all five from dictation — no new Cyrillic letter appears in any
       of them]

The narration was correct, because it joins a paragraph or list item before
it splits cues. The book was not. `stripDeliveryCues` in
`human-language-data/src/book.ts` converts cues one source line at a time. A
cue with no `]` on its own line never matches, so the chapter printed
`\item {[}YOU WRITE: all five from dictation …{]}`. That is valid LaTeX, so
`check:books` (which compares the generator with itself) and the book compile
both pass. 28 cues did this.

**The fix.** Each cue sits on one source line, however long. A sentence that
followed the cue goes in its own paragraph or, inside a numbered step, into
the cue itself.

**Do differently.** After adding or editing cues, regenerate the books and run
`git diff -- '*/book/chapters/*.tex' | grep '^+' | grep 'YOU '`. Any hit is a
cue the book did not convert.

**Since then.** `src/delivery-cue.ts` gave the book and the narration one cue
grammar: `bookVoice` scans each paragraph and list item as a unit, so a wrapped
cue now typesets like a one-line cue, and `check:books` / `generate:books` fail
on any `{[}YOU`, `{[}PAUSE` or `{[}REPEAT` in a generated `.tex`. The grep above
is now a quick local check rather than the only guard.
