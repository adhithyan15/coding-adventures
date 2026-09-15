## 0.126.0 — 2026-08-12 — statically nonempty step-loop initialization

Definite string initialization now flows out of `step`/`until` elements when
finite static start, step, and limit values prove the loop body executes at
least once. Descending loops use the negative-step bound rule; zero-trip and
dynamic cases continue to restore the entry initialization set.
Local string slots receive an empty runtime seed so typed backends can verify
the syntactic zero-trip predecessor; the compiler's separate initialization
set still rejects every source-level read before assignment.

