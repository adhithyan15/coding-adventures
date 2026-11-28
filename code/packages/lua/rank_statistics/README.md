# coding-adventures-rank-statistics

Pure Lua 5.4 medians and exact distribution-free confidence intervals for a
population median. `median(values)` sorts a copy of a dense array and rejects
empty or non-finite samples. `median_interval(values, confidence_percent)`
returns a closed `{low, high}` observation pair or `nil` when no rank meets the
confidence cutoff. The binomial-tail comparison uses base-1e9 integer limbs,
so it remains exact beyond Lua's signed 64-bit integer range.

```lua
local stats = require("coding_adventures.rank_statistics")
local center = stats.median({ 4, 1, 3, 2 })
local interval = stats.median_interval({ 1, 2, 3, 4, 5, 6, 7, 8 }, 95)
```

Run `lua tests/run.lua` for direct sign-enumeration conformance, frozen wide
cutoffs, floating-point boundaries, and seeded generated samples.
`tests/installed-consumer/Consumer.lua` is a separate-process smoke test for
the installed rock.

## Coverage evidence

On Windows x64 with Lua 5.4.5, LuaCov 0.17.0-1 and its Cluacov 1.0.0-1 C hook,
the installed `coding_adventures.rank_statistics` module measured 100.00%
executable-line coverage (77 hit, 0 missed). The report includes only
`coding_adventures/rank_statistics/init.lua` using the include pattern
`coding_adventures/rank_statistics/init$`; LuaCov reports line coverage, not
branch coverage. The module source is unchanged from snapshot
`d98f0030a92e6002a4eef47a982770985f58ff20`. The full test suite passed with
1,188 bounded cutoffs, 9 wide reference vectors, and 300 generated samples. In
the task-local test runner,
the hook is disabled only while the independent sign-enumeration oracle counts
sign assignments, then restored before every `median_interval` call. That
oracle block makes no calls into this library, so every library call remains
instrumented.

`BUILD` measures LuaCov 0.17.0-1 executable-line coverage for
`src/coding_adventures/rank_statistics/init.lua` only and requires more than
80% with a nonzero denominator. Test, sibling dependency, and C-source files
are outside this library metric. BUILD first runs the complete suite normally;
the instrumented pass uses representative vectors instead of the three
10,000-value stress vectors, which remain covered by the normal suite.
