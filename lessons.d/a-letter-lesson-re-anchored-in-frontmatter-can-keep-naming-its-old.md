---
category: Repo policy / workflow reminders
---

# A letter lesson re-anchored in frontmatter can keep naming its old, later anchor word in its prose

HL-C443 gave KA-S125-letter-oo (ಓ) a new anchor: KA-C29-oota (ಓಟ), the lesson
right before it. The frontmatter changed (`prerequisites`, `requires`,
`practises`). The body did not. It still said "The word you already say that
opens with it: ಓದು", and "ಓ had to wait until ಓದು was taught". ಓದು is
KA-C33-oodu, 13 lessons later. Four Punjabi letter lessons had the same fault
in a different form. Their gloss reads "met inside a word you already say", but
PA-S03, PA-S07 and both PA-S09 lessons anchored on a word taught after them,
one of them 149 lessons later.

Every validator passed, because nothing compares the anchor named in the prose
with the anchor in the frontmatter. The forward-reference report
(`measureContinuity`) is what surfaced them: each letter lesson printed a word
a later lesson owns.

The fix was in the prose. KA-S125 now names ಓಟ. The Punjabi lessons anchor on a
word taught before them (ਹੋਣਾ, ਪੰਜ, ਤੂੰ). PA-S03 has no earlier word holding ਟ ਠ
ਡ, so it shows the next lesson's word only as a decomposition and says so.

Do differently: when a letter lesson's anchor or position changes, grep its body
for the old anchor word as well as its frontmatter. Then run
`measureContinuity` filtered to that track. A letter lesson that appears there
is naming a word the reader does not have yet.
