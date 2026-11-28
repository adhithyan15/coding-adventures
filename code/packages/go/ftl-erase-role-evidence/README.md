# ftl-erase-role-evidence (Go)

This small pure qualifier recovers the page roles that existed when an erase
intent was issued. Bytes and program markers alone cannot distinguish which
stale page had a marker cleared by an interrupted erase. A retained intent's
ordered stale mask supplies those roles after its sequence, block and erase
generation are matched to the current physical observation.

The result contains only `ftl-page-map.Free` and `ftl-page-map.Stale`, in page
ordinal order. A set program marker must be supported by the retained stale
mask. Every error returns a nil role slice. A successful result is a detached
slice and the input arrays must remain stable until the call returns.

`ValidateGeometry` exposes the same bounded dimension and portable erase-count
checks for recovery layers that need to admit an entire image before selecting
any individual incomplete block. `Qualify` calls it first, so the geometry
and erase-ceiling refusal order has one implementation.

```go
geometry := ftleraseroleevidence.Geometry{
    BlockCount: 2, PagesPerBlock: 3, MaxCompletedErases: 4,
}
intent := ftleraseroleevidence.RetainedIntent{
    Sequence: 7, Block: 0, CompletedErases: 0,
    Stale: []bool{true, false, false},
}
observed := ftleraseroleevidence.BlockObservation{
    Block: 0, CompletedErases: 0, IncompleteErase: true,
    Programmed: []bool{false, false, false},
}
roles, err := ftleraseroleevidence.Qualify(geometry, 7, intent, observed)
// roles: [Stale, Free, Free]
```

Validation checks bounded geometry and erase-count ceiling first, then the
observed block and incomplete state, remaining erase budget, exact array
widths, selected sequence, block and generation identity, a nonempty stale
mask and marker-to-stale consistency. Recovered roles can then be supplied to
the raw interrupted-media restore path for byte and feasible-prefix checks.

The tests independently enumerate all 1,364 marker/stale combinations across
one to five pages and compare 5,000 fixed-seed evidence tuples against a
separate ordered model. Geometry admission also checks more than 90,000
dimension/ceiling combinations, signed extrema, maximum-product factorizations,
zero budget and the portable ceiling boundary. Tests also recreate two
identical marker observations whose original stale roles differ, verify
maximum supported geometry and detached results, and exercise malformed
precedence. The runtime has no dependencies beyond the local page-map value
definitions.

Run from this module directory with Go 1.26.1 and `GOTOOLCHAIN=local`:

```sh
go test -count=1 ./... -cover
go test -race -count=1 ./...
go vet ./...
go test -run=^$ -fuzz=FuzzQualifyEvidence -fuzztime=45000x -parallel=2
GO=/path/to/go python3 tools/check_mutations.py
```
