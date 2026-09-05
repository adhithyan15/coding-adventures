# OCAML05 — Representative core packages

Status: in progress

## Purpose

This contract adds the first substantive OCaml libraries to the emerging
implementation lane: `logic-gates`, `graph`, `directed-graph`, and
`state-machine`. Together they prove pure computation, generic data
structures, an exact local package edge, a transitive local closure, stateful
APIs, deterministic algorithms, checked metadata, capability declarations,
and measured tests without promoting OCaml into the established-language
denominator.

The tranche does not execute or extend the native OCaml build tool or claim
denominator promotion. It does add the minimum guarded generic-CI bootstrap
needed for the canonical build job to execute the newly discoverable package
BUILD files: the reviewed OCAML03 setup identity, fail-closed runtime and opam
repository checks, checksum enforcement, and single-worker execution while an
OCaml package is selected. This does not complete the execution-coupled build
substrate or the separate exact Ubuntu, macOS, and Windows representative-chain
workflow. OCAML01 through OCAML04 remain authoritative for lane classification,
package shape, toolchain identity, discovery, resolution, hashing, and
process-free BUILD validation.

## Exact toolchain and package shape

Every package uses OCaml 5.2.1, opam 2.5.2, Dune 3.17.2 with Dune language
3.16, Alcotest 1.9.0, bisect_ppx 2.8.3, and ocamlformat 0.27.0. Every package
contains the complete OCAML02 library tree, a substantive README and
CHANGELOG, an empty pure-computation capability profile, and no skip-success
build path. BUILD and BUILD_windows run formatting, tests, and measured
coverage and name every production `.ml` file with `bisect-ppx-report
--expect`.

The local dependency graph is exact:

```text
logic-gates       (no local dependency)
graph             (no local dependency)
directed-graph -> graph
state-machine  -> directed-graph
```

`directed-graph` pins `../graph`. `state-machine` pins the complete leaf-first
closure `../graph`, then `../directed-graph`, while declaring only
`coding-adventures-directed-graph` as its direct opam/Dune dependency. Local
package versions are exactly 0.1.0.

## Shared conventions

All APIs return typed `result` errors for invalid caller input. Public
collections and property bags are defensive snapshots. Algorithms must not
mutate graph definitions, machine definitions, or caller-owned inputs.
Observable node, edge, state-set, transition, component, and tie-break order is
the supplied `Map.OrderedType.compare` order, never hash-table iteration order.
Traversal implementations are iterative or tail-recursive for the 1,000-node
vectors. Configured size ceilings fail before unbounded allocation.

## Logic gates

The public module is `Coding_adventures_logic_gates` with `Basic`,
`Combinational`, and `Sequential` submodules. A bit is represented as an `int`
so the portable invalid-bit contract remains testable; every bit-taking API
rejects values outside 0 and 1.

`Basic` exposes the seven primitive gates, NAND-derived NOT/AND/OR/XOR, and
`and_n`, `or_n`, and `xor_n`. `Combinational` exposes `mux2`, `mux4`, `mux8`,
`mux_n`, `demux`, `decoder`, `encoder`, `priority_encoder`, and `tri_state`.
Select and result bit lists are least-significant-bit first. MUX input counts
must be nonzero powers of two; select widths must match. Encoder input must be
one-hot, while the priority encoder chooses the highest active index and
returns a valid bit.

The OCaml decoder accepts at most 16 selector bits. A wider selector returns
`Invalid_width` before computing or allocating the `2^N` output vector; this
fixed ceiling is identical on 32-bit and 64-bit targets.

`Sequential` exposes explicit immutable `latch_state`, `flip_flop_state`, and
`counter_state` records plus `Left | Right` shift direction. It implements SR
and D latches, rising-edge D flip-flops, simultaneous registers, directional
shift registers, and wrapping counters. Counter state includes both its value
and complete flip-flop state; no hidden global state is permitted.

The required vectors include all primitive truth tables, NAND equivalence,
invalid 2 and -1, 2/3/4-input reductions, every MUX index, LSB-first decoder
`[1; 0] -> [0; 1; 0; 0]`, one-hot and priority encoding, tri-state high-Z,
SR hold/set/reset/invalid, two-call low-to-high DFF capture, simultaneous
register capture, both shift directions, and a three-bit counter sequence
`000 -> 100 -> 010` in stored LSB-first order with `111 -> 000` wrap.

## Undirected graph

`Coding_adventures_graph.Make(Node : Map.OrderedType)` exposes one abstract
graph API over adjacency-list and adjacency-matrix representations. It owns
node, edge, graph, and property operations plus BFS, DFS, connectivity,
components, cycle detection, shortest path, and minimum spanning tree.
Both representations must produce identical observable results.

Property values are `String`, `Number`, `Bool`, or `Null`. Existing node and
edge additions merge properties. The canonical `weight` property and the
structural edge weight remain synchronized; removing it restores 1.0.
Weights must be finite and non-negative because the portable algorithms are
Dijkstra and Kruskal. Missing nodes, missing edges, disconnected spanning
trees, and invalid weights return explicit errors.

Required vectors cover property copy/merge/removal and weight synchronization;
paths, triangles, disconnected graphs, empty and singleton graphs, self edges,
both representations, and 1,000-node traversal. The weighted graph
`A-B=1, A-C=4, B-C=2, C-D=1` has shortest path `A,B,C,D` and MST edges
`A-B`, `B-C`, and `C-D` under deterministic ordering.

## Directed and labeled graph

`Coding_adventures_directed_graph.Make(Node)` composes the graph package and
uses literal ordered edge orientation `u -> v`: successors and transitive
closure walk forward; predecessors and transitive dependents walk in reverse.
A future build-tool adapter whose domain stores dependency-to-dependent edges
must perform that adaptation explicitly rather than changing DT01 semantics.

The module exposes the shared node/property surface, directed edge operations,
successors, predecessors, degrees, topological sort, cycle detection, closure,
dependents, independent groups, affected nodes, and strongly connected
components. Self loops are rejected by default and are opt-in at creation.

The nested `Labeled` module supports multiple distinct string labels on the
same ordered endpoint pair while retaining one structural directed edge.
Adding an existing label is idempotent, removing one label preserves the edge
until its last label is removed, and removing the structural edge removes all
labels. Returned labels are deterministic and immutable.

Required vectors include the diamond `A->B,A->C,B->D,C->D`, groups
`[[A];[B;C];[D]]`, closure from A as B/C/D, reverse dependents of D as A/B/C,
cycle and cross-edge-to-BLACK cases, self-loop off/on, affected sets, two SCCs,
an isolated SCC, and multi-label addition/removal.

## State machines

`Coding_adventures_state_machine` uses string states and events and exposes
abstract mutable DFA, NFA, PDA, and modal machines with defensive snapshots.
Epsilon is represented internally and publicly as `event option`; the empty
string sentinel remains only for file-format interoperability.

DFA construction validates states, alphabet, transitions, accepting states,
actions, and a default 100,000-entry trace ceiling. Processing is checked,
actions receive source/event/target, acceptance runs without changing current
state or trace, and reset clears runtime state. Reachability, completeness,
validation, tables, ASCII, and DOT output are deterministic.

NFA execution applies epsilon closure, supports multiple targets, and converts
to a DFA by bounded subset construction. Defaults are 4,096 generated DFA
states, 100,000 trace entries, and 1,000,000 total trace-state cells.
Minimization removes unreachable states and preserves language.

PDA transitions read one stack symbol and replace it with a list whose last
element is the new top. Defaults are 4,096 stack entries, 2,048 trace entries,
and 10,000 epsilon steps. Missing transitions reject without corrupting the
machine. Modal machines require explicit `switch_mode`; `process` dispatches
only to the active DFA and never switches implicitly. A switch resets the new
mode before it becomes active.

Required vectors include turnstile actions and trace, binary divisibility by
three, unknown/missing transitions, non-mutating acceptance, NFA epsilon
chains/cycles/branching and subset equivalence, minimization with unreachable
states, balanced-parentheses and `a^n b^n` PDAs, explicit modal switch/reset,
invalid triggers, and every configured ceiling.

## Coverage and validation

Each library must independently reach at least 95% production line coverage
under bisect_ppx and exercise every exported function, constructor validation,
custom error, representation, and configured limit. Coverage must be nonempty.
The complete package chain must pass formatting, Alcotest, optimized builds,
opam metadata checks, source distributions, capability-schema validation,
process-free canonical discovery/resolution/hash/BUILD checks, affected and
prerequisite planning, the collision-checked parity report, dependency and
license review, diff hygiene, and credential/artifact scans.

Generic CI must provision OCaml only when the detected incremental plan needs
the lane, or when a forced main-build shard explicitly contains OCaml. Pull
requests that select OCaml must retain all three generic operating-system legs.
The setup action, compiler, and opam repository revision must equal OCAML03;
`opam-pin` and Dune caching remain disabled; the configured repository and
runtime versions are checked immediately; and checksum enforcement is exported
for subsequent BUILD commands. Because the generic executor does not yet own a
global opam-switch mutation lock, an OCaml-active invocation must use one build
worker. The later `ocaml-build-substrate` owner replaces that conservative
serialization with the reviewed execution-coupled resource contract.

The separate `ocaml-representative-package-ci-execution` owner retains durable
ownership of a package-specific, exact Ubuntu/macOS/Windows chain that records
format, test, coverage, install, source-archive, and downstream-consumer
evidence. The bootstrap in this tranche is only what makes the already
mandatory generic build truthful and green.

The four package roots remain emerging inventory evidence. They do not change
the 15-language parity denominator until the separate OCaml promotion owner is
reviewed and merged.
