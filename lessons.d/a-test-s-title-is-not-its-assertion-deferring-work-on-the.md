---
category: Testing & coverage
---

# A test's TITLE is not its assertion — deferring work on the strength of a test name can cost rounds for nothing

Marathi was passed over **twice** in a corpus-wide reinforcement programme, in two successive
tranches, because a test in its corpus directory was called:

    gives the five Chapter 14 family atoms genuine R1, R2, and R3 retrieval

That name reads like a *position* pin: R1/R2/R3 are retrieval windows measured as distances in
lesson indexes, so a title promising "genuine R1, R2 and R3" sounds like it asserts where those
lessons sit. Inserting a lesson anywhere below them would shift every index and break it. Two
rounds of work routed around the track on that reading.

The body is four lines:

```ts
const repairedAtoms = new Set([...]);          // five named atoms
expect(report.reinforcement.filter((d) => repairedAtoms.has(d.atom))).toEqual([]);
```

It is a **set-emptiness assertion over five named atoms**: none of them may appear in the
reinforcement defect list. Adding a `review` lesson can only give an atom *more* revisits, so it
can only move an atom **out** of that list. The assertion is not merely compatible with the
deferred work — it is *monotone* in the direction of that work and cannot be broken by it.

**The title described the test's PURPOSE; the body describes its PREDICATE, and only the predicate
constrains you.** A good test name states why the test exists, in the vocabulary of the domain. That
is a different sentence from the one the runner evaluates, and it is written to be read by someone
who already knows the answer. When the question is *"can I safely change X?"*, the title is
evidence about intent and nothing more.

**The cost was asymmetric and invisible.** Reading the body is one `sed -n` over a file already on
disk — seconds. The deferral cost two full tranches of planning, during which the track sat at the
same blocker while smaller ones were cleared around it, and produced two backlog entries explaining
a constraint that did not exist. Nothing warned that the reasoning was wrong, because a deferral
never fails: the suite stays green and the work simply does not happen.

**The operational form**: before deferring work because a test looks like it blocks it, open the
test. Specifically —

- Read the **assertion**, not the `it(...)` string. Ask what value it computes and what it compares
  that value to.
- Classify it. A `toEqual([])` / `toHaveLength(0)` over a *filtered* set is monotone and usually
  safe in the loosening direction. An ordered list of ids, an exact index, or a whole-track count is
  a genuine pin.
- If it is a genuine pin, **measure the boundary** rather than assuming a conflict. The companion
  track in the same tranche had five real index pins and the insert was still safe: the pinned index
  was sequence 870, and everything being inserted sat above sequence 1820.

A test you have not opened is not a constraint. It is a guess wearing a constraint's clothes.

Related: [[a-corpus-count-is-measured-against-a-base-and-pushed-against-a]] — also about a number
believed rather than re-derived, but there the belief made a claim *wrong*; here it made work
*not happen*, which is the failure mode that leaves no trace.
