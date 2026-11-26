# DT01 — Bounded Go shortest-path SAT evidence

## Purpose and boundary

This specification defines an independently encoded, solver-backed finite check
of Go `graph.ShortestPath`. It complements the graph package's unit, property,
and native fuzz suites. The harness is a standalone Go program package under
`code/programs/go/graph-shortest-path-sat`; it does not add a SAT-solver
requirement to the published `code/packages/go/graph` module.

The check targets the Go graph implementation described by [DT00 Go profile](DT00-go-shortest-path-preconditions.md). It covers only graphs with 2–4 labeled vertices, undirected simple topology, and edge weights selected from `{1, 2}` or absent. A graph is enumerated by assigning each canonical pair `i < j` one of three states: absent, weight 1, or weight 2. This excludes loops, parallel edges, zero and negative weights, and non-finite values. Weights are exactly representable `float64` conversions of small positive integers; this is not a proof about float rounding, cumulative overflow, or arbitrary graph sizes.

For each graph, both adjacency-list and adjacency-matrix layouts are constructed. Every ordered pair of distinct endpoints is checked. The finite case count is:

| Vertices | Topologies | Ordered endpoint pairs | Layouts | Cases |
| --- | ---: | ---: | ---: | ---: |
| 2 | 3 | 2 | 2 | 12 |
| 3 | 27 | 6 | 2 | 324 |
| 4 | 729 | 12 | 2 | 17,496 |
| **Total** | | | | **17,832** |

The package also exposes a native Go fuzz target. Its byte input maps to a
2–4 vertex topology, one layout, ordered endpoints, and a strict integer cost
bound from -1 through 7. It compares `ShortestPath` reachability and cost, then SAT versus UNSAT, with
the independent DFS oracle and replays every SAT model. Fixed seed inputs include disconnected,
single-edge, and competing-route shapes. The exhaustive deterministic check is
the finite coverage claim; the time-bounded fuzz smoke is complementary and
does not extend that claim beyond the stated domain.

## Independent oracle and CNF model

A test-only DFS independently enumerates all simple source-to-destination paths
and calculates their integer costs. It does not call `ShortestPath` or the SAT
encoder. The SAT side does not enumerate these paths. For each exact path length
`k` from 1 through `n-1`, it creates one-hot vertex variables at positions
`0..k`, fixes the endpoints, forbids repeated vertices, and constrains each
adjacent pair to be an edge in the graph. One-hot cumulative-cost variables
track exact integer weight sums. Each legal edge transition implies the next
cost state is the previous state plus that edge's weight.

A cost query disables final cost states at or above a strict bound. The
all-length query returns a SAT path model if any exact length is SAT and UNSAT
only when every length is UNSAT. Every SAT model is decoded, replayed against
the original undirected edge set, and checked for endpoint, edge, simplicity,
and exact cost validity.

## Claims and controls

For every reachable pair, the test calls `ShortestPath`, independently checks
the returned path is legal and has the oracle's minimum cost, then asks the SAT
encoder whether a strictly cheaper path exists. The query must be UNSAT. As a
negative control, it asks whether a path exists below `optimum + 1`; this must
be SAT, and its model must replay as a legal cheaper path. This control checks
that the CNF can produce a witness and that replay catches witness mistakes.

For every unreachable pair, the all-length path-existence query must be UNSAT,
and `ShortestPath` must return no path. Solver errors, UNKNOWN outcomes,
missing or invalid models, algorithm errors, and mismatches fail the test with
the graph representation, vertex count, topology index, edge list, and ordered
endpoints in the diagnostic.

This is bounded SAT evidence. The Go SAT package exposes SAT/UNSAT/UNKNOWN but
does not emit DRAT certificates, so UNSAT results are not independently checked
proofs and do not establish E5 proof status. The evidence applies to the pinned
source revisions and the finite domain above; it makes no general theorem claim.

## Build integration

The standalone verifier is a normal discovered Go program package so the build
planner can see its direct `require` dependencies on `graph` and `sat-solver`.
Its local `replace` directives point to the repository package roots. This lets
source changes in either provider select the verifier through ordinary reverse
dependency closure while keeping `graph/go.mod` unchanged. The verifier module
pins Go 1.26.1 because the solver module requires Go 1.26 and CI installs
1.26.1. Both Unix `BUILD` and `BUILD_windows` run the same focused `go test`
command. The module has no external Go dependencies.
