## [Unreleased]

### Security

- Fixed all 3 critical CodeQL `go/unsafe-quoting` alerts (`#314`–`#316`) in
  test fixtures by marshaling `code/kiln/legacy/host-roots.json` via
  `json.Marshal` with `json.RawMessage` fields in
  `internal/record/prepare_unix_test.go`, `record_step_policy_test.go`, and
  `sandbox_record_probe_linux_amd64_test.go` (sharing `marshalRecStepHostRoots`).
- Fixed all 3 high CodeQL `go/incorrect-integer-conversion` alerts
  (`#256`, `#306`–`#307`) in `internal/trace/line.go`:
  - Added explicit `math.MinInt` / `math.MaxInt` bounds checks in
    `result.pid()` before converting `int64` to `int`.
  - Added an explicit `v > math.MaxInt64` check on `strconv.ParseUint` output
    in `parseResult`, preserving `0xffffffffffffffff` (`math.MaxUint64`) as
    `-1` (still requiring an errno downstream) and rejecting other values
    above `math.MaxInt64` with `reasonResult`.
- Added integer boundary test cases in `internal/trace/lex_test.go`
  (`TestParseResultShapes` and `TestResultPidBounds`).
