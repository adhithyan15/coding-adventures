## HL-C377 — a zero-introduction synthesis lesson counts as measurement-blind, because the retrieval-only list omits its type

Chapter 420 added `ES-C420-sintesis-salir`, a lesson that introduces nothing and
exists purely to retrieve five places and one verb. The corpus-wide
`measurement-blind` count went **6 → 7**.

That is not a defect in the lesson. `isExplicitRetrievalOnlyLesson` in
`ramp.ts` accepts `review`, `practice` and `practice-mix`, and does **not**
accept `synthesis`. A lesson whose `introduces` list is empty is retrieval-only
by construction, whatever its `type` says, so the type-based allowlist is
measuring the label rather than the thing.

The same shape already exists at `ES-C57-sintesis-reportar`, so this is the
second instance rather than a novelty.

**The type was deliberately NOT changed to `review` to clear the count.** The
lesson is a synthesis — it puts five previously separate words into one
connected exchange and makes the learner choose between `al` and `a la` — and
relabelling it would be fixing the measurement rather than the thing measured,
which is the failure mode this backlog keeps recording. The snapshot records the
increment honestly and the changelog names it.

**The fix worth making, when someone touches `ramp.ts`:** treat a lesson with an
empty `introduces.knowledge` as retrieval-only regardless of `type`, or add
`synthesis` to the allowlist. Either removes two false positives and stops the
next synthesis lesson from paying the same toll.

Related: the terminal-lesson cost measured in the same chapter. Adding three
atoms at the end of the Spanish track dropped its attained level from A1 to
pre-A1, because the level gate wants every atom at or below the level retrieved
twice and a last-introduced atom has nowhere to be retrieved. Two retrieval
lessons discharge it. **Budget them into any tranche that introduces new atoms
at the end of a track**, and check `attained` before and after — it is the one
number in this corpus that is supposed to rise, and it falls silently.
