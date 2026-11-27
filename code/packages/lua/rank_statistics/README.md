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

`BUILD` measures LuaCov 0.17.0-1 executable-line coverage for
`src/coding_adventures/rank_statistics/init.lua` only and requires more than
80% with a nonzero denominator. Test, sibling dependency, and C-source files
are outside this library metric.
