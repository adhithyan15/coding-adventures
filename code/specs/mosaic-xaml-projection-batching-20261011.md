# XAML projection notification batching

Native VisiCalc profiles (#16865) show 288–369 ms to the next rendering callback,
with roughly 7.6–8.5 MB of managed allocations. This experiment addresses repeated
row projection invalidation during one runtime props application; it does not
claim that this is the only cause or that native keyboard acceptance is complete.

Generated components with row projections implement System.ComponentModel's
ISupportInitialize. BeginInit/EndInit nest; projection notifications collect in
insertion order, deduplicate, and flush at the outermost EndInit. Changes outside
a batch notify immediately. Clear the pending collection before invoking handlers
so reentrant changes belong to their own batch. An unmatched EndInit is an error.

The shared runtime brackets property assignment with this optional interface,
using finally to end the batch even when conversion or assignment fails. Required
property presence checks precede BeginInit. Components without the interface retain
their behavior. This batches notifications, not mutations: failures do not roll back
already assigned properties. Snapshot persistence and adapter dispatch are unchanged.

Validate generated C# batching behavior (nesting, deduplication, coherent values,
immediate notification and reentrancy), runtime exception cleanup, emitter tests,
and generated native build. Compare actual native profile samples before claiming
a performance improvement. Native focus and navigation remain separate acceptance
gates under #14278.

## Experiment results

Generated from the local candidate after merging origin/main (c783b73b81), using
its generated host and the retained adapter DLL, isolated copied workbook and
Debug x64 WinUI. Build: zero errors, 20 binding warnings. Compiled probes cover
nested batching, ordered deduplication, immediate changes, reentrant handlers,
unmatched EndInit, runtime assignment within the batch and setter-failure cleanup.
Emitter suite passed 382 unit tests plus integration tests; bindings passed 44
unit tests and the compiled effect driver including props-batch.

Actual Computer Use clicked G6 then sent Right. Selection again moved to G5 and
scrolled horizontally; UIA still named formula-field as focused. Batching does
not resolve #14278. Normal exit flushed the accompanying JSONL file. The two
next-render callback samples were 187.9267 and 251.0274 ms, allocating 8,446,656
and 7,666,224 managed bytes. Handler durations were 15.2677 and 6.3967 ms.

Earlier retained-binary samples were 369.0762 and 288.1936 ms. This is NOT a
controlled speedup claim: the base was refreshed, samples are tiny, first-input
warmup varies, and callbacks are not presented pixels. Allocations remain high.
Keep #16865 open; profile projection identity/realization next. This change
provides coherent, deduplicated projection notification within one props update,
not stable VM identity or asynchronous snapshot persistence.
