---
category: Repo policy / workflow reminders
---

# Hand-corrected lesson prose must avoid the banned words too, such as just in the line you just drew

**Context:** `human-language-data`, correcting the stroke description in the
Japanese writing lesson JA-W01-ha while adding the に, は, ま, り and ん ductus.

**What happened:** the corrected list item said the loop comes "back up across
the line you just drew". Every check:* gate passed, but the full suite failed
`tests/banned-words.test.ts`: the corpus-wide count of banned words in
learner-facing prose went from 1056 to 1057.

**Why:** that test counts *simply*, *just*, *obviously* and *as you know* in
every lesson block a learner sees, and pins the total as a ratchet. A small
prose correction is learner-facing prose like any other, and "just" is easy to
write without noticing in a phrase like "the line you just drew".

**Fix:** reworded to "back up across its own downstroke", then regenerated the
books, modality record and narration for that lesson.

**Do differently:** after editing any lesson prose, grep the changed lines for
`\b(simply|just|obviously|as you know)\b` before running generators, and run
`tests/banned-words.test.ts` alongside the check:* gates rather than waiting
for the full suite.
