---
category: Testing & coverage
---

# A validator that re-walks a transitive closure per item goes quadratic once the corpus is one long chain

**What went wrong.** `validateCurriculum()` computed each lesson's assumable
knowledge by walking its prerequisites transitively, from scratch, for every
lesson. A language track is essentially one long prerequisite chain, so lesson
n re-walks n-1 ancestors. Over about 29,500 lessons that came to about 18
million walk steps, roughly 26 of the function's 41 seconds. The integration
test "has a valid shared spine" then hit its 30 s timeout on a loaded machine.
Every stroke PR in a row reported it as a "parallel load" timeout, when the
cause was the algorithm.

**Fix.** Memoise the closure: known(lesson) is the OR of its prerequisites'
closures, and each closure is a per-language bitset. Compute it bottom-up
with an explicit stack, since chains are thousands deep. Keep the original
walk as the definition, and fall back to it whenever the graph is not a clean
same-language DAG (cycles, cross-language edges, duplicated ids). Before
landing, compare the two on the real corpus and on a mutated corpus that
produces about 20k issues.

**Do differently.** When a corpus-wide test sits near its timeout, profile it
(`node:inspector/promises` Profiler inside a scratch vitest file works where
`--cpu-prof` does not reach the worker) before blaming load. Look for any
per-item scan of the whole corpus: `lessons.some(...)` inside a loop over
chapters, or a filter of every lesson inside a loop over spine nodes. Never
raise the timeout.
