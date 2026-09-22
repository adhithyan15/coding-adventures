### Fixed — generate:root-slug-splits refused the fix check told you to make (HL-C419)

`--check` reported seventeen entries that had **lost** a spelling and said to
run `generate:root-slug-splits`. That command refused, counting each one among
"new split(s)" and pointing at `--allow-new` — the flag that exists to make
accepting a genuine regression visible. A correct normalisation had only the
escape hatch built for the incorrect one.

`diffRootSlugSplits` now returns `shrunk` beside `added`. A live slug list that
is a **strict subset** of its baseline list is a shrink: `--write` takes it with
no flag, `--check` still fails until it is recorded. Subset membership is
tested, not list length — `[a,b] -> [a,c]` is the same length and
`[a,b,c] -> [a,x]` is shorter, and both smuggle in a spelling nobody accepted.
All 17 were strict subsets; none was new.
