## HL23 — node STAGES were never audited, and at least one was wrong

`SPINE-SAY-WHAT-I-WANT` sat at **A2**. Its `canDo` is *"I can say what I want or
need, and ask for it"* — an A1 capability by any reading of the CEFR, and one
DELE A1 tests directly. Nothing about its wording was wrong. It sat at A2 because
it declared `SPINE-SAY-WHAT-I-DO` (A2) as a prerequisite, and *quiero un café*
needs one memorised form rather than the present-tense machinery that node
teaches. HL23 §11 restages it A1 and drops the prerequisite; the `canDo` is
untouched.

**This whole document has audited which CONCEPTS sit on which node and has never
once asked whether the NODE's stage is right.** There is no reason to think this
was the only mis-staged rung, and the check is cheap: for every node, compare its
declared stage against the stages of its declared prerequisites, and flag any
node whose capability statement describes something a learner needs before the
level its dependencies force it to. `SPINE-SAY-WHAT-I-WANT` would have been
caught by the narrow mechanical half of that alone.

Two related items.

**§8.2 and §9.1's price tables are wrong and §11.2 corrects them.** Both counted
"realizing lessons that are not extension-resident". A concept moves without its
lesson moving when the whole segment is retargeted, because level derives from
`segment.spine_node`. The consequential correction: the four 9-track concepts
(`VERB-ASK`, `VERB-WRITE`, `VERB-READ`, `VERB-LIKE-LOVE`) are the SAME twenty
realizers in the SAME segments — `ES-PATH-031` holds all four Spanish ones — so
moving them together costs one split per segment rather than one per concept.
`gustar` is listed in §8.2 as the most expensive row in the table and is very
nearly free provided it travels with its neighbours. **Anything still to move
should be batched by segment, not by concept.**

**A segment split is an extension split.** Cutting a segment across an
extension's lesson set breaks two `validateCurriculum` invariants at once — an
extension is attached to exactly one segment, and every lesson it names lives in
that segment. It produced 173 errors before it was caught. An extension whose
lessons land in *n* runs must become *n* extensions.
