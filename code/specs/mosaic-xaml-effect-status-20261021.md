# XAML deferred-effect status refresh

## Problem

The native VisiCalc Save picker completes successfully, but the generated window
continues showing the persistence warning captured while the file effect was
pending. Cancellation has the same stale diagnostic. Evidence is recorded in
GitHub issue #14279. The generated window's RefreshProps callback discards the
status returned by ApplyRequiredProps.

## Contract

After a deferred effect completes, the shared native-complete XAML UserControl
window must apply the latest properties and display the status returned by that
same host call. This status includes any remaining persistence or effect failure;
the window must not replace it with an unconditional success message. Existing
unavailable-runtime guards, UI-thread dispatch, and exception reporting remain.
No application-specific branches or generated-output edits are needed.

## Validation

Check the generated callback, including its unavailable-runtime guard and error
path. Run XAML emitter tests. Native Save/cancel and Save/Open validation must be
repeated on regenerated output before claiming native acceptance. This change
does not complete #14279 or the release gate #14282.
