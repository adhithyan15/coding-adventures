---
category: Repo policy / workflow reminders
---

# One untaught letter can block an entire tense, and the fix is to teach the letter

The Hindi A1 inventory said of `HI-A1-V-12`: *"Nothing in the corpus is
past-tense."* Four hundred lessons, ninety-four chapters, and no way to say
where you had been. The obvious reading is that nobody got round to it.

The real cause was one glyph. Hindi's past copula is **था / थी / थे**, and the
letter **थ** had never been taught anywhere in the track — not in any of the
twenty-five letter lessons, not in passing. `measureScriptClosure` counts a
glyph as taught only when a script lesson (`type: writing` or `delivery:
script`) shows it, so every past-tense sentence would have been something the
reader was asked to decode without ever having been given it. Nine lessons were
already carrying थ words (हाथ, माथा, थाली, चौथा); four of them were live closure
violations.

Two routes were open. Romanize the past copula throughout the prose — legal,
because a headword with a declared `romanization` is exposure rather than
load-bearing — or teach the letter. Romanizing would have produced a chapter
whose central word the reader could not read, and left the nine existing
violations untouched. Teaching it cost one lesson of about 150 seconds:
`HI-S126-letter-tha`, slotted into the existing recognition chain after
`HI-S124-letter-dha`, with the stroke order already sitting sourced and unused
in `data/scripts/devanagari.json`. Hindi's closure violations went 38 → 35, and
the past tense became writable.

Three things to carry forward.

**When a track cannot say something, check the alphabet before the
curriculum.** Run the taught-glyph set against the words the point needs, in
reading order, before planning any lessons. Rendering the check takes a
30-line script over `measureScriptClosure`'s own definition of taught, and it
answers "why has nobody written this" in one line.

**The stroke data is usually already there.** `devanagari.json` carries all 44
letters with sourced stroke orders and pen-lift counts; only 25 had lessons. The
blocking resource was authored prose, not research, and the letter lesson is a
fill-in-the-template job once the data row exists.

**Position the letter lesson before the earliest word that needs it, not before
the new chapter.** Closure is measured in `readingOrder`, so a letter taught at
chapter 20 retroactively clears every violation from chapter 21 onward. Placing
it as late as the chapter that prompted it would have cleared one violation
instead of four.
