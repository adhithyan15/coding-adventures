# trig-reference-v1

The bit-exact conformance corpus of [PHY00a](../../PHY00a-trig-convergence.md)
(TRIG-REF-1). Every `trig` port must return **exactly** the bits in the `y` column.
It extends, and does not replace, the tolerance corpus
[`phy00-phy01-v1`](../phy00-phy01-v1/README.md).

Do not edit the `.tsv`, `manifest.json` or `schema.json` by hand. They are generated
and hashed, and `.gitattributes` marks them `-text` so no checkout rewrites a line end.

## Row format

One vector per line, five tab-separated fields, `\n` line ends, UTF-8 without BOM:

    op <TAB> x <TAB> y <TAB> cr <TAB> flags

| Field | Content |
|---|---|
| `op` | `sin cos tan radians degrees sqrt atan atan2` |
| `x` | the input's IEEE binary64 bits, 16 lowercase hex digits. `atan2` has `<y>,<x>` (33 characters, the first argument first) |
| `y` | the TRIG-REF-1 result bits, 16 lowercase hex digits. A NaN result is written `7ff8000000000000` and matches **any** NaN (VER04p `same`); `error` for `sqrt` of a negative |
| `cr` | the correctly rounded value of the mathematical function at the exact input, 16 hex digits, or `-` when none is claimed (NaN or infinite result, a `pole-sentinel` row, Part II) |
| `flags` | comma-separated, closed set: `nan-class`, `nonfinite=unrepresentable` (an input that cannot exist as a BEAM float; such a row never also carries `nonfinite=rejects`), `nonfinite=rejects` (a non-finite result from a representable input), `out-of-domain` (finite `|x| > 2^29` given to `sin`/`cos`/`tan`), `pole-sentinel` (a `tan` result of +-1e308), `beam-raises` (`atan2` whose `y/x` overflows), `signed-zero` (the result is +-0) |

A decimal number, an exponent letter, a `0x` prefix, upper-case hex or whitespace inside a
field is a corpus error (one value has one spelling, so hashes are stable).

## Groups

| File | Rows | SHA-256 |
|---|---|---|
| `edge.tsv` | 2854 | `23355a9744a4bfba39343c3c872da0c7b784cebdc75b4ec10a33d6c71f71a1bd` |
| `sweep-binade.tsv` | 19520 | `e3a007d76cd4d70d102fa0bc40c0e1ad1d833100abc96e432582965400e80113` |
| `sweep-cw.tsv` | 19300 | `992c64127b129ced1c780136b28834dfe444bc8cc2dbf054b4ff9df1decc03e5` |
| `sweep-prng.tsv` | 20480 | `66af28eb2004710e1654c352fd4e41304e219e91127c4a036cc9238175cf2d43` |

`edge.tsv` holds, for each of `sin cos tan radians degrees`, VER04p 11.1's edge list
plus the trig-specific points of PHY00a 6.3 (including the directed *witnesses* for the
constants and fused sites a sweep sees only rarely, see PHY00a 6.4), then the Part II rows
(`sqrt`, `atan`, `atan2`; no `cr` is claimed for these, PHY00a 6.2). The three sweeps are bounded sets (never "exhaustive"): `sweep-binade`
(`2^e (1 + j/32)`, `e` in [-30, 30], both signs), `sweep-cw` (neighbours of `fl(k pi/2)`
for `k` up to 3.4e8, the continued-fraction hard cases, and 512 binade-drop inputs
`fl(k pi/2 + r)` with `r` just above 1/2, 1/4, 1/8), and `sweep-prng` (1,024 raw bit
patterns, 1,024 log-uniform, 2,048 uniform in [-2^29, 2^29]). `manifest.json` records
each group's row count and SHA-256, the constant table of PHY00a 3.2, the hash of every
input list, and the measured accuracy.

### The PRNG

xoshiro256** whose four state words are the first four outputs of SplitMix64 seeded with
the integer `0x545249472d524546` (the ASCII bytes `TRIG-REF`, big-endian). `sweep-prng` starts a
stream from that seed; the binade-drop part of `sweep-cw` starts a second stream from the
seed XOR 1. Log-uniform: `u = next()`, sign `u >> 63`, exponent `-27 + u mod 56`, mantissa
`next() & (2^52 - 1)`. Uniform: `n = next() >> 11`, value `(n - 2^52) 2^-23`. Binade-drop:
`k = 2^26 + next() mod (340000000 - 2^26 + 1)`, `u = next() >> 11`, `s = next() >> 63`,
`r = c (1 + u / 2^53 / 16)` for `c` cycling over 1/2, 1/4, 1/8, input
`(-1)^s fl(k pi/2 + r)` rounded once, from exact arithmetic.

## Measured accuracy (every row that has a `cr`)

| Op | Worst error vs the exact value | Worst steps vs `cr` | Gate |
|---|---|---|---|
| `cos` | 2.3738 ulp at `41b08a633012e355` | 2 at `41b08a633012e355` | 3 |
| `degrees` | 0.8042 ulp at `40916a693ef35acc` | 1 at `401f6a7a29553860` | 1 |
| `radians` | 0.6486 ulp at `41ac61fd6279f390` | 1 at `3fefffffffffffff` | 1 |
| `sin` | 2.3829 ulp at `41bb531ad2e28013` | 2 at `41bb531ad2e28013` | 3 |

## Independence

`y` is computed by an integer-only soft-float model of the algorithm
(`code/scripts/trig_ref_softfloat.py`) and cross-checked against two unrelated host
implementations (`code/scripts/trig_reference_xcheck/xcheck.c` built with
`-ffp-contract=off`, and `xcheck.rs`); the generator stops on any disagreement. `cr` is
computed from exact rational arithmetic (`code/scripts/trig_ref_exact.py`, which has its own
binary64 encoder and decoder and imports nothing from the model) and cross-checked
against GNU `bc -l` at 100 digits (`tan` as `s(x)/c(x)`). No host `sin`, `cos` or `float()`
touches a corpus value.

## Regenerate and check

    python3 code/scripts/generate_trig_reference_fixture.py          # write, run the gates and cross-checks
    python3 code/scripts/generate_trig_reference_fixture.py --check  # regenerate in memory; fail on any byte of difference
    python3 -m pytest code/scripts/tests/test_trig_reference_fixture.py

## What a port does

Decode `x` from hex by a bit cast, call the public function, encode the result back to
bits and compare as integers (NaN as a class). Skip `nonfinite=unrepresentable` rows (BEAM);
on `nonfinite=rejects` and `beam-raises` rows require the raise. PHY00a 6.5 has the rules.
