# G1 synthetic profile receipt

This is diagnostic evidence for the current 66,600-byte / 936-line synthetic
G1 admission slice. It is not the full protected Sandbox suite, a proof of host
contention, or evidence that a candidate meets the 14,000,000 bytes/s floor.

## Baseline identity

- Diagnostic workflow source: `af31a9e268164fc2838c69428c27e33dd4ed26fc`.
- Runtime under test: `adadc8fc0b9783b3a829fbb4309182f558d4561b`.
- Gate source blob: `f834ff07849b8c1dd8b412d99e4e95e119176614`.
- Measure source blob: `c04b3a093c81262201ecb2f43d48f9a162312eff`.
- Fixture: 1,850 bytes, SHA-256 `dc8acd0de1d9b235c3924d0d71e2fcd565274e09ded9975570f429b548525aae`.
- Synthetic slice SHA-256: `6cd61f63fdec55ce37294ad061705394b1babda80e5368b17ecb5bd38e2207af`.
- Go 1.26.1 linux/amd64, default GOMAXPROCS, four vCPUs on both jobs.

The official admission binary was compiled and hashed before adding the
diagnostic test helper. The profiled diagnostic binary is distinct. Both jobs
executed all required named tests and workload markers; profiling continued
after admission, and the U26 failure status was retained.

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

The candidate in the lexer changelog walks the immutable ordered table by
index and checks a nonempty mode guard before scanning token triggers. These
profiles motivate testing that candidate; they do not establish its gain or
prove a microarchitectural cause. No timing result for the candidate is part
of this receipt. A same-host A1/B1/B2/A2 comparison and the unchanged official
admission gate are required before any performance claim or integration.

The diagnostic workflow retained binaries, profile files, per-cell logs, and
resource snapshots as workflow artifacts. They are evidence for this receipt,
not permanent repository blobs.
