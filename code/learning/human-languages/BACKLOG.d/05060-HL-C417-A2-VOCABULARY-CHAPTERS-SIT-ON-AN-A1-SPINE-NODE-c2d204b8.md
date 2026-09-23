## HL-C417-a8d61e11 — A2 vocabulary chapters sit on an A1 spine node, so the A1 gate counts A2 words

**Status: CLOSED (2026-09-22).** Option 1 was taken. See RESOLVED below.
Raised by the security review of the first Spanish A2
vocabulary tranche (chapters 431-436), and confirmed by measurement.

A lesson's CEFR level is **derived, never authored**: `lessonsUpToLevel` reads
the stage of the **spine node** that the lesson's path segment names
(`src/levels.ts`). The module says so deliberately — "a track cannot claim A1
by editing frontmatter — it has to actually realize the A1 spine nodes."

The six new chapters use `SPINE-READ-SIGNS-AND-NOTICES`, the node chapters
424-430 already use. That node is `"stage": "A1"`. So thirty headwords chosen
**because a DELE A2 paper needs them** are counted inside the A1 cut-off:

| | before | after |
|---|---:|---:|
| A1 audit `lessonCount` | 1022 | 1064 |
| A1 audit `taughtForms` | 1721 | 1801 |

**Nothing is masked today.** The A1 audit was 0 failures / 0 missing before and
after; adding vocabulary cannot break a floor criterion. The concern is
directional: the A1 gate's taught set now contains A2 material, so A1 becomes
progressively easier to satisfy as A2 vocabulary lands, and "Spanish A1 is
pass-ready" comes to rest partly on words an A1 candidate would not be expected
to hold.

**The structural cause is that there is no A2 node for this kind of chapter.**
The four A2 spine nodes are `SPINE-TALK-ABOUT-FUTURE`, `SPINE-SAY-WHAT-I-DO`,
`SPINE-NEGATE-AND-ASK` and `SPINE-TALK-ABOUT-PAST` — all production/grammar
nodes. There is no A2 counterpart to the A1 TEXT-strand node for reading signs
and notices, which is exactly what these chapters teach: administrative notices,
job adverts, repair messages.

Options, roughly in order of preference:

1. **Add an A2 TEXT-strand spine node** — something like
   "I can read a short administrative text and act on it" — and move these path
   segments onto it. This is the honest fix: the material really is A2 reading,
   and the ladder currently has no rung for it. It is a spine change, so it
   touches every track's realization and wants its own review.
2. **Leave them at A1 and state it.** Defensible while the A1 audit stays clean,
   but the claim "A1 is pass-ready" quietly weakens each tranche.
3. **Split the audits' taught sets by authored intent** — rejected on sight: it
   would reintroduce the authored level the module exists to prevent.

Worth deciding before tranche 2 lands, because each tranche makes option 1 a
larger migration. Note also that `levels.ts` observes "today it is A1 or pre-A1
for every track in the corpus — nothing has reached A2", which remains true and
is the same gap seen from the other end.

### RESOLVED

Option 1, the honest fix, on 2026-09-22 — after the vocabulary programme had run
to its end at chapter 474, which is the worst time to do it and exactly what the
entry above warned would happen: *"each tranche makes option 1 a larger
migration."* Six chapters when this was filed; forty-four when it was fixed.

`SPINE-READ-PRACTICAL-TEXTS` was minted — TEXT strand, stage **A2**, prerequisite
`SPINE-READ-SIGNS-AND-NOTICES`, `core: false` like the A1 node it follows. Spanish
chapters **431-474** moved onto it: 44 path segments, 44 chapter shards and 281
lesson frontmatters. Chapters 424-430 did **not** move — 424-426 teach reading
mechanics and 428-430 close enumerated **A1** syllabus points (`A1-NE07-04`,
`A1-NE12-02`), so they are A1 material and stay there.

**The A1 gate returned to the exact numbers this entry recorded before the
tranches landed**, which is the measurement that the fix did what it claimed:

| | filed as | now |
|---|---:|---:|
| A1 audit `lessonCount` | 1022 | 1022 |
| A1 audit `taughtForms` | 1721 | 1721 |

`objectiveFailed` stayed **0** across the move, so no A1 criterion was resting on
the A2 material that has now been taken away from it. The A2 audit did not move
at all — `lessonsUpToLevel("A2")` includes everything at or below A2, so the
chapters the A2 book needs are still in it.

Its `concepts` list is **empty**, following `SPINE-DESCRIBE-QUALITIES` rather
than `SPINE-READ-SIGNS-AND-NOTICES`. Two reasons, and the second is a hard
constraint: a canonical concept is a claim on all 23 tracks and this rung is
justified today by one track's exam evidence; and `CONNECTED-READING` is already
owned by the A1 node, where a concept may own exactly one node. When a second
track reaches A2 reading, the concept is the thing to add.

Option 3 stayed rejected and was never reconsidered. Nothing here reads authored
intent — the level is still derived from the spine node, and the fix was to give
the material an honest node rather than to teach the audit to ignore where it
sits.

**One claim in the entry above was already wrong when it was written.** It says
`levels.ts` observes *"today it is A1 or pre-A1 for every track in the corpus —
nothing has reached A2"*, "which remains true". Measured while closing this
entry: **21 of 23 tracks already reach A2, and Spanish reaches C2** — 39 B1, 17
B2, 10 C1 and 18 C2 lessons. The module's own header warns that a fact copied
into prose goes stale where a derived one cannot, and this is that failure
happening inside the warning. The comment is corrected rather than deleted, so
the next reader sees what it used to claim.

**What this does not fix.** `HL-C418` (creer, explicar), `HL-C420` (problema) and
`HL-C422` (responder) are untouched: those four lexemes are taught and still
invisible to the A2 audit, for reasons that have nothing to do with this node.
