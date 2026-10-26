# VisiCalc warm native selection receipt, 2026-10-26

For #16865 and epic #14267. The populated-cell Release run still misses the
initial 50 ms selection-to-next-render p95 target. Row projection identity is
preserved and no repeater lifecycle callbacks occur in these measured selections.
This receipt records a failure boundary, not completion or release acceptance.

## Artifact and workload

Both x64 WinUI configurations were built from the same generated XAML sources
from implementation commit 9182081a72cc3ddbe94e77e44a5d4e983c334fd4, later merged
by #18278 (663f824ac18169945401d137034ae89248b02a8e). The final PR head
b679c6f3b0a00d8cd8746bd02ca57ccad55930d9 only changes lesson metadata after that
implementation. No generated source was edited. Both builds reported zero errors
and two existing CS0067 warnings. These are retained native candidates, not a
claim that the current repository head was rebuilt.

The unchanged October 16 Rust runtime was reused in both configurations:
SHA256 A18198012F2985EC5CEA6E10A2151388D2E9093F567A59F46A72E45C28BC2124.
Managed VisiCalc.dll SHA256:

- Release: BE7BFD7505479361BABF71F03BE7C50D87001F15E8D7EFC4655B0BCB01497D53
- Debug: A49BE2396433C36824B27A0FB894C85698F4227AD00CA8FF8AF5C41DB9784E9E

The desktop window was maximized, 1908 by 1019 screenshot pixels, dark theme,
100% app text size, initial populated Sheet 1. Other desktop applications remained
open; CPU, GC, disk and compositor load were not controlled or recorded. Each
process used a distinct fresh MOSAIC_APP_STATE_PATH and profile output path,
with MOSAIC_PROFILE_INTERACTIONS_ONLY=1. Persistence was not disabled.

Real Computer Use sequence, separately for Release then Debug:

1. Observe startup A1=15. Click B2=14 to focus the grid.
2. Warm up with ten individual alternating Right/Left presses (C2=7 / B2=14).
3. Measure fifty further alternating Right/Left presses, twenty-five round trips.
4. Inspect accessibility document text after every press: all values alternated
   correctly. Inspect the final screenshot: B2=14 selected, headers and grid intact.
5. Close normally to export the bounded in-process profiler queue; confirm exit.

Inputs were individually issued and observed, not a held key or rapid burst.
Automation calls and observations are outside the in-process timer, but their
concurrent UI accessibility work may perturb the app. Runs were sequential and
not randomized; this is not evidence that Debug is faster than Release.

Each raw file has exactly 124 records: 62 handler/next-render pairs in alternating
order. Pair 1 is startup viewport work, pair 2 the B2 click, pairs 3-12 warm-up,
and pairs 13-62 the fifty measured keyboard selections. This ordering is the
workload attribution; the profiler deliberately does not log event payloads.

## Results

Nearest-rank empirical quantiles: sorted value at ceil(p*N), one-based, N=50.
At this sample count p99 equals the maximum; it is not a stable population tail
estimate. Handler and next-render scopes overlap and must not be added together.
Next-render is the next WinUI Rendering callback, not compositor presentation.
Bytes are current-thread managed allocation during each interval, not retained
heap size, total process allocation or native allocation.

| Build / interval | p50 ms | p95 ms | p99 / max ms | p50 bytes | p95 bytes | p99 / max bytes |
|---|---:|---:|---:|---:|---:|---:|
| Release handler | 8.7410 | 11.2339 | 113.1564 | 364600 | 364632 | 372664 |
| Release next-render | 21.0468 | 115.3661 | 125.7931 | 373056 | 410216 | 422312 |
| Debug handler | 9.0141 | 17.8425 | 110.3650 | 364608 | 364640 | 372272 |
| Debug next-render | 20.2829 | 33.4671 | 124.4266 | 373064 | 414664 | 421696 |

Every measured interval in both phases/configurations reports projection
reads=21, created=0, reused=539, removed=0 and realization prepared=0, cleared=0.
These are the emitted diagnostic contracts: lifecycle callbacks are not a census
of all native descendant controls or proof of visibility. Stable identities do
not imply zero list traversal, binding updates, layout, or allocation.

All measured next-render intervals above 50 ms:

| Build | Measured selection | Handler ms | Next-render ms |
|---|---:|---:|---:|
| Release | 38 | 113.1564 | 125.7931 |
| Release | 40 | 8.5383 | 117.0554 |
| Release | 49 | 7.8789 | 115.3661 |
| Debug | 28 | 110.3650 | 124.4266 |
| Debug | 48 | 107.1951 | 120.9782 |

Three tails include a long handler; two Release tails occur mainly between handler
completion and the next render callback. The filtered profile cannot attribute
them to persistence, GC, scheduling, layout, or desktop contention. Do not infer
a causal fix from the zero realization deltas. A detailed-phase follow-up and
repeated matched runs should separate these mechanisms before changing semantics.

## Reproduce the arithmetic

Raw receipts are adjacent:

- [Release](visicalc-warm-selection-release-20261026.jsonl)
- [Debug](visicalc-warm-selection-debug-20261026.jsonl)

From the repository root, using Python's standard library:

```python
import json
import math
from pathlib import Path

for build in ("release", "debug"):
    path = Path(f"code/specs/visicalc-warm-selection-{build}-20261026.jsonl")
    rows = [json.loads(line) for line in path.read_text().splitlines()]
    assert len(rows) == 124
    assert all(row["phase"] == ("interaction-handler" if i % 2 == 0
                               else "interaction-next-render")
               for i, row in enumerate(rows))
    for phase in ("interaction-handler", "interaction-next-render"):
        samples = [row for row in rows if row["phase"] == phase][12:]
        assert len(samples) == 50
        for field in ("milliseconds", "bytes"):
            values = sorted(row[field] for row in samples)
            print(build, phase, field,
                  [values[math.ceil(p * len(values)) - 1]
                   for p in (0.5, 0.95, 0.99)])
```

Original local files and build logs remain under
C:/Users/adhit/.codex/worktrees/visicalc-realization-20261026-output/.
Source JSONL SHA256 before Git newline normalization:
Release 797E84E4AEF138DEF09505E67ADF565E6B48CCBAC6D027AF9D357E895FE91B67;
Debug 5B3D7775B15F48D035EF88D4DF921C3738583F2434773A6989411C520BB82C27.
Screenshots were inspected in the chat; no screenshot files are claimed here.

## Remaining acceptance

#16865 stays open. This is a matched configuration receipt for populated keyboard
selections only. It does not cover blank cells, click distributions, editor caret,
formula commits, save/restart durability, scrolling or larger viewports. It is
not a same-setup before/after comparison with the October 7 baseline. Repeat and
extend the samples before claiming robust p95/p99 performance; isolate the tails
without weakening persistence. Native release promotion under #14282 remains gated
on those requirements and the other design/accessibility acceptance work.
