### Added — default-off Flutter diagnostic timing

The Flutter phone host may collect bounded stage timings when built with the
explicit `MOSAIC_NATIVE_DIAGNOSTICS=true` Dart define. Ordinary builds do not
construct the timing stopwatches or emit diagnostic records.
