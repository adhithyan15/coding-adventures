# HL126 — Chinese lesson-content budget audit

Issue: #13263 (parent #13247; program #12206).

## Problem and boundary

At the audited baseline `6a301134eabb`, 3,043 of 3,234 Chinese lesson files explicitly
declare `introduces_idioms`, `introduces_senses`, and
`introduces_culture_claims`. The 191 undeclared files are concentrated in the
early, hand-authored runway. An absent field is unknown, not a reviewed zero.

This first bounded tranche reviews the 13 canonical Chapter 1 and Chapter 3
lessons, including the floor-number claim. It does not infer declarations from
filenames, automatically write empty arrays, change the shared detector, or
relax the one-idiom, one-sense, two-culture-claim limits. The remaining 178
undeclared Chinese lessons and Japanese coverage remain in #13263. Later,
already annotated lessons are not retroactively certified by this tranche.

## Audit rule

Read each lesson's whole learner-facing body and current frontmatter. Declare
each of the three fields explicitly. Use an empty list only after checking that
the lesson introduces no such unit. When a new sense or culture claim is
present, use a stable `ZH-`-prefixed identity whose meaning is clear from its
name, and count that identity exactly once at its first teaching lesson.
Pronunciation, script, grammar, etymology, and ordinary vocabulary are not
automatically idioms, new senses, or culture claims. A claim about how people
use a greeting is a culture/pragmatics candidate and must be examined, not
silently treated as zero. Do not mark unsupported or disputed statements as
settled cultural facts.

For Chapter 1 specifically, `好` *hǎo* (`good/well`) and *hào* (`be fond of`)
are explicitly taught as different tone-distinguished words, not a new sense
of one word. The woman-and-child gloss is identified as a disputed character
etymology, not a claim about a people. An unsupported generalization about
when Mandarin speakers use `你好` was removed (#19378); the lesson now teaches
the greeting itself, with an official YCT beginner example, rather than a
cultural usage rule. These are reviewed zeroes for the three declaration
families; the lexical and etymology teaching remains represented by existing
knowledge atoms.

Chapter 3 teaches the floor-number omission example first in `ZH-C03-si`;
`ZH-C03-practice` revisits it without introducing a second claim. This is
one `ZH-CULTURE-SI-FLOOR-NUMBER-01` unit. The claim's source and scope repair
is tracked by #19374; the unrelated unsupported universal numeral-history
analogy in `ZH-C03-san` is tracked by #19375.

## Evidence and release gate

Add a Chinese-owned corpus assertion that pins all 13 reviewed lesson IDs,
their three declarations, and the one direct culture-claim identity. Ratchet
the measured Chinese coverage from the baseline 3,043 to at least 3,056 of
3,234 lessons while keeping zero over-budget lessons and `ZH-`-prefixed unit
IDs. Do not assert 100% coverage before #13263 is complete. The test must fail
on the baseline before the lesson edits and pass after this bounded tranche.
Existing duration, writing, modality, narration, artifact, and PDF gates stay
intact.
