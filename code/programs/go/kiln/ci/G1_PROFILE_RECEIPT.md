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

## Matched B/C follow-up hypothesis (candidate C under evaluation)

After the protected PR run on candidate B failed the U26 G1 threshold at
13.7 MB/s / 4.848802 ms, a fixed four-runner cohort compared baseline A
`efe517a24fef9ddad591e468f72f8e1251e177ac` with frozen candidate B
`1a3f378412a5eeb899bcc8e10c8c0c3b9a066bb1`. Diagnostic workflow commit
`3d904dab76ae96bcd67f159d8ad49cac4048727b` produced run
[`44377433767`](https://github.com/adhithyan15/coding-adventures/actions/runs/44377433767).
The unmodified gate blob remained `f834ff07849b8c1dd8b412d99e4e95e119176614`,
BestPass1 source blob `c04b3a093c81262201ecb2f43d48f9a162312eff`, fixture SHA-256
`dc8acd0de1d9b235c3924d0d71e2fcd565274e09ded9975570f429b548525aae`, and
66,600-byte / 936-line slice SHA-256
`6cd61f63fdec55ce37294ad061705394b1babda80e5368b17ecb5bd38e2207af`. Each
runner used Go 1.26.1, froze clean official A/B binaries, ran A1/B1/B2/A2,
collected separate precision-helper cells, then captured three independent
20-call CPU profiles for each arm. All four jobs and profile receipts completed
with integrity status zero. The uploaded artifacts retain every cell, source
tree, host identity, binary hash, profile, and resource snapshot:

| Job / artifact | Host | Official slice MB/s A1 / B1 / B2 / A2 | Census MB/s A1 / B1 / B2 / A2 | Helper ns A1 / B1 / B2 / A2 | Paired helper mean |
| --- | --- | --- | --- | --- | --- |
| `135060225752` / `14461923700` | Ubuntu 24.04, AMD EPYC 9V45 | 22.7 / 24.0 / 23.8 / 23.5 (all pass) | 20.6 / 21.6 / 22.5 / 21.8 | 3,035,916 / 2,754,380 / 2,808,372 / 3,047,123 | A 3,041,519.5 ns; B 2,781,376 ns; B elapsed 8.55% lower |
| `135060225631` / `14462082375` | Ubuntu 24.04, Intel Xeon 6975P-C | 20.4 / 21.4 / 21.3 / 20.4 (all pass) | 19.0 / 19.7 / 19.6 / 19.0 | 3,346,100 / 3,155,642 / 3,271,189 / 3,305,231 | A 3,325,665.5 ns; B 3,213,415.5 ns; B elapsed 3.38% lower |
| `135060225584` / `14463880902` | Ubuntu 26.04, Intel Xeon 6975P-C | 20.5 / 21.5 / 21.4 / 20.5 (all pass) | 19.2 / 20.0 / 19.7 / 18.8 | 3,292,244 / 3,163,702 / 3,160,256 / 3,299,036 | A 3,295,640 ns; B 3,161,979 ns; B elapsed 4.06% lower |
| `135060225029` / `14464105422` | Ubuntu 26.04, AMD EPYC 9V74 | 13.6 / 14.1 / 14.3 / 13.7 (A1/A2 fail; B1/B2 pass) | 12.6 / 13.0 / 13.0 / 12.6 | 4,912,518 / 4,833,429 / 4,807,045 / 4,933,802 | A 4,923,160 ns; B 4,820,237 ns; B elapsed 2.09% lower |

The EPYC 9V74 is the sole within-host A-miss/B-pass recovery in this cohort;
the other three slots passed on both revisions. The separate helper timing is
not an admission result. The diagnostic profile shares on that 9V74 show
`GrammarLexer.applyTransitions` at 13.79–15.17% flat CPU in A and 1.47–1.84%
in B across the three captures (about 543–547 samples per capture), matching
the reviewed transition-loop change. The same repeated reduction appears in
the remaining profiles; the artifact files retain the per-capture shares and
sample totals. This is bounded evidence for the current candidate only and
does not settle broader runner robustness or establish a population-wide
margin.

The post-B 9V74 profiles repeatedly identify `matcher.match` as a residual
production hotspot: 11.05%, 7.90%, and 10.87% flat, with 2.70 seconds cumulative
through `rule.matchLen` over the three captures. `pprof -list` attributes
220–250 ms per capture to `unit.at` in the single-class repetition path and
80–140 ms flat to its count/maximum loop. The falsifiable candidate C hypothesis
is to scan the immutable ASCII byte set directly only for a `stepRun` whose
unit has `high == false`, with the upper bound limited by both remaining input
and finite `max`; Unicode/high classes and every other matcher path remain
unchanged. The profiles do not apportion ASCII versus high-rune input and do
not themselves prove this specialization improves official admission. A local
component benchmark and representative strace fixture-pipeline benchmark were
used only to check plausibility before the separate held-out native B/C
experiment recorded below.

Protected CI for PR #19657 remains failed on the separate U26 synthetic merge
at 13.7 MB/s. Auto-merge is disabled. Keep the 14 MB/s floor and gate unchanged.
The later fixed B/C cohort is bounded validation on its four selected host
slots; it does not replace that protected failure or establish population-wide
G1 robustness. The Go lexer VER/Warrant/native-source and SAT/source-equivalence
obligations also remain open.

## Candidate C local semantic and plausibility receipt

Candidate C is the local, review-approved checkpoint
`29e15236f358f22f66fab507a38a652afd786ea3`, based on B
`1a3f378412a5eeb899bcc8e10c8c0c3b9a066bb1`. The production matcher blob is
`72ca3a12145dcc74b84ccb3baf757bd6d32c4bcb` (B blob
`e2ecb4d52a32122f2056a632f7a4e645860f9667`). Its only production Go source
change in the compiled-input roots is `code/packages/go/lexer/matcher.go`;
the added test file is test-only. The transition source is unchanged at
`3fec22ab311048fc219823d714ce2fb0deefb8ec` in both versions. The strace grammar
tree, G1 gate blob `f834ff07849b8c1dd8b412d99e4e95e119176614`, measurement blob
`c04b3a093c81262201ecb2f43d48f9a162312eff`, fixture blob
`698d6f039119ba6a572b31e9d3d3ec9175819a31`, and fixture/slice SHA-256 values
above are identical. These identities were also checked by the held-out
workflow's compiled-production-delta preflight.

The matcher specialization applies only to a repeated class with `high ==
false`. It bounds scanning by remaining bytes and finite `max` before adding to
the current position, then checks `min`; all Unicode/high classes keep the
existing `unit.at` loop. The source/test checkpoint passed the following
bounded controls:

- Actual grammar dispatch: the compiled strace `SYSCALL`, `FUNC`, `PID`,
  `PID_TAG`, `WORD`, and `SPACES` rules each contain an ASCII-only `stepRun`.
- Differential tests compare the optimized matcher with both a frozen copy of
  the pre-change matcher loop and anchored Go regexp. The exhaustive test covers
  30,941 byte strings over a 13-byte alphabet (lengths 0 through 4) for each
  of ten supported patterns: 309,410 pattern/input pairs. It includes zero,
  minimum, maximum, unbounded, delimiter-following, valid/invalid UTF-8, high
  bytes, and lengths around 64. A deterministic property test covers 2,048
  inputs from seed `0x475231`.
- Targeted native Go fuzzing was run with
  `go test -run '^$' -fuzz '^FuzzAsciiRunMatchesLegacyAndRegexp$' -fuzztime=3s -parallel=4`:
  exit 0, 74,847 executions, 51 new interesting inputs; the fuzzer reported
  15 baseline seed cases. This is bounded fuzz evidence, not exhaustive input
  coverage.
- The minimum-count mutation (`<` to `<=`) was caught by the exhaustive test
  on `[a-z]*!` with input `!` (mutant exit 1; got -1, expected 1). The
  add-before-clamp overflow mutant was caught on prefixed inputs `Qa!` and
  `Qaaaa!` (mutant exit 1; got -1, expected lengths 3 and 6). Both mutations
  were reverted; the matcher source SHA-256 before and after restoration was
  `7b045a1ea4cd7d0af88f39377a5968ce86deeff4b3d671158390569a2e1c22b2`.
- Full `go test ./...` and `go vet ./...` passed for Go lexer, parser,
  strace-lexer, and strace-parser. Full Kiln `go test ./...` also passed
  (including `internal/record`, `internal/sandbox`, trace, and traced packages).
  Kiln verification used macOS and does not replace Linux Sandbox CI.

Same-machine component and fixture-pipeline benchmarks were run on macOS arm64,
Apple M4 Pro, using five samples per benchmark. For the direct matcher
microbenchmark, B/C ns per match were: syscall 12.57–12.77 / 4.958–5.092;
word 13.22–13.56 / 4.226–4.289; PID 18.12–18.42 / 7.360–7.639; spaces
16.87–17.27 / 4.813–5.263. Allocations were zero in every sample. The actual
four-fixture strace pipeline measured 26.54–26.72 MB/s for B and 29.04–29.58
MB/s for C (lex plus parse); lexer-only measured 65.85–66.92 MB/s for B and
76.51–80.42 MB/s for C. These prototype timings justify a native experiment
only; they do not establish Linux throughput, a 14 MB/s gate pass, a general
gain, or repair of the protected CI failure.

Required capabilities for `go/lexer` currently declare no external capability
requirements. No new VER/Warrant source binding, SAT/source equivalence, or
native source proof was added; those obligations remain open. The reviewed
held-out workflow is commit `362e7a3d371dc68efc9fc4b29fee94a27a4784cc` on the
dedicated diagnostic branch. It pins baseline B
`1a3f378412a5eeb899bcc8e10c8c0c3b9a066bb1` and candidate C
`29e15236f358f22f66fab507a38a652afd786ea3`, with matcher blobs
`e2ecb4d52a32122f2056a632f7a4e645860f9667` and
`72ca3a12145dcc74b84ccb3baf757bd6d32c4bcb`, respectively. The fixed four-job
run `44385830717` completed successfully at that workflow SHA. Complete
artifacts are retained under
`/tmp/kiln-g1-heldout-bc-44385830717/all-artifacts/`; the per-cell machine-
readable receipt is `/tmp/kiln-g1-heldout-bc-44385830717/receipt-cells.csv`.
All sixteen official B/C gate cells passed the unchanged 14 MB/s floor. Each
job also completed separate precision-helper cells with exit zero and no
refusal; helper timings are independent observations, not admissions. Official
rates below are slice/census MB/s in B1/C1/C2/B2 order; helper times are ns in
the same order:

| Job / artifact | Host and image | Official slice rates | Official census rates | Helper ns | Paired helper mean |
| --- | --- | --- | --- | --- | --- |
| `135090006241` / `14466351863` | Ubuntu 24.04 image `20261111.127.1`, Intel Xeon 6975P-C | 20.8 / 23.8 / 23.7 / 21.0 (all pass) | 19.2 / 21.4 / 21.6 / 19.4 | 3,198,142 / 2,917,635 / 2,825,001 / 3,218,273 | B 3,208,207.5 ns; C 2,871,318 ns; C elapsed 10.5009% lower |
| `135090006387` / `14466432193` | Ubuntu 24.04 image `20261116.133.1`, Intel Xeon 6975P-C | 21.4 / 24.0 / 24.0 / 21.5 (all pass) | 20.0 / 22.2 / 22.0 / 19.7 | 3,188,259 / 2,845,422 / 2,810,032 / 3,142,672 | B 3,165,465.5 ns; C 2,827,727 ns; C elapsed 10.6695% lower |
| `135090005595` / `14467365297` | Ubuntu 26.04 image `20261111.196.1`, AMD EPYC 9V74 | 14.2 / 15.7 / 15.8 / 14.2 (all pass) | 13.2 / 14.4 / 14.6 / 13.1 | 4,725,942 / 4,394,768 / 4,272,455 / 4,710,052 | B 4,717,997 ns; C 4,333,611.5 ns; C elapsed 8.1472% lower |
| `135090005797` / `14466875533` | Ubuntu 26.04 image `20261111.196.1`, AMD EPYC 9V74 | 18.3 / 20.0 / 20.0 / 18.2 (all pass) | 16.8 / 18.5 / 18.5 / 16.6 | 3,817,402 / 3,438,723 / 3,969,005 / 3,688,000 | B 3,752,701 ns; C 3,703,864 ns; C elapsed 1.3014% lower |

The clean frozen binary identities are SHA-256 values from each artifact's
`identity.txt` (U24 slots shared compiled binaries; U26 slots shared compiled
binaries):

| Slots | B admission | C admission | B precision helper | C precision helper |
| --- | --- | --- | --- | --- |
| U24 slots 1 and 2 | `60a745ad10a7978f5388e14dfda7d4d31308dadf4b8d2e830d4531b5889ea42a` | `fdb76684a0cd79f7582d81789dd3f045088cccd4b4343198330a67cfb6c00b28` | `408e2729309410e259f03d1ba736fdb6926e33e01992dce47a6a7b5d57f354f4` | `8b3ef6c355d7c446e82617fd8d1069eb03f8c2b69acd9e0ab7181c9da88f4f64` |
| U26 slots 1 and 2 | `7d629fe96e7d2f6a1e665c14bffd7186116d61237f7397cb9969230b1157d3ae` | `2235e84de652719d1744a533f43a6370ce01b903ea34c614d4872639875e1cc0` | `8c6d9bd988374e5a9e5540580aa01517788654862d92386c5406fc6eb41d281c` | `01f8b572b3ee4de86e4d40c8ee46916829706ab49821da8e62a21321877629b0` |

Both arms used Go 1.26.1, the same Kiln tree `b325e72dda79d212f5c82eb96f35dd51eac47ec6`, parser tree `edac03b279cbd41a5e8589ff6e693bdb64ddc6f5`, strace grammar tree `ace3bc98711a6c815a17cd37ca900c75737438ec`, gate blob `f834ff07849b8c1dd8b412d99e4e95e119176614`, measurement blob `c04b3a093c81262201ecb2f43d48f9a162312eff`, fixture blob `698d6f039119ba6a572b31e9d3d3ec9175819a31`, fixture SHA-256 `dc8acd0de1d9b235c3924d0d71e2fcd565274e09ded9975570f429b548525aae`, and 66,600-byte / 936-line slice SHA-256 `6cd61f63fdec55ce37294ad061705394b1babda80e5368b17ecb5bd38e2207af`. Runtime identities were recorded separately for every arm/slot; the only compiled production Go delta is the reviewed matcher blob change from `e2ecb4d52a32122f2056a632f7a4e645860f9667` to `72ca3a12145dcc74b84ccb3baf757bd6d32c4bcb`. The transition blob remains `3fec22ab311048fc219823d714ce2fb0deefb8ec` on both.

In each slot C1 and C2's rounded official slice rates exceed both B1 and B2.
All sixteen official classifier outcomes were `PASS` with exit zero, and all
sixteen precision helper runs exited zero. Precision results vary: on U26 slot
2, C2 at 3,969,005 ns is slower than both B values despite the paired C mean
being 1.3014% lower. This cohort contains zero matched B-miss/C-pass rescues;
it supports only the fixed four-slot observation, not population-wide
robustness or a claim that the protected failure is repaired. PR #19657 remains
at remote head `1a3f378412a5eeb899bcc8e10c8c0c3b9a066bb1`; its separate protected
U26 gate failed at 13.7 MB/s on synthetic merge `cc832fafea9c`, and auto-merge
is disabled. Preserve the 14 MB/s floor, this failure, and the open Go lexer
VER/Warrant/native-source and SAT/source-equivalence obligations.
