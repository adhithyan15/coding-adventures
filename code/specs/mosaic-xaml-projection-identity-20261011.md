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


## Completed native validation and lookup profile

The interrupted-validation status above is historical. Head 1068c32889 later
passed populated/blank selection, G6 to H6 keyboard navigation, formula commit
and normal-exit restart persistence. H7 restored =6*7 and 42, confirmed visually
and in UIA with horizontal scrollbar 0. Its final profile is committed as
mosaic-xaml-projection-filtered-profile-20261011.jsonl. Later selection samples
were 129.7900-175.5680 ms. CI had 41 passed and 24 skipped on that head.

After the early-return lookup and refresh from main, all 383 emitter unit tests
and integrations passed. The emitted-method probe uses 1,001 branches and proves
finding its first cell visits just one child; nested and missing cells are also
checked. Generated Release WinUI built with zero errors/two unused-event warnings.
Security review passed. Native B2, E5, blank F6, A1, C3 and Right to D3 rendered
correct addresses/highlights; existing H6/H7 formulas survived startup.

Next-render callback milliseconds in order: 223.9854, 136.2198, 22.5265, 10.6993,
14.2523, 16.9592. Bytes: 479672, 654128, 706160, 402312, 525248, 390424. Raw phases:
mosaic-xaml-focus-lookup-profile-20261011.jsonl. This small sequential sample is
not a p95 acceptance claim. First selections remain slow. More warmed samples,
late rows, scrolling, editing, realization counts and persistence stalls remain.
Retained Rust DLL, callback and allocation caveats still apply. Keep PR draft.

## Retained focus candidate follow-up

Queued pointer and keyboard focus may reuse the original realized cell only if its current row and column still match the requested coordinates and its current nearest table ancestor is the dispatch table. Otherwise use the existing first-match lookup. The candidate lives only for the queued callback; there is no persistent native-element cache. This avoids a full preceding-tree walk for retained late-row cells while preserving replacement, detach, reparent and coordinate-change behavior. Compile the emitted resolution helper with executable tests for all those cases, then validate generated WinUI and native interaction before claiming performance acceptance.

### Retained focus validation

All 383 emitter unit tests and integration tests pass, including the executable
emitted resolution helper cases. Generated Release WinUI builds with zero errors
and two existing unused-event warnings; security review passed. Actual native
A1, B2, W20, A20, Right-to-B20, L10 and E5 selections displayed correct labels and
highlights. H6/H7 persisted values remained present. Next-render callback samples
were 18.7307, 38.9987, 29.5246, 27.9534, 39.8952, 28.2211 and 218.3981 ms, with
396624, 388720, 388272, 387944, 391656, 388560 and 388816 current-thread managed
bytes. The final E5 sample included 198.5781 ms in persist-snapshot; it is retained
as an outlier, not discarded. Raw phases are in mosaic-xaml-retained-focus-profile-20261011.jsonl.
This small matched-sequence diagnostic supports reduced late-cell focus work but
does not establish p95 acceptance. Persistence stalls remain open in #16865.

## Persistence phase diagnostics

Keep synchronous snapshot persistence, exception handling, warning recovery and
atomic replacement behavior unchanged. Under the existing opt-in profiler, add
nested persist-snapshot-ffi, persist-encode, persist-write and persist-replace
scopes. Record only phase duration and managed allocation, never workbook bytes
or file paths. This separates adapter serialization from disk stalls before
choosing a durability-preserving performance change. Existing aggregate
persist-snapshot remains comparable with older native traces.

Persistence-phase validation: 44 bindings unit tests and the emitted-XAML effect
driver pass; generated Release WinUI builds with zero errors (two existing unused
event warnings), and security review passed. Native A20 then E5 interactions
verified selection and emitted every nested phase. Render callbacks were 26.9731
and 17.9705 ms; persistence was 1.9580 and 2.1042 ms. This two-event smoke check
did not reproduce the earlier stall and does not resolve performance acceptance.
Raw samples: mosaic-xaml-persistence-phases-profile-20261012.jsonl. Next step is a
longer matched run to capture the intermittent outlier with the new breakdown.
