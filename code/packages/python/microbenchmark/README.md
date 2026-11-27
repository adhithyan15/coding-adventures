# Python microbenchmark

This package provides a paired collector and rank based analyzer for the VER07
reference experiment. `array-microbenchmark` compares indexed access with native
iteration over the same 4,096 element array. The collector accepts caller-owned
batch functions and an injectable monotonic clock for deterministic tests.

Install with `uv pip install -e .` after installing the sibling
`rank-statistics` package. Run `array-microbenchmark --help` for the command
options, or call `microbenchmark.collect` and `microbenchmark.analyze` from
Python.

`Report.from_dict` accepts an already-decoded Python mapping, validates the
closed report shape, balanced pair order, analysis, and optional six-field
driver metadata group, then returns the core analysis result. Workload IDs
reject invalid Unicode, NUL, empty, and ASCII-whitespace-only values while
accepting other Unicode identifiers. Invocation may be an empty array and may
contain empty strings. This object API cannot inspect duplicate keys in the
original JSON bytes and leaves the input mapping unchanged. `analyze` accepts
at most 10,000 pairs, matching the collector and report limits; the generic
rank-statistics API has no microbenchmark pair cap.
