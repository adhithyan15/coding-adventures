# coding-adventures-microbenchmark

Lua 5.4 library and CLI for paired microbenchmarks, strict JSON report replay,
and the `indexed-array-sum-4096` workload. It depends on the sibling
`coding-adventures-rank-statistics` and `coding-adventures-json-parser` rocks.

```lua
local benchmark = require("coding_adventures.microbenchmark")
local report = benchmark.collect(baseline, candidate, expected_checksum, config)
local encoded = benchmark.to_json(report)
local replayed = benchmark.from_json(encoded)
```

Collection is deterministic under an injected nanosecond clock and reports no
partial result on invalid clocks, callback failures, checksum mismatch, or
integer overflow. Native CLI timing uses an in-process monotonic wall clock
(QPC on Windows and `CLOCK_MONOTONIC` on POSIX); `os.clock()` measures CPU time
and is unsuitable here. The included C module is linked against the Lua 5.4
API. Reports preserve signed-64-bit fields exactly, reject duplicate JSON keys,
and are replayed by recomputing the analysis from raw pairs. A linear,
quote-aware depth and token preflight bounds input before recursive parsing.
The shared fixture is copied to `testdata/`; tests verify its bytes against the
canonical source.

Build prerequisites: Lua 5.4, LuaRocks, a C compiler, and the Lua 5.4 headers
and import/development library. Run `BUILD` on POSIX or `BUILD_windows` from
PowerShell/cmd. Load the rock tree paths with `eval "$(luarocks path)"` on
POSIX; on Windows, set `LUA_PATH` and `LUA_CPATH` from `luarocks path --lr-path`
and `--lr-cpath`. Then invoke `lua script/array-microbenchmark --pairs 40`.
`tests/installed-consumer/Consumer.lua` is the external consumer smoke test.
Benchmark analysis accepts at most 10,000 pairs while rank statistics remain
uncapped. The CLI reads at most 4 MiB plus one byte from a replay path before
UTF-8 validation and JSON parsing.
Report replay preserves any nonblank producer-supplied workload ID; the native
array CLI continues to emit `indexed-array-sum-4096`.

`BUILD` measures LuaCov 0.17.0-1 executable-line coverage for
`src/coding_adventures/microbenchmark/init.lua` only and requires more than
80% with a nonzero denominator. The native C clock shim, test/CLI wrapper files,
and sibling dependency modules are excluded; their native and installed
consumer checks still run separately.
