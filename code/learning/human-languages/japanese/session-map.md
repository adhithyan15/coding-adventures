# Japanese Session Map — One Small Lesson at a Time

This map describes the current authored pre-A1 runway. One canonical lesson is
one new-lesson session, and every lesson has a hard ceiling of five minutes.
Reviews may be added when due, but two new signs are never fused into one larger
lesson merely to shorten the schedule.

The exact order is derived from lesson frontmatter: chapter, then `sequence`,
then lesson id as a stable tie-breaker. [`curriculum.json`](./curriculum.json)
and [`chapters.json`](./chapters.json) are authoritative; this learner-facing map
summarises them instead of maintaining a second 100-row ordering by hand.

## Current sessions

| Sessions | Chapter | Lessons | Learner outcome |
|---|---:|---:|---|
| **S1-S6** | 1 | 6 | trace, copy, recall, read, and answer with **はい / いいえ** |
| **S7-S13** | 2 | 7 | build **こんにちは** from five prepared signs and explain final **は = wa** |
| **S14-S21** | 3 | 8 | build and use plain **ありがとう** with its dakuten and long final vowel |
| **S22-S26** | 4 | 5 | extend the known form to polite **ありがとうございます** and choose its register |
| **S27-S33** | 5 | 7 | write **日 / 本**, practise the components of **語**, and read **日本語** |
| **S34-S37** | 6 | 4 | write **コ / ー / ヒ** and read **コーヒー** as four morae |
| **S38** | 7 | 1 | retrieve the doorway exchange in four independently checked skills |
| **S39-S44** | 8 | 6 | hear and say **さようなら**, add three signs, then read and write it |
| **S45-S56** | 9 | 12 | say “I do not understand” and ask for one repetition politely |
| **S57-S68** | 10 | 12 | ask for slower speech, confirm understanding, and write five new signs |
| **S69-S80** | 11 | 12 | learn seven body words with four prepared signs and a no-guessing body map |
| **S81-S100** | 12 | 20 | add seven body words while interleaving twelve foundation, repair, script, and body reviews |

The current ledger therefore contains 100 sessions across twelve chapters. Its
47 writing lessons teach all 47 target-script glyphs that the book currently
shows in load-bearing text. The 35 lexical lessons are 28 words plus seven
phrases; reading, listening/speaking, mixed-practice, and retrieval lessons are
counted separately so practice volume is never mistaken for vocabulary volume.

## Script-before-decoding rule

A sign becomes load-bearing only after a writing lesson has isolated its shape,
sound or reading, and a small hand action. A useful spoken expression may arrive
first in romanization. The learner then observes and traces its new forms, copies
with support, recalls without the model, and only afterward decodes or writes the
whole expression. The current script-closure measurement is 47 shown, 47 taught,
zero never taught, and zero violations.

## Review rule

The `reviews_of` field in each lesson is the machine-checked review queue. The
fixed no-tracking fallback uses the session-count windows from
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md): retrieve a
new atom at N+1, N+3, N+7, and N+15 when the authored runway is long enough, then
move it into mixed practice. A missed item returns sooner; it never makes the
next new lesson exceed five minutes.

Chapter payoffs are cumulative retrieval points, not permission to forget the
earlier material. Chapters 7, 8, 9, 10, 11, and 12 deliberately mix skills and
old material. Listening, speaking, reading, and writing are scored separately,
matching the eventual assessment contract rather than allowing recognition to
hide a production gap.
