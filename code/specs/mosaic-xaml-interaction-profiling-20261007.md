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

## Measured baseline, 2026-10-07

Generated x64 Debug WinUI app, bundled existing VisiCalc Rust DLL, isolated
restored workbook, 20 visible rows and 26 columns. Five explicit selections
(B2, E5, blank F6, A1, C3) produced six handler/next-render samples; attribution
of the extra event requires follow-up. No event names or payloads were logged.
The first two render intervals overlap and must not be added together.

Raw data: mosaic-xaml-interaction-profile-baseline-20261007.jsonl.

- Next-render intervals: 1090.314, 839.6526, 468.9264, 273.8088, 358.3973,
  241.7521 ms. The last four allocate 8,450,504-8,454,880 managed bytes on the UI
  thread during the interval. This is not retained heap size or native allocation.
- FFI dispatch after startup: 0.7797-1.1655 ms.
- Property application after startup: normally 7.8825-12.6171 ms, one 149.5486 ms
  outlier. Synchronous handler normally 10.7195-17.6529 ms, one 508.6726 ms outlier.
- Snapshot persistence normally 1.5421-3.4498 ms; one 357.2592 ms stall.

This confirms substantial UI latency. Most warm delay occurs after the handler;
source inspection shows list props are reconverted/reassigned on every update,
row projection getters allocate all-new row VMs, and selection-slot changes
invalidate those projections again. UI rebuild/layout churn is the leading
hypothesis, not yet an isolated causal measurement. Persistence also stalls the
UI thread. Next steps: measure projection counts and layout separately; batch
updates and preserve row identity where valid; investigate ordered/coalesced
persistence without weakening durability. Compare before/after in Release too.
Target a warm selection-to-render p95 below 50 ms initially, with work bounded
by changed/visible cells and a larger reproducible sample before claiming it.

Instrumentation acceptance: generated WinUI build 0 errors/20 binding warnings;
compiled C# host tests cover disabled mode, valid bounded JSONL export, duplicate
Dispose, and invalid export destination without application failure. This
baseline does not satisfy native keyboard or release acceptance.
