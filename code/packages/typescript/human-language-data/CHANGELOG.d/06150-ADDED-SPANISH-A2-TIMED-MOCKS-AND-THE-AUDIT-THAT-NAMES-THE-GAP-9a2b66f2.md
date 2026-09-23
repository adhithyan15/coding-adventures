### Added — Spanish A2 timed mocks, and the audit that names the vocabulary gap

`spanish/assessment.json` has named two timed A2 full mocks since it was
written, and `mocks/a2/` did not exist. It does now: `rubric.md`, two candidate
papers and two answer keys, written to the real DELE A2 shape already recorded
in `task-shapes/a2.json`.

#### What the audit says, and why it is allowed to fail

```
objectiveFailed: 93 of 100     missingObjectiveLexemes: 191
mock 1  reading 4  listening 0   mock 2  reading 0  listening 3
```

**That is a debt, and it is the point.** pre-A1 and A1 both audit at zero
because their mocks were written *after* the vocabulary existed, so the audit
could only confirm what was already true. A2 is the other way round: the mocks
were written **first**, and the audit is what names the words still to be
taught.

`--check` asserts the committed report is **not stale**, never that it is clean,
so this commits an honest machine-checked statement of how far Spanish A2 is
from passable. Every vocabulary tranche moves the number down; the day it
reaches zero the A2 test block should read like the A1 one.

#### Why the mocks came before the vocabulary

Choosing A2 words by theme was tried first and failed: **27 of 35** candidates
were already taught, because at ~817 headwords the obvious concrete domains are
saturated. Deriving the list from the exam has no such waste, and it prioritises
by what the paper actually demands — `devolver` alone unblocks 7 items,
`retraso` 5, `recoger` 4. 46 of the 191 appear in two or more items.

Mock 2 deliberately samples the **same lexical range** as mock 1: two papers at
one level should test one vocabulary, not two. It adds only 82 new lexemes on
top of mock 1's 109, so 191 rather than 218.

#### The module needed three lines, as its own comment predicted

`spanish-a1-mock-audit-cli.ts` already carried the level as a parameter, and its
comment recorded that generalising from A1-only to pre-A1 needed changes in
"only the three places that spelled `a1` out loud". Those were exactly the
places: the `MockAuditLevel` union, `AUDIT_DIR`, and the CLI's level parse (plus
its usage string). No measurement changed.

#### The ceiling fell by exactly three

`core/assessment-artifact-ceiling/spanish.json` lists unbuilt assessment
artifacts and "may fall and may never grow". Building these dropped
`mocks/a2/rubric.md` and the two A2 answer keys from it — independent
confirmation that the contract's named artifacts are the ones now on disk.

#### Provenance and placeholders

The rubric cites only the three Instituto Cervantes sources already recorded in
`task-shapes/a2.json` (accessed 2026-09-14); none was invented. No exam item or
stimulus is reproduced — every item is original — and the rubric names the one
thing it does not claim originality for, the standard task-instruction phrasing.

Every name, address, telephone number and email in both papers is invented, and
every domain is a reserved RFC 2606 `.invalid` name that cannot resolve. Worth
flagging separately: the **A1** papers use `@correo.es`, and `.es` is a live
ccTLD that could resolve to a real entity. That is pre-existing and was left
alone rather than fixed inside an unrelated change.

#### Verification

`npm run validate` 21/21, all thirteen gates including the new
`check:spanish-a2-mock-audit`, and the full suite at 172 files / **2204 passed**
(up 3 — the new A2 block) / 1 skipped.
