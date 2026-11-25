# Python microbenchmark

This package provides a paired collector and rank based analyzer for the VER07
reference experiment. `array-microbenchmark` compares indexed access with native
iteration over the same 4,096 element array. The collector accepts caller-owned
batch functions and an injectable monotonic clock for deterministic tests.

Install with `uv pip install -e .` after installing the sibling
`rank-statistics` package. Run `array-microbenchmark --help` for the command
options, or call `microbenchmark.collect` and `microbenchmark.analyze` from
Python.

`Report.from_dict` checks an optional producer workload identifier before
returning its core analysis result. It rejects invalid Unicode, NUL, empty or
ASCII-whitespace-only identifiers and accepts other Unicode identifiers without
changing the input mapping. This value API does not inspect duplicate raw JSON
keys.
