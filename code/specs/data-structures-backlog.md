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

**DS-H-GO: checked Go heap, independent histories and paired construction
benchmark.** Go's existing heap has one small example test; the benchmark
collector and statistics dependencies are already on main. This gives a small,
usable first structure without waiting for the remaining benchmark ports.
The public profile is [DT04b](DT04b-go-heap-validation.md).

Each lane below is a separate future work item and ready PR. Reconsider the
order after each merge; a correctness failure in the active item takes priority.

| Order | Item | Deliverable | State |
|---|---|---|---|
| 1 | DS-H-GO | Go heap interfaces, checked bounds, stateful model/property/fuzz tests, usage lesson, paired heap-build benchmark and publishable package metadata. | Selected; specification first |
| 2 | DS-H-PY | Python heap profile and same evidence; fix the incorrect push trace and unsupported “twice as fast” statement while teaching the operations. | Queued |
| 3 | DS-H-RS | Rust heap profile, checked growth/construction, release validation, independent histories and paired benchmark. | Queued; Rust benchmark PR #20073 |
| 4 | DS-H-TS | TypeScript heap with explicit number/comparator domain and operation benchmark. | Queued |
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
| MB-CLOSE | P0 carryover | Finish existing PRs #20073, #20080, #20092, #20093, #20096, #20097, #20100, #20110 and #20111. Convert remaining drafts to ready, fix current-head CI/conflicts and merge in dependency order. Record each verified merge. |
| MB-META | P0 correctness | The generic workload contract exposed blank/NUL/Unicode inconsistencies in native readers. Finish the bounded fixes already in progress, independently review, replay unchanged reports and propagate them to the affected PRs. |
| MB-WIN-C | P0 CI | The C PowerShell install check can stop on expected native stderr before checking its exit code. Repair the CLI test harness and run the real Windows BUILD entrypoint. |
| MB-PLAN | P1 validation | Validate discovery/selection after correctly populating partial worktrees. A passing command that omitted a language's packages is not that language's validation. |
| MB-COVER | P2 evidence | Refresh coverage for changed workload-reader sources; retain historical source hashes until remeasurement. |
| DS-GRAPH-WEIGHTS | P1 audit | The earlier Python review identified unstated Dijkstra negative-weight preconditions. Recheck current source and reproduce before changing the API. |
| DS-PUBLISH | P1 per lane | Inventory package manifests, dependency/version policy, registry availability and installed consumers; publication is separate from source presence and PR merge. |
| DS-TRACE | P2 dependency | Define truthful operation/state/work-count traces for interactive GitHub Pages, then ship one accessible heap visualization. |
| DS-CACHE | P2 experiment | Compare representation, branching and working-set choices using controlled equivalent work; do not infer exact cache sizes or PMU events from timing alone. |
| CPU-PIPELINE | Later | Resume pipeline/stall/cache integration after foundational data-structure and measurement work; keep this separate from measured host behavior. |

Before selecting a new item, update this table with new findings and run the
prioritization step again. An item is complete only with its linked PR, relevant
evidence and verified merge; local tests alone do not mark it complete.
