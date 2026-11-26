# DT00 Go profile: shortest-path distance overflow

The Go `graph.ShortestPath` operation accepts finite non-negative edge weights.
Path distances use Go's IEEE-754 `float64` addition semantics. If a candidate
distance addition produces positive infinity, that candidate is not a finite
path and is excluded from Dijkstra relaxation.

When a query has a path whose computed cumulative distance remains finite,
`ShortestPath` returns the least-distance such path and preserves its existing
`([]string, error)` result shape. A finite route remains preferable to an
alternative whose cumulative sum overflows. If no finite computed route exists,
a topology-only reachability check distinguishes two outcomes: a disconnected
target retains the existing empty-path, nil-error result, while a connected
target returns `ErrPathDistanceOverflow` and no path. Same-node queries retain
their existing one-node path result after graph-weight validation.

The overflow error is scoped to this Go implementation and to computed
`float64` additions. It does not claim exact real-number path ordering or prove
correctness for arbitrary float64 inputs. Mutation behavior, stored edge
weights, MST behavior, endpoint errors, and results for queries with a finite
computed shortest path are unchanged.

## Verification evidence

Focused regressions cover both graph representations, a connected route whose
sum overflows, a disconnected target, a finite alternative beside an
overflowing route, the finite rounding boundary at `MaxFloat64`, and same-node
precedence. A bounded model/fuzz domain independently enumerates simple paths,
tracking topology separately from finite computed distance. This is reproducible
bounded evidence, not a general float64 proof.
