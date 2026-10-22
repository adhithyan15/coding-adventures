# XAML projection diagnostic counters

For #16865, expose opt-in cumulative counts on each generated component that owns
row projections. Count projection reads, rows created (append or replacement),
retained rows refreshed, and rows removed (including null sources). Nested row
projections accumulate into their owning component, so one snapshot covers the
whole projected tree. Enable only when MOSAIC_PROFILE_PATH is nonblank; keep the
standalone component independent of MosaicRuntimeHost. Counters contain no data,
labels or payloads, perform no I/O, and are UI-thread diagnostics rather than
thread-safe application state. Expose a read-only tuple snapshot for measurement.

These count view-model projection work, not realized native elements. They do not
establish native latency acceptance. Validate append/reuse/replacement/removal
instrumentation and nesting through generated-source tests, the emitter suite
and downstream package-builder suite. A later native profiling session must
correlate snapshots with warm Release/Debug interactions and realization traces.
