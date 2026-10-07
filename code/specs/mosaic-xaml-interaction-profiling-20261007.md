# Measure native interaction latency

The user reports lag when selecting VisiCalc cells. Before optimizing, measure
the generated native host, including Rust dispatch, synchronous snapshot
persistence, property application and the next WinUI rendering callback.

Add opt-in shared XAML host instrumentation, disabled unless
MOSAIC_PROFILE_PATH is set. Collect bounded timing/allocation samples in memory
and export on normal process exit, avoiding log I/O on the click path. Labels
must be fixed phase names; never log formulas, cell values, payloads or paths.
Diagnostics must not throw into application code. Next-render time is a frame
callback proxy, not proof of pixels being presented to the display.

Generate the real VisiCalc host from source and click populated/empty cells.
Report sample count, distribution and allocations. Separate tool overhead from
in-process measurements. Retain baseline evidence and use measured bottlenecks
to choose reusable fixes and regression benchmarks. Preserve persistence,
selection correctness, editor caret and native keyboard acceptance.
