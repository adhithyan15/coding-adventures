## A concept cannot leave a spine node unless every lesson realizing it moves too

`HL23` recommended splitting six everyday-action concepts off
`SPINE-SAY-WHAT-I-DO` and priced it at 46 ledger entries: one new entry and one
recomputed `omits` per track. It read `validateCurriculum` carefully enough to
find the two rules that make a spine change a 23-track change, and still missed
the one that costs money.

`curriculum.ts` classifies every lesson sitting in a path segment:

```ts
const isSharedContent =
  CONTENT_TYPES.has(lesson.realization.type) &&
  conceptOwner.get(lesson.realization.concept) === placement.node;
if (!isSharedContent && (extensionLessonCount.get(lessonId) ?? 0) === 0) {
  error("unclassified-curriculum-extension-lesson", ...);
}
```

Either a lesson is the **canonical realization** of one of its node's concepts,
or it is **local support** and must belong to an extension. So re-parenting a
concept silently reclassifies every lesson that realizes it, in every track, and
each one that is not already extension-resident becomes an error.

Measured over the 42 concepts: **five** were free. The recommended set cost 19
lesson migrations across 13 tracks, and one concept — `VERB-LIVE` — additionally
tripped `misplaced-shared-realization`, a second and independent error class,
because two of its realizers carry no explicit `spine_node`.

The shipped slice was three concepts and one Spanish-only segment split.

**Generalisable check:** when a taxonomy edit is priced by counting the
*bookkeeping* it changes, ask what else joins against the thing being moved. A
concept id here is a join key in three places — the ledger's `omits`, the
ledger's `relocates`, and the lesson-classification pass — and only the first
two look like bookkeeping. The one that does not is the one that reclassifies
content, and its cost is measured in lessons, not entries.

**And the direction matters.** The rule is right and should not be relaxed: it
is what stops a node quietly acquiring lessons no `canDo` covers. What was wrong
was a cost estimate taken from reading two of the three consumers. Read the
error catalogue for the *thing* you are moving, not for the *file* you are
editing.
