---
category: Compiler / VM / language pipeline
---

# A new edge target pass must reuse the index and preserve node contexts

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.

Swimlane animation configurations introduced another full-edge filter per
directive, after class/style targeting already had a bounded index. Native
32-directive/33-edge red controls counted 1,089 discovery visits, while repeated
real updates should visit only their matching edge. Build one ordered index
before all target passes and distinguish discovery, lookup and matching work.

The same change treated any @{ substring as configuration syntax before edge
or node parsing. Native controls rejected node-shape endpoints and literal
node labels with an unterminated-configuration diagnostic. Preserve their
complete authored IR and order; classify standalone identifiers separately.
Reconcile new animation fields in the resource fixture with exact layout
metadata assertions, preserving all original spacing/work/overflow checks.

Independent arithmetic review also caught a new unexecuted ALGOL backend
expectation: four exponentials rooted at negative sign of 2.25 yield entier of
sqrt of their log equal to 2, not 3. Distinct 2.25/0.0 procedure roots now test
both conditional arms with exact expected outputs 2/3. Do not label a model
correction as an executed test failure; require real backend results afterward.
