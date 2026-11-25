# Python microbenchmark

This package provides a paired collector and rank based analyzer for the VER07
reference experiment. `array-microbenchmark` compares indexed access with native
iteration over the same 4,096 element array. The collector accepts caller-owned
batch functions and an injectable monotonic clock for deterministic tests.

Install with `uv pip install -e .` after installing the sibling
`rank-statistics` package. Run `array-microbenchmark --help` for the command
options, or call `microbenchmark.collect` and `microbenchmark.analyze` from
Python.
