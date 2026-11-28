# HWEB11 UTF-16 scanner transition

Run from the repository root with the pinned tools:

```sh
HWEB_GO=/path/to/pinned/go/bin/go \
  /path/to/pinned/python3 \
  code/packages/fsharp/historical-web/verification/sat_utf16_advance.py
```

The model follows `HtmlTags1992.validateUnicode` in
[`../HistoricalWeb.fs`](../HistoricalWeb.fs). At a scanner step, let
`remaining = source.Length - index`, where the loop condition establishes a
current code unit (`remaining >= 1`). For a high surrogate, the source rejects
the step when `index + 1 >= source.Length` and otherwise checks the next code
unit is low before advancing `index` by two. The model bit-blasts the equivalent
32-bit `remaining - 2` transition. The positive SAT case assumes the current
unit is high and the next unit is low; the result queries the bounds condition
only. Removing the next-unit bounds check admits `remaining = 1`, which would
advance beyond the source end.

This is a one-step range model, not a proof of F# or .NET indexing semantics,
surrogate classification, parser-wide Unicode validation, or the complete
HWEB00 implementation.
