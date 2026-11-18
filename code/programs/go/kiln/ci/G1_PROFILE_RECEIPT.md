# G1 synthetic profile and candidate receipt

This is diagnostic evidence for the current 66,600-byte / 936-line synthetic
G1 admission slice. It is not the full protected Sandbox suite, a proof of host
contention, a broad guarantee that every runner meets the 14,000,000 bytes/s
floor, or proof of source equivalence.

## Baseline identity

- Diagnostic workflow source: `af31a9e268164fc2838c69428c27e33dd4ed26fc`.
- Runtime under test: `adadc8fc0b9783b3a829fbb4309182f558d4561b`.
- Gate source blob: `f834ff07849b8c1dd8b412d99e4e95e119176614`.
- Measure source blob: `c04b3a093c81262201ecb2f43d48f9a162312eff`.
- Fixture: 1,850 bytes, SHA-256 `dc8acd0de1d9b235c3924d0d71e2fcd565274e09ded9975570f429b548525aae`.
- Synthetic slice SHA-256: `6cd61f63fdec55ce37294ad061705394b1babda80e5368b17ecb5bd38e2207af`.
- Go 1.26.1 linux/amd64, default GOMAXPROCS, four vCPUs on both jobs.

The official admission binary was compiled and hashed before adding any
diagnostic test helper. Profile/precision binaries are distinct from the frozen
admission binaries. Both baseline jobs executed all required named tests and
workload markers; profiling continued after admission, and the original U26
failure status was retained. Their artifacts are run `44362035252`, jobs
`135005748354` and `135005747661`.

| Job | Host | Official slice | 4 MiB census | CPU profile samples |
| --- | --- | ---: | ---: | ---: |
| `135005748354` | Ubuntu 24.04.5, image `20261111.127.1`, Xeon 6975P-C | 20.0 MB/s, pass | 18.7 MB/s, ungated | 537 / 536 / 536 |
| `135005747661` | Ubuntu 26.04.1, image `20261111.196.1`, EPYC 7763 | 13.2 MB/s, fail (5.06166 ms) | 12.2 MB/s, ungated | 548 / 543 / 547 |

Counts are the profile sample totals (about 100 Hz), not the number of unique
stack records. Each CPU capture made 20 `BestPass1` calls. Separate two-call,
`memprofilerate=1` allocation captures completed on both hosts. Cgroup CPU
limits were unlimited, no throttling was recorded, and observed steal deltas
were zero; pressure readings are time-window associations only.

## Hot paths and candidate under evaluation

Across the three U26 CPU captures, `GrammarLexer.applyTransitions` accounted
for 10.22%, 12.15%, and 10.24% flat CPU samples. Aggregated profile data puts
1.78 of 16.38 CPU seconds flat in that function and about 1.83 seconds
cumulative through `tokenizeStandardInto`. `pprof -list` attributed about
930 ms to `grammar_lexer_f10.go:110` (copying a transition rule in a range),
580 ms to `:113` (scanning token triggers), and 140 ms to `:114`. U24 also
showed 1.02 seconds at `:110` and 1.12 seconds at `:113`.

## Candidate and paired measurement

The candidate `eedb854e2a5e6436be8ce6dc55295c902710d9e5` walks the immutable
ordered transition table by index and checks a nonempty mode guard before
scanning token triggers. Its production lexer blob is
`3fec22ab311048fc219823d714ce2fb0deefb8ec`; pinned baseline A is
`adadc8fc0b9783b3a829fbb4309182f558d4561b`, with lexer blob
`901967eefdc8dc4c05d43c89e77c5974c14b6c24`. The paired workflow source is
`09db4f83d2e8636b092afdebee4aa52ab8c24146`; run
[`44370322596`](https://github.com/adhithyan15/coding-adventures/actions/runs/44370322596)
used Go 1.26.1 on both hosts. The gate blob remained
`f834ff07849b8c1dd8b412d99e4e95e119176614`, measure blob
`c04b3a093c81262201ecb2f43d48f9a162312eff`, fixture SHA-256
`dc8acd0de1d9b235c3924d0d71e2fcd565274e09ded9975570f429b548525aae`, and
66,600-byte / 936-line slice SHA-256
`6cd61f63fdec55ce37294ad061705394b1babda80e5368b17ecb5bd38e2207af`.
Each host ran the unchanged official gate and separate precision helper in
A1/B1/B2/A2 order. All eight official cells and all eight precision processes
passed; the helper never replaced an official result.

| Host / job | Official slice MB/s A1 / B1 / B2 / A2 | Ungated 4 MiB census MB/s A1 / B1 / B2 / A2 | Helper BestPass1 ns A1 / B1 / B2 / A2 | Paired helper mean |
| --- | --- | --- | --- | --- |
| Ubuntu 24.04, job `135035055136`, image `20261111.370.1`, Intel Xeon Platinum 8573C | 18.4 / 19.3 / 19.5 / 18.6 (all pass) | 17.0 / 17.7 / 17.6 / 17.1 | 3,871,256 / 3,484,245 / 3,569,618 / 3,627,890 | A 3,749,573 ns; B 3,526,931.5 ns; B elapsed 5.94% lower |
| Ubuntu 26.04, job `135035055638`, image `20261116.202.1`, AMD EPYC 9V45 | 22.1 / 22.9 / 22.7 / 22.2 (all pass) | 20.3 / 21.0 / 20.9 / 20.0 | 3,043,198 / 3,163,751 / 2,998,331 / 3,142,800 | A 3,092,999 ns; B 3,081,041 ns; B elapsed 0.39% lower |

On U24, both official B rates exceed both A rates and both B helper times are
below both A times. On U26, official B rates exceed both A rates, but helper
B1 is slower than either A time and B2 is faster than either; the 0.39% paired
mean difference is within the observed spread. Report the hosts separately:
these data do not support a pooled or general 6% gain claim. Both A hosts
passed the floor, so this comparison does not demonstrate rescue of the
historical slow host. The prior Ubuntu 26.04 / EPYC 7763 baseline miss at
13.2 MB/s (job `135005747661`) has no paired candidate measurement and remains
an open G1 robustness question.

### Artifact and binary identity

The retained run artifacts are GitHub artifact `14458713072`
(`kiln-g1-abba-candidate-ubuntu-24.04`) and `14457444089`
(`kiln-g1-abba-candidate-ubuntu-26.04`). They contain all cell logs, resource
snapshots, identities, and frozen binaries. Admission-binary SHA-256 values:

| Host | A admission | B admission | A precision | B precision |
| --- | --- | --- | --- | --- |
| Ubuntu 24.04 | `f061d5a5778b63c4842578063915e29720d3a8fcf8e9fdc5c75a341800d7e492` | `75356236db6f3f587b4fc2fce6cbc7807e41c99e8ccf6ef40708da00cfa75d46` | `69cc999c35fb52a915f28169b155ba7ab6931fa15e4d5456091eb5af5cf90841` | `cd625114039163ec660db94ddd2329f6f83c3df51f3bd12325c55058b13c25bf` |
| Ubuntu 26.04 | `b9046cb667715e05efc61df6ae82b54ce9c20033b7ed0c56609ed491302f7104` | `bdb76ce791cd96bcf560fd861b8911064175d4a2adff31f918faa8dcf0833e8d` | `a6ea1a494d92c07a688dd86cbf15545a8b8cdeeba65c9993a08bc1f2cf5ed84e` | `5523981d8ecb2d0b568dc2eae82d8ae7cf12e42ab2c5abc1c68d52ad3f11b243` |

The unchanged 14 MB/s gate, slice, refusal behavior, census, and default CI
selection remain intact. This narrow experiment supports evaluating the
lexer fast path; ordinary protected CI and merge are still required. G1
robustness remains open until a predeclared matched-host cohort, independent
of candidate outcomes, records every run and demonstrates any requested
slow-host rescue without retries selected on results.

These results do not close the Go lexer VER/Warrant source-binding obligations
or establish SAT/source equivalence. The local frozen-oracle, deterministic,
fuzz, and negative-mutation checks are bounded behavioral evidence only.
