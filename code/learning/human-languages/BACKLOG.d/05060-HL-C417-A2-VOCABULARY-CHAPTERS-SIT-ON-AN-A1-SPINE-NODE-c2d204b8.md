## HL-C417-a8d61e11 — A2 vocabulary chapters sit on an A1 spine node, so the A1 gate counts A2 words

**Status: OPEN.** Raised by the security review of the first Spanish A2
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
