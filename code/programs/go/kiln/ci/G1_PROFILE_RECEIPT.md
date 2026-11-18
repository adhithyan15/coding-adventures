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

## Candidate correctness checks

The candidate's frozen-oracle differential test has a reproducible finite
generator: `math/rand.NewSource(0x4731)`, 2,048 cases, 1–7 ordered rules,
0–3 trigger names per rule, 0–4 actions per rule, token types
`WORD/MARK/OPEN/CLOSE`, active modes empty/default/quote/comment, and values
empty/`x`/quote/`other`. The stack starts at default and has at most three
additional entries; all generated action targets and groups stay in the
bounded `default/quote/comment` set. Each case checks mode stack and skip state
against the frozen `oracleLexer.applyTransitions`.

The native Go fuzz target accepts arbitrary `[]byte`, clamps input to 128
bytes, and derives at most six rules, three token triggers and four actions
per rule, from the same four token types, four modes, four values, and five
actions (`push`, `set_mode`, `pop`, `enable_skip`, `disable_skip`). The stack
has one or two entries. A bounded 10-second run passed 5,998,782 executions
with 33 newly interesting inputs (37 corpus inputs total). This is native Go
coverage-guided fuzzing; it is not a proof of all states. The package's
established `go test ./... -v -cover` check passed at 96.8% statement coverage,
and `go vet ./...` passed. The targeted negative mutation that inverted the
mode guard failed both the named guard test and generated differential test.
The Go package's 96.8% statement-coverage result is specific to this lexer
module; it does not establish Python/TypeScript producer coverage or limits.
The A/B workflow's shared receipt classifier has 11 focused cases covering
valid pass, valid baseline shortfall, valid candidate shortfall, zero/negative
or below-bound durations, missing status/markers, abnormal exits, and terminal
marker/exit mismatches. The recorded baseline U24 pass and U26 shortfall both
classify as expected. No A/B candidate measurement has been collected yet.

The Go lexer manifest declares no external capabilities and describes the
package as pure computation; its generated capability cage remains unchanged.
The shared lexer spec documents behavior, but no Go-lexer-specific VER/Warrant
contract or binding was found in the package or spec. No native-code proof,
source proof, or SAT/SMT target is declared. Those obligations remain open for
this touched package; they are not established implicitly by the filter
package's AF-04b work, and test coverage/fuzzing make no substitute claim.

The diagnostic workflow retained binaries, profile files, per-cell logs, and
resource snapshots as workflow artifacts. They are evidence for this receipt,
not permanent repository blobs.
