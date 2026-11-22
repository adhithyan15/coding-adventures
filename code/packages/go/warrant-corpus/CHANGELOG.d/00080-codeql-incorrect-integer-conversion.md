## [Unreleased]

### Security

- Fixed all 3 high CodeQL `go/incorrect-integer-conversion` alerts (`#311`–`#313`):
  - Added explicit `math.MaxInt` upper-bound checks (both inside `need(...)`
    and as an explicit local `if n < 0 || n > math.MaxInt` guard) before
    converting `int64` to `int` in `count()` (`corpus.go:136`),
    `decodeExample()` (`corpus.go:244`), and `(*compiler).subjectOp()`
    (`ir.go:371`).
- Added boundary/overflow unit tests in `corpus_test.go`
  (`TestCountAndExampleWidthBounds`) and `ir_test.go`
  (`TestIRSubjectOp` overflow cases).
