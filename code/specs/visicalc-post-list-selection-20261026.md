# VisiCalc selection after equivalent-list reuse, 2026-10-26

For #16865 and epic #14267. Both populated-keyboard runs meet the initial
50 ms p95 target, but occasional tails remain. This receipt does not close
performance, design, accessibility, durability or native release acceptance.

## Candidate and protocol

The retained generated candidate from #18294 uses the shared XAML host generator
with equivalent-list identity reuse (implementation a63fb132f8f, merged as
cbddd61e6e53bfc75668b809b2d54737904629b6). See
[generation provenance](mosaic-xaml-list-identity-20261026.md). Other generated
components and the October 16 Rust runtime were retained; this is not a claim
that current main or the complete runtime was rebuilt. No generated source was
manually edited. Release and Debug builds each passed with zero errors and two
existing CS0067 warnings. Managed VisiCalc.dll SHA256:

- Release: 51431AF4D73686617B6B8B410E6D913B5D05BE849E13A82924101ECEF1261240
- Debug: 1DD4079EEAD5DCF0221F826F576E2F769C78C0871B4573E363588058B48CEC70
- Shared Rust DLL: A18198012F2985EC5CEA6E10A2151388D2E9093F567A59F46A72E45C28BC2124

Repeat the [prior populated workload](visicalc-warm-selection-20261026.md):
1908 by 1019 screenshot pixels, dark theme, 100% app text, initial Sheet 1.
Each process uses fresh distinct state/profile paths and
MOSAIC_PROFILE_INTERACTIONS_ONLY=1. Persistence stays enabled. Startup A1=15;
click B2=14; ten alternating Right/Left warm-up keys; fifty measured alternating
keys; inspect final screenshot; normal close and verify process exit.
Every one of the sixty keyboard transitions per configuration was individually
issued and observed through Computer Use, alternating C2=7 and B2=14 correctly.
The final screenshots showed B2=14 with intact headers/grid. Screenshots were
inspected in chat; no screenshot files are claimed.

Runs were sequential Release then Debug, with other applications open. System
load and input cadence were not controlled. Accessibility observations are
outside the native timer but may perturb execution. This is a matching protocol
comparison, not randomized evidence that list reuse caused all latency changes.

## Measurements

Each adjacent raw file contains 124 alternating handler/next-render records:
startup viewport, B2 click, ten warm-up pairs, then fifty measured pairs.
Quantiles use nearest rank ceil(p*N), N=50. Empirical p99 is the maximum, not a
stable population-tail estimate. Next-render means the WinUI Rendering callback,
not compositor presentation. Overlapping scopes must not be added; bytes count
current-thread managed allocation, not retained/native/whole-process memory.

| Build / scope | p50 ms | p95 ms | p99/max ms | p50 bytes | p95 bytes | p99/max bytes |
|---|---:|---:|---:|---:|---:|---:|
| Release handler | 8.2925 | 10.5240 | 10.7604 | 371512 | 371544 | 371552 |
| Release next-render | 18.4884 | 22.8189 | 117.0611 | 379968 | 403552 | 417160 |
| Debug handler | 7.6317 | 9.3043 | 85.4219 | 371520 | 371552 | 379064 |
| Debug next-render | 17.6592 | 22.5404 | 98.7927 | 380168 | 418824 | 428480 |

All measured scopes: projection reads/created/reused/removed = 20/0/513/0;
realization prepared/cleared = 0/0. The prior receipt reported 21/0/539/0 and
0/0. Lifecycle diagnostics are not a census of native descendants. Fewer
projection reads do not mean zero traversal, layout or binding work. Median
allocation increased slightly from the prior run, despite reduced projection
work; do not claim an allocation reduction.

All measured next-render intervals above 50 ms:

| Build | Measured selection | Handler ms | Next-render ms |
|---|---:|---:|---:|
| Release | 32 | 7.9939 | 117.0611 |
| Debug | 8 | 85.4219 | 98.7927 |
| Debug | 25 | 6.3136 | 81.1045 |

Release p95 was 115.3661 ms in the prior receipt; now 22.8189 ms. Debug was
33.4671 ms; now 22.5404 ms. One remaining tail includes a long handler; two
occur mostly after handler completion. Filtered data cannot attribute these
to persistence, GC, scheduling, layout or contention. No causal elimination
claim follows from this single pair of runs.

## Reproduce and preserve

- [Release raw profile](visicalc-post-list-selection-release-20261026.jsonl)
- [Debug raw profile](visicalc-post-list-selection-debug-20261026.jsonl)

Run from repository root with standard-library Python:

```python
import json
import math
from pathlib import Path

for build in ("release", "debug"):
    path = Path(f"code/specs/visicalc-post-list-selection-{build}-20261026.jsonl")
    rows = [json.loads(line) for line in path.read_text().splitlines()]
    assert len(rows) == 124
    assert all(row["phase"] == ("interaction-handler" if i % 2 == 0
                               else "interaction-next-render")
               for i, row in enumerate(rows))
    for phase in ("interaction-handler", "interaction-next-render"):
        samples = [r for r in rows if r["phase"] == phase][12:]
        assert len(samples) == 50
        assert all(r["projection"] == dict(reads=20, created=0, reused=513, removed=0)
                   and r["realization"] == dict(prepared=0, cleared=0)
                   for r in samples)
        for field in ("milliseconds", "bytes"):
            values = sorted(r[field] for r in samples)
            print(build, phase, field,
                  [values[math.ceil(p * len(values)) - 1] for p in (.5, .95, .99)])
```

Original files remain in
C:/Users/adhit/.codex/worktrees/mosaic-xaml-equal-lists-20261026-output/:
warm-release-post-01.jsonl and warm-debug-post-01.jsonl. Source SHA256 before Git
newline normalization: Release
9B6C408111A1B5E3F3A059A3772F84447EBAB3A7EBD530D864C43D789E6CD460;
Debug 19CFC23A45F043AD6B42E13180ABF848966A57CA1252977000E46A8B32EE2D43.

## Remaining acceptance

Keep #16865 open. Extend distributions to blank cells, editing, scrolling and
larger viewports; repeat populated measurements and investigate tails without
weakening durability. Earlier formula/restart smoke is not complete Save/Open
or workbook-command acceptance. #14282 native release promotion remains gated
on these requirements plus full design, accessibility and downloaded-artifact
launch tests. This receipt adds evidence only; no application semantics change.
