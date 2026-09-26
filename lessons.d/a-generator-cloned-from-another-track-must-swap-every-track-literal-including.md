---
category: Repo policy / workflow reminders
---

# A generator cloned from another track must swap every track literal, including the book chapter-label prefix

**What went wrong (Hindi, Tamil and Telugu A1 tranches).** The three new
tranche builders were made by copying the Marathi one with `sed`. The copy
replaced `TRACK`, `PREFIX`, the language name, the script set and the `MR-` id
prefix. It missed one more literal: the book chapter label, `"ch:mr-" +
slug`. So 132 new chapters in three other books were labelled `ch:mr-…`. The
books still compiled, because a label only has to be unique within its own
book. That is why neither the suite nor the book compile noticed. A security
review of the Telugu diff spotted the odd prefix.

**Fix.** The three builders now use `ch:hi-`, `ch:ta-` and `ch:te-`. The
generated `chapters.d` shards and book chapters were corrected on each
track's branch, and the book hashes were regenerated.

**Do differently.** Before cloning a track's generator, grep it for every
two-letter track code and every language name, not only the id prefix:
`grep -n -i 'mr\b\|mr-\|marathi'`. After generating, grep the new output for
the source track's code.
