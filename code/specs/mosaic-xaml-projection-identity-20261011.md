# XAML projection identity (2026-10-11)

Follow-up to #16865 and epic #14267. Batching alone still measured 269–391 ms
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
2.35 MB but still took 159–244 ms on six selections and 1221 ms on one stalled
selection. Retained rows must also compare computed helper values and notify
only those that changed. Cache the last observed helper result, update it before
raising the event, and leave direct getter reads current. Nested projection
refreshes still propagate owner changes to descendants. Add a compiled check
that an unchanged helper does not notify while changed helpers do.
