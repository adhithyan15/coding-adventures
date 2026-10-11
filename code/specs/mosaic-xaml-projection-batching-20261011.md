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
