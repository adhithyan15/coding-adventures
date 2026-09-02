## HL-C211 — Level labels are bookkeeping; stop chasing pre-A1 counts

Owner decision (2026-09-01), settling a question four tracks escalated
independently:

> "Feel free to juggle the pre-A1, A1 and all these existing level
> classifications. I do not care about if you teach a word in pre-A1. Just make
> sure that it is a gentle ramp plus at the end of the A1 material, they should
> be able to sit and pass the A1 exam even if they know more than what the A1
> exam requires them to."

### What was escalated

The Kannada agent found that chapter 7's counting node is staged `A1` in
`KA-EXT-011`, as are the counting nodes of Tamil, Telugu, Malayalam and Hindi;
only Sanskrit stages its `pre-A1`. So ten new number headwords landed at A1 and
the track's `pre-A1 vocabulary 152/300` did not move. It observed that roughly
**200 headwords across five tracks** sit one level above the target they were
meant to fill, and asked whether to restage.

Tamil, Gujarati and Punjabi hit the adjacent version of this with the canonical
`VERB-*` concept ids, which are owned by spine nodes staged A1/A2.

### The decision

**Do not restage anything for the sake of a number.** Restaging would move
`pre-A1 vocabulary` by relabelling, teaching nothing. That is metric theatre, and
it is the failure this corpus keeps rediscovering.

**Place content where the RAMP needs it, and let the level label fall where it
falls.** Numbers belong early because a learner needs them early, not because a
counting node is tagged pre-A1.

**Stop reporting `pre-A1 vocabulary N/300` as a goal.** It measures a label, not
a learner. Under the owner's criterion the two real measures are:

1. **Is the ramp gentle?** One new headword per lesson, at most ~3 atoms, five
   minutes against the COMPUTED ceiling, gloss-first with glyphs interleaved and
   spaced rather than batched.
2. **At the end of the level's material, can the reader pass that level's exam?**
   Measured by `measureExamCoverage(inventory, lessons)` — which requires the
   track to have an exam inventory. Twenty tracks do not have one yet; that is
   the binding gap, not the level tagging.

A corollary worth stating: **knowing more than the level requires is fine.** The
owner said so explicitly. A track is not "wrong" for teaching an A1 word early.

### Consequences

- The `VERB-*` question dissolves. A pre-A1 lesson claiming a canonical verb
  concept relocates itself to A1, and under this decision that does not matter.
  Six tracks are short on the verb criterion; note the matcher is
  `/(^|-)VERB-/`, so namespaced tags like `MW-VERB-*` already count — Marwadi and
  Tamil both cleared it that way.
- `LEVEL_VOCABULARY` remains the road to C2 (300 / 600 / 1200 / 2500 / 4000 /
  8000 / **16000**) and is still the scale of the work. The change is that a
  headword counts wherever it is taught, and no tranche should spend effort
  moving one between labels.
- Where a track's own `curriculum.d` staging makes the ramp *worse* — a word
  needed early stranded late — fix the ramp. That is a placement change, not a
  relabelling exercise, and the test is whether a reader meets it when they need
  it.
