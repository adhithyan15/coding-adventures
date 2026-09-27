## HL-C451-af9a95ec — language-ladder lazily loads writing filmstrips

**Status: CLOSED — tracked by #16114.** HL-C443's books printed generated
stroke-order filmstrips, but the study app deliberately excluded them after an
eager URL map crossed its 500 kB first-paint budget.

Filmstrip URL loaders now live in their own dynamically imported module. A
writing lesson requests its lesson-owned SVG only when the learner opens the
details, then places it at the top of the first Writing or Script section.
Lessons without cited ductus resolve to no image, preserving HL11 section 5.2.

The ordinary generated-figure path remains synchronous. Focused tests cover a
real Tamil filmstrip, an uncited lesson, and unsafe lookup keys; the bundle gate
requires exactly one source-map chunk and proves that it is not preloaded. The
largest eager chunk remains 495,771 bytes.
