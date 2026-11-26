# Data structures and measurement delivery backlog

Data structures lead this learning track. Complete each structure's language
packages with its interfaces, bounds, independent validation, examples and
microbenchmarks. Warrant is optional integration. The CPU simulator remains
later work; real-machine experiments can teach locality and allocation now.

## Delivery loop

1. Reconcile the active PR and this backlog, then prioritize by correctness,
   dependencies, learning value and package parity. Record the selected item.
2. Implement one bounded item, with the specification committed first. Keep
   newly discovered work in this backlog rather than silently adding it.
3. Run relevant tests, property/fuzz checks, packaging and independent security
   review. Open a **ready PR**, never a draft PR.
4. Watch that PR's current head. Fix CI failures and merge conflicts; missing
   checks are pending. Enable auto-merge when required checks are healthy.
5. Verify the merge, record evidence, clean up only that item's worktree, then
   reprioritize before selecting the next item.

Existing microbenchmark PRs are a carryover queue, not permission to open more
parallel implementation PRs. Their fixes continue alongside the first data
structure. No recurring automation or additional chat is required for this loop.
Build/test tools such as Busted and LuaRocks run in a CLI with captured output.

## Current selection

**DS-H-TS-A: TypeScript checked bounded heaps.** Go heap #20121
(`4ad9ec27f4fc00bffb34ee83817f5c84bb8dd399`), Python heap #20126
(`1db91bb7c989448425fc246f71050452b357bb05`), and Rust heap core #20150
(`6d02c654fea328c87fcfea22b1149905e7f0c745`) are merged. The Rust paired
benchmark remains separate and incomplete until the shared Rust collector
work is delivered. The selected TypeScript profile is specified in
[DT04-heap.md](DT04-heap.md#typescript-profile-checked-bounded-queues). It adds
checked bounded min/max queues while preserving the current unbounded API and
uses independent model histories, package installation, and real direct
consumer builds. The TypeScript paired benchmark remains held on the shared
collector; this heap item does not implement a private timer. Merged source is
not evidence that a package has been published.

Each lane below is a separate future work item and ready PR. Reconsider the
order after each merge; a correctness failure in the active item takes priority.

| Order | Item | Deliverable | State |
|---|---|---|---|
| 1 | DS-H-GO | Go heap interfaces, checked bounds, stateful model/property/fuzz tests, usage lesson, paired heap-build benchmark and publishable package metadata. | Merged #20121 at `4ad9ec27f4fc00bffb34ee83817f5c84bb8dd399` |
| 2 | DS-H-PY | Python heap profile, checked bounds, replayable standard-library state-model histories, corrected heap trace and complexity claims, and package evidence. | Merged #20126 at `1db91bb7c989448425fc246f71050452b357bb05` |
| 3 | DS-H-RS | Rust heap profile, checked growth/construction, release validation and independent histories. | Core merged #20150 at `6d02c654fea328c87fcfea22b1149905e7f0c745`; paired benchmark remains held on shared collector #20073 |
| 4 | DS-H-TS | TypeScript checked bounded heaps with legacy compatibility, bounded iterator construction, model tests, package installation and direct consumer builds. | Selected; specified in DT04-heap.md. Paired benchmark is a separate held collector-dependent item. |
| 5 | DS-H-CPP | C++ heap with bounds/ownership/exception profile and operation benchmark. | Queued |
| 6 | DS-H-JAVA | Java heap contract, model validation and operation benchmark. | Queued |
| 7 | DS-H-KOTLIN | Kotlin heap contract, model validation and operation benchmark. | Queued |
| 8 | DS-H-CS | C# heap contract, model validation and operation benchmark. | Queued |
| 9 | DS-H-FS | F# heap contract, model validation and operation benchmark. | Queued |
| 10 | DS-H-C | C heap with checked sizes/allocation/ownership and operation benchmark. | Queued |
| 11 | DS-H-SWIFT | Swift heap contract, model validation and operation benchmark. | Queued |
| 12 | DS-H-DART | Dart heap contract, VM/Node model validation and operation benchmark. | Queued |
| 13 | DS-H-ELIXIR | Elixir heap contract, persistent-state validation and operation benchmark. | Queued |
| 14 | DS-H-RUBY | Ruby heap contract, model validation and operation benchmark. | Queued |
| 15 | DS-H-LUA | Lua heap contract, native/runtime validation and operation benchmark. | Queued |
| 16 | DS-H-PERL | Perl heap contract, runtime validation and operation benchmark. | Queued |
| 17 | DS-H-HASKELL | Haskell heap contract, persistent-state validation and operation benchmark. | Queued |
| 18 | DS-H-OCAML | OCaml heap contract, model validation and operation benchmark. | Queued |
| 19 | DS-H-MOSAIC | Establish actual guest data/operation capability before implementing a Mosaic heap package and benchmark. | Capability audit required |
| 20 | DS-H-STARLARK | Establish actual VM data/operation capability before a Starlark heap package and benchmark. | Capability audit required |
| 21 | DS-H-TWIG | Establish actual VM data/operation capability before a Twig heap package and benchmark. | Capability audit required |
| 22 | DS-H-WASM | Specify linear-memory heap ownership/bounds and guest benchmark boundary. | Capability audit required |

Guest capability work must be explicit; a Rust implementation behind a host
adapter is not an independently implemented guest heap. Presence of an existing
package is not evidence that its API, validation or publication is complete.

## Structure and teaching queue

After the heap parity milestone, prioritize array/growth foundations, linked
lists, stacks/queues, hash maps/sets, trees/tries, graphs and range structures.
Audit existing packages before adding replacements. Each structure includes:

- A host-language interface/profile, invariants, error precedence, ownership,
  limits and compatibility decisions.
- Independent models checked after every operation, finite enumerations with
  declared bounds, seeded longer histories, shrinking/replay where supported,
  fuzzing and selected negative controls. State precisely what each establishes.
- A separately publishable package with installed-consumer evidence and the
  relevant per-language paired benchmark. Timing thresholds remain advisory.
- A small worked application. The user supplies LeetCode problems; explain the
  solution here before implementing it across the completed language packages.
- Interactive GitHub Pages operations and traces, including iteration/work
  counts, costs and runtime observations. Plan the shared trace contract and
  accessibility before choosing a visualization implementation.

## Discovered work and carryover

| Item | Priority | Evidence and next action |
|---|---|---|
| MB-CLOSE | P0 carryover | The benchmark carryover remains separate from the TypeScript heap API. At the last coordinator review, #20073 was still held on Rust profile completeness/hardware-key owner approval and an observed Ubuntu `strace` `PTRACE_LISTEN`/`EIO` CI failure; its auto-merge was off. Eight frozen platform/profile/native/coverage/guest carries remained unpublished pending validation. Recheck live heads, required gates, the capability-approval route, and the actual trace failure before claiming readiness; do not treat a successful local suite as hosted CI or owner approval. |
| MB-VOS | P0 metadata | The canonical VOS/source-input boundary repair was delivered upstream in #20069. Our #20178 repair was closed unmerged as superseded after source equivalence was verified. A bounded Ruff ISC004/doc follow-up remained; do not revive the superseded branch or conflate it with heap implementation. |
| MB-API | P2 follow-up | Consider a shared strict Go report-reader API only if another production consumer needs one; keep per-program replay parsers out of individual demos. |
| MB-META | P0 correctness | The generic workload contract exposed blank/NUL/Unicode inconsistencies in native readers. The coordinator reports the bounded fixes and independent review complete; unchanged reports still need to replay on each affected current PR head before closure. |
| MB-LESSONP0 | P0 hygiene | A tracked lesson title slug diverged from its filename. The coordinator reports the root correction complete; verify shard validation on each current PR head before closure. |
| MB-WIN-C | P0 CI | The C PowerShell install check previously stopped on expected native stderr before checking its exit code. The coordinator reports the harness repair and real Windows BUILD run complete; confirm the result is present on the current PR head. |
| MB-PLAN | P1 validation | A new downstream consumer can be orphaned from planner discovery even while a named BUILD passes. Structures added an explicit exclusion for the consumer whose coverage is supplied by named BUILD fixtures; the full planner passed with the fixture present. Keep its recorded reason and fixture evidence together if the registry changes. |
| MB-COVER | P2 evidence | Refresh coverage for changed workload-reader sources; retain historical source hashes until remeasurement. |
| DS-GRAPH-WEIGHTS-PY | P1 correctness | Python validation merged in #20140 at `60c3fac1ebb398435223229d63e68d392d177548`, after spec-first commits `23ae623b`, `046c3720` and `3266951c`. It rejects negative/non-finite/too-large-to-convert weights before same-node, missing-endpoint, or search paths, including disconnected edges and self-loops. The bounded simple-path oracle enumerates 729 four-node edge assignments per representation and compares minimum path costs for 1,458 graph/query cases; package acceptance was 145 tests with 96.56% coverage in the full verbose test run and 96.30% in the separate JSON-coverage run on Windows. These results do not imply package publication. The pre-fix audit remains at `outputs/data-structures/graph-negative-weight-audit.json`; source-pinned after-fix child-process evidence is `outputs/data-structures/graph-negative-weight-audit-after-fix-ae2ed75.json`. |
| DS-GRAPH-ZERO-MATRIX | P2 representation | The adjacency matrix uses zero to mean no edge, so it cannot represent a zero-weight edge while the adjacency list can. This repair tests the list behavior and documents the parity gap; do not claim both representations support all nonnegative weights. |
| DS-GRAPH-EDGES-SELFLOOP | P2 representation | Matrix `Graph.edges()` omits diagonal self-loops although `neighbors_weighted()` exposes them. `shortest_path` now validates via neighbor views, but the public edge-enumeration parity issue remains separate. |
| DS-GRAPH-PATH-SUM-RANGE | P2 numeric | Finite weights can still sum to binary64 infinity; shortest-path result behavior for path-sum overflow is not changed or proven by this item. Define and test that numeric policy separately. |
| DS-GRAPH-LANGUAGE-PARITY | P2 profile | DT00 now names the Python compatibility exception to the portable finite/nonnegative domain. Audit each language's actual Graph/MST behavior before harmonizing domains; this Python-only change does not establish cross-language parity. |
| DS-PUBLISH | P1 per lane | Inventory package manifests, dependency/version policy, registry availability and installed consumers; publication is separate from source presence and PR merge. |
| DS-TRACE | P2 dependency | Define truthful operation/state/work-count traces for interactive GitHub Pages, then ship one accessible heap visualization. |
| DS-CACHE | P2 experiment | Compare representation, branching and working-set choices using controlled equivalent work; do not infer exact cache sizes or PMU events from timing alone. |
| CPU-PIPELINE | Later | Resume pipeline/stall/cache integration after foundational data-structure and measurement work; keep this separate from measured host behavior. |

Python publication remains subject to the repository's `LV-007c` package
publication gate. No package has been published as part of selecting this
work. Go release-adapter support is also absent; record its actual status before
claiming publication parity for that lane.

Before selecting a new item, update this table with new findings and run the
prioritization step again. An item is complete only with its linked PR, relevant
evidence and verified merge; local tests alone do not mark it complete.
