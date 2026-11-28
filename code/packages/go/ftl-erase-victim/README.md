# ftl-erase-victim (Go)

Selects whole physical blocks for erase from a detached DB05a status vector.
`Select` keeps the stale-only policy: it chooses the candidate with most stale
pages and lowest block index on ties. `SelectCopyAware` also accepts an
exclusion mask and ranks reclaimable blocks by `stale / (live + 1)`, with ties
resolved by larger stale count and then lower block index. It requires enough
free destination pages outside the candidate and excluded blocks for all live
pages that must be copied. Both selectors are read-only and the input slices
must remain stable for the call.

```go
status := []ftlpagemap.PhysicalStatus{
    ftlpagemap.Stale, ftlpagemap.Free,
    ftlpagemap.Free, ftlpagemap.Stale,
}
victim, found, err := ftlerasevictim.Select(status, 2)
copyAware, copyFound, copyErr := ftlerasevictim.SelectCopyAware(status, 2, nil)
```

Runtime imports only the standard library and the exact local `ftl-page-map`
module. The fixture adapter and model are test-only. They reject duplicate JSON
keys, unknown fields, invalid UTF-8, trailing values, depth above 32 and files
above 64 KiB. The independent model validates every declared selection before
runtime replay.

Run from this module directory with Go 1.26.1 and `GOTOOLCHAIN=local`:

```sh
go test -count=1 ./... -cover
go test -race -count=1 ./...
go vet ./...
go test -run=^$ -fuzz=FuzzParseVictimFixture -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzVictimStatusVectors -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzParseCopyAwareFixture -fuzztime=45000x -parallel=2
go test -run=^$ -fuzz=FuzzCopyAwareStatusVectors -fuzztime=110000x -parallel=2
go test -run=^$ -fuzz=FuzzCopyAwareFenceStatusVectors -fuzztime=110000x -parallel=2
GO=/path/to/go python3 tools/check_mutations.py
GO=/path/to/go python3 tools/check_copy_aware_mutations.py
```

The stale-only independent model exhausts all 19,360 status-vector/block-width
pairs for 2–6 pages. The copy-aware enumerator checks every status vector and shared block mask
for 2–6 pages, including pending reservations, capacity refusals, ratio
ordering, both tie rules, and input preservation. The separate-fence
enumerator checks all 1,128,000 status and two-mask combinations through five
pages, with per-candidate external-capacity calculation and retained inputs. The canonical
copy-aware vector is `code/specs/fixtures/database-media-v1/ftl-copy-aware-victim.json`;
the adapter validates exact keys and token kinds and checks every expected
result against an independent model before replay.

Go 1.26.1 on Linux/amd64 passed native tests, race and vet. The new selector
uses a linear scan over physical pages and compares bounded score cross
products without rounding. An earlier stale-only review exercised the
package under QEMU and cross-compiled a Windows/amd64 test binary. The new
copy-aware policy’s current cross-target receipts are recorded in
[`tests/copy-aware-evidence.md`](tests/copy-aware-evidence.md).

The external sorted-candidate model matched all 19,360 legacy layouts, including
7,319 selections and 2,597 maximum-score ties. The new rational model also
checks separate victim and destination fences independently. It checked input
preservation, maximum geometry and refusal precedence, including an
unknown status in an already ineligible block. A separate model matched
all five fixture decisions.

The stale-only fixture reader checks the exact decoded lowercase key sets for
the document, cases, and expected results before decoding to typed structs.
It rejects the reproduced `SCHEMA`, `PAGES_PER_BLOCK`, and `STALE_PAGES`
aliases, including an extra case-variant result key. One escaped spelling is
accepted when it decodes to a canonical key that occurs only once. The
unchanged vector still passes model validation and runtime replay. On the
DB-E65 revision, native/race/vet and four stale-only mutation controls passed;
the reader fuzzed 45,000 fixture inputs and 110,000 status-array inputs. The
native suite also passed under Linux/386 via QEMU.

`SelectCopyAwareWithFences` separates the blocks eligible as erase victims from
blocks whose free pages can serve as copy destinations. Each mask may be empty
or match the block count. Candidate-local free pages are excluded from that
candidate's copy capacity, and both masks are validated before status and
pending-reservation checks. `SelectCopyAware` retains its shared-mask behavior.

The separate-fence model exhausts status and both mask roles through five
physical pages, checks the width-two excluded-destination witness and
candidate-local capacity, and compares 12,000 fixed-seed layouts. The
`FuzzCopyAwareFenceStatusVectors` target checks bounded arbitrary status and
mask vectors.
