# What to do next — the computed backlog (HL15)

```bash
npm run plan              # the ordered queue, text
npm run plan -- --format json --head 5
npm run plan -- --ceiling A1     # just the first rung
```

The gate above says what is *wrong*. This says what to *do* about it, and it is a
pure function of the same measurements rather than a list somebody maintains:

```text
23 tracks, 0 done; 146 enumerable item(s) today, ~10,791 projected to C2

    1. [pre-A1] assessment-contract — marwadi
       require independent four-skill passes, a writing ramp, and timed full mocks
    2. [pre-A1] assessment-contract — japanese
       require independent four-skill passes, a writing ramp, and timed full mocks
    3. [pre-A1] assessment-contract — chinese
       require independent four-skill passes, a writing ramp, and timed full mocks
```

Three ordering rules, all mechanical. **The floor is universal** — every pre-A1 item
outranks every A1 item in any track, because a track that climbs while its floor is
missing has built a cliff with upper-level lessons on top. **Family priority** then
decides what a track's next action is: the complete pass-ready assessment contract
first, then any declared non-CEFR capstone, its sourced four-skill task shape,
gentle writing evidence, human mock validation, external/project content inventory,
script closure, exam points, and vocabulary. And the
queue **rotates across tracks**, furthest-behind first, so every language moves once
before any language moves twice.

Two corrections are baked in and worth knowing about, because both looked right:
a flat sort produced a head of 22 consecutive research tasks and no content, and an
inventory for a rung a track has not reached was outranking the floor it is standing
on. See `code/specs/HL15-the-completion-plan.md` §4.

Some families report `null` rather than a number in the projection. That means
**not projectable** — human-validation work is exact for mocks in contracts that
exist but unknowable for tracks with no contract, while reinforcement debt at B1
is a function of lessons nobody has written. That is a different fact from zero.
