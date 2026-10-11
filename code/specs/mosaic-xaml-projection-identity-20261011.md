# XAML projection identity (2026-10-11)

Follow-up to #16865 and epic #14267. Batching alone still measured 269â€“391 ms
selection-to-next-render callbacks and about 8.4 MB managed allocation in a
Release WinUI host. This change targets repeated ItemsSource replacement.

## Contract

Generated For projections retain one observable collection per projection
owner. Reconcile by position: retain an existing row VM when all constructor
inputs are equivalent, replace changed rows, append new positions, and remove
trailing positions. Null sources clear the collection. Lists compare recursively
by value; scalar and opaque inputs use their typed equality comparer. Positional
identity does not promise keyed identity across insertions or reorderings.

Retained VMs notify computed helper properties and nested projection properties
when the projection is refreshed. Thus owner selection changes remain visible
even when source data is unchanged. Nested projections follow the same contract.
Do not use generated record equality: cache and event fields are implementation
state, not constructor input values. No app-specific emitter branches or edits
to generated output.

## Validation

Compile actual emitted C# against a minimal owner and exercise unchanged list
replacement, owner-dependent helpers, nested rows, null, growth, shrink, edits,
widths, selection and captured inputs. Run emitter regression tests and compile
a generated WinUI VisiCalc host. Profile real populated/blank selection and check
keyboard/editor/persistence behavior. Record measured results and limitations;
this change alone does not close #16865 or establish native release readiness.

## Follow-up within this change

The first retained-identity native sample reduced selection allocations to about
2.35 MB but still took 159â€“244 ms on six selections and 1221 ms on one stalled
selection. Retained rows must also compare computed helper values and notify
only those that changed. Cache the last observed helper result, update it before
raising the event, and leave direct getter reads current. Nested projection
refreshes still propagate owner changes to descendants. Add a compiled check
that an unchanged helper does not notify while changed helpers do.


## Recorded validation

- Full mosaic-emit-xaml suite: 383 unit tests plus integration tests passed;
  one existing ignored doctest. The compiled emitted-C# probe covers matching,
  nested collection identity, changed/unchanged helper notifications, owner
  isolation, data edits, selection, widths, captures, null, growth and shrink.
- Generated Release WinUI host builds with zero errors (two CS0067 warnings for
  the header VM's unused notification event). No generated-source edits.
- The first retained-identity candidate, before computed notification filtering,
  selected B2, E5, blank F6, A1, C3, G6, then Right to H6. Labels and highlights
  matched; Right reached H6 with no horizontal scroll, unlike the prior G5
  failure. Formula editor accepted =21+22 and Enter displayed 43 in H6.
- Its seven selection-to-next-render intervals were 243.6641, 190.4996,
  192.3001, 159.1397, 1220.8031, 182.4646 and 175.4240 ms; allocations were
  2,345,912–2,368,504 bytes. The C3 outlier contained a 1030.8376 ms synchronous
  persist-snapshot operation. Formula change/commit intervals were 64.9440 and
  55.3658 ms. Raw phases are in the adjacent identity-profile JSONL.
- Prior batching-only Release data used the same first five click targets and
  recorded 268.7676–391.1765 ms and about 8.4 MB. These small, separate runs
  support lower managed allocation, not a controlled latency speedup or p95.
  Rendering callbacks are not compositor presentation; bytes are current-thread
  managed allocations, not native heap. The Rust DLL is the same retained
  adapter, not rebuilt or asserted release-optimized in this experiment.
- The final computed-notification filtering candidate passed the compiled probe,
  full suite and native build. Native restart validation was stopped by physical
  Escape before observation; no final-candidate latency or restart success is
  claimed. Existing native acceptance and release gates remain open.

Remaining: final-candidate interaction/restart evidence, larger matched workloads,
projection/realization counters, persistence stalls, and the under-50-ms p95
acceptance. This PR is a draft and does not close #16865 or #14278.

## Focus lookup follow-up

FindCell currently collects every descendant cell before testing coordinates.
Replace it with depth-first search that returns immediately at the first match,
preserving traversal order, null for absent/unrealized coordinates and automation
provider behavior. Do not cache native elements across virtualization. Verify the
actual emitted method in a fake visual tree: first match stops before later
branches, nested and missing cells resolve correctly, and no collection is built.
Native timing must be measured again; reduced traversal is not itself a claim
that the latency acceptance target is met.
