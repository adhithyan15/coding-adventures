# Elixir microbenchmark

Paired in-process batch measurements with reusable analysis and exact JSON replay. `CodingAdventures.ArrayWorkload` exposes indexed and enumerable callbacks over the supplied persistent Erlang `:array`; construct it before collection. On Windows, the CLI reads OTP's high-resolution performance counter (QueryPerformanceCounter on this runtime) because `System.monotonic_time/1` is quantized at roughly 102 microseconds on some hosts. The counter is sampled relative to a start value, and every reading is checked for regressions; a regressing host counter fails the run without emitting a report. Other platforms use `System.monotonic_time/1`.

Run `BUILD` or `BUILD_windows` to test the library and build the escript. `mix escript.build` creates `array-microbenchmark`.

Benchmark analysis accepts at most 10,000 pairs; the separate rank-statistics
API has no sample-count cap. CLI replay reads at most 4 MiB plus one byte
before decoding the report.
The JSON preflight counts root, container, scalar, and string values before
`:json.decode` allocates the decoded structure; object keys are excluded.
Scalar preflight tracks only token state and does not copy untrusted scalar bytes.
