## What the fact-check then caught, before anything was committed

Twenty-seven findings. The three that mattered most:

**`ES-C60-` is a legacy module id, not a chapter number.** The draft said *"the
scale you have had since chapter sixty"*; `ES-C60-repaso-grado` is
`chapter: 229`. Filed in `lessons.d/` — a chapter number is only ever read from
the `chapter:` field, never off an id.

**The word *grado* had never appeared in corpus prose at all.** Every prior hit
is inside an identifier. So "the word you already had under another job" was
false; the lesson now says the review carries this word in its filename and
never once showed it.

**`roots: []` does not mean what the draft claimed.** It taught the empty root
list as a positive signal meaning "somebody looked and the answer is unknown".
526 of 1,476 Spanish lessons carry `roots: []`, and 332 of those give a settled
etymology in the prose anyway — including this chapter's own prerequisite
`ES-C300-piel`. `lessons.d/` had already recorded that the ledger is opt-in
metadata and a lower bound. The rule about leaving an origin *open* belongs to
`ES-C450-patio`, which is now the prerequisite and the citation, with
`el gorro` — winter headwear, origin also uncertain — as the nearest precedent.

