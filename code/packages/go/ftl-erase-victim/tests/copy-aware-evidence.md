# DB05c3c verification evidence

The selector’s public result is matched against an independent enumerator
over every physical-page role vector and every exclusion mask for 2–6 pages
and every aligned block width. The bounded test checked 360,672
combinations, including 40,521 selections. The canonical seven-case fixture SHA-256
is `f24badf998303b5a92b6ac05ca8b389c0551942006a9b3022addb1218c0f1c25` and is
fully checked against the model before runtime replay. Adapter regressions
cover duplicate/escaped keys, case-insensitive key aliases, null mask entries,
unknown keys/roles, bad dimensions, fractional and out-of-range numbers,
invalid UTF-8, trailing values, depth, and the 64 KiB input bound.
The deterministic model history test checks 8,000 inputs with seed
`0x434F5059`.

Run native tests, race, vet, both fuzz campaigns and mutation controls from the
module directory with Go 1.26.1:

```sh
GOTOOLCHAIN=local go test -count=1 ./... -cover
GOTOOLCHAIN=local go test -race -count=1 ./...
GOTOOLCHAIN=local go vet ./...
GOTOOLCHAIN=local go test -run=^$ -fuzz=FuzzParseCopyAwareFixture -fuzztime=45000x -parallel=2
GOTOOLCHAIN=local go test -run=^$ -fuzz=FuzzCopyAwareStatusVectors -fuzztime=110000x -parallel=2
GO=/path/to/go python3 tools/check_copy_aware_mutations.py
```

Final native `sh BUILD` passed with Go 1.26.1 on Linux/amd64. The selector
reported 100.0% statement coverage and the relocation package reported
96.0%. Race and vet passed for both. The independent rerun completed 45,026
fixture inputs and 110,000 role arrays. Both mutation runners passed their
clean controls and all eight named fault witnesses. Both packages passed
under Linux/386 with QEMU, and both Windows/amd64 test binaries compiled.
Native Windows execution remains pending.

The independent exact-rational oracle matched 150,552 cases, including
85,074 selections and 65,470 valid layouts with no eligible candidate.
It also reproduced all seven canonical expected results and both policy
outcomes on the same device trace. The stale-only selector body and
relocation runtime match the earlier admitted revisions byte for byte.

The fixture fuzz adapter rejects invalid input classes without invoking the
runtime. Any accepted vector must first agree with the independent model, then
is replayed against `SelectCopyAware`. The role-array target bounds input to
128 statuses and compares every admitted input to a separately represented
reference calculation.

The isolated mutation runner requires the clean native selector control, then
requires each mutant’s named Go test and behavioral failure text: first block
incorrectly excluded, candidate-local free slots counted as external, ratio
order reversed, and equal-score tie order reversed. Controller composition is
tested separately in `ftl-nand-relocation`: the common workload shows stale-
only reclaims one slot while copy-aware copies one live page and reclaims
three, preserving the same logical bytes. A separate interrupted-copy trace
records one spent destination, a refused retry, and successful replanning
around an incomplete block. Runtime dependencies remain the Go standard
library and local `ftl-page-map` module.
