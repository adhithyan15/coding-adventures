# DB04b1 — Go profile for interrupted NAND page programming

Implement the pure page transition in
[DB04b](DB04b-nand-page-program-cuts.md) as `go/nand-page-program-cuts`.
The runtime uses the Go standard library. A later block adapter can replay
these page outcomes through an independently admitted medium.

```go
type PageState struct {
    Bytes      []byte
    Programmed bool
}

type CutKind uint8

const (
    BeforeAccept CutKind = iota
    AfterPrefix
    AfterMedia
    Complete
)

type Cut struct {
    Kind        CutKind
    PrefixBytes int
}

type Outcome struct {
    State        PageState
    Acknowledged bool
}

func Apply(pageSize int, before PageState, payload []byte, cut Cut) (Outcome, error)
```

Validate size `1..=65536`, exact length of `before.Bytes`, and erased bytes
for a clear marker. Then validate exact payload length, cut, and the current
program marker, in that order. `AfterPrefix` uses `0..pageSize-1` as its byte
count. Every other cut kind requires `PrefixBytes == 0`. A refusal returns
the zero `Outcome` with its matching error. Inputs remain caller-owned and
are kept stable for the call. Each successful result has its own byte slice.

| Sentinel | Stable message |
|---|---|
| `ErrInvalidPageSize` | `page size outside supported range` |
| `ErrInvalidPageState` | `invalid NAND page state` |
| `ErrWrongPayloadLength` | `payload length must equal page size` |
| `ErrInvalidCut` | `invalid NAND program cut` |
| `ErrAlreadyProgrammed` | `page has already been programmed since erase` |

## Shared fixture

Read `fixtures/database-media-v1/nand-page-program-cuts.json` with schema
`coding-adventures.database-media.nand-page-program-cuts.v1`. The root has
`schema` and `cases`. The exact case keys are `name`, `page_size`, `before`,
`payload`, `cut`, and one of `result` or `error`. A state object has
`bytes` as lowercase hex and `programmed` as a boolean. A cut object has
`kind` as a raw integer byte and `prefix_bytes` as a signed 32-bit integer.
Represent `page_size` as a signed 32-bit integer as well. The result has
`state` and `acknowledged`. Use the five DB04b error codes as strings.

Bound ingestion to 16 KiB, 32 cases, and depth 16. Check exact object keys,
duplicate members, UTF-8, well-formed Unicode scalar escapes, integer token
kinds, lowercase hex, and complete document consumption. A case name must
contain at least one scalar outside Unicode whitespace. Asymmetric states,
wrong byte lengths, and invalid codes can be structurally valid case inputs.
The independent prefix oracle validates every declared result or refusal
before the first runtime replay.

## Verification

- Check all 2,272 payload/cut combinations for lengths one through four
  with byte alphabet `{0x00, 0xff, 0xa5, 0x5a}`. Verify the next program's
  result after each outcome. Exercise 5,000 seeded bounded commands with
  successful cuts and typed refusals on owned and previously consumed states.
- Test size 65,536 directly, plus invalid signed sizes, inconsistent states,
  wrong lengths, invalid kinds and prefixes, and refusal precedence. Check
  full and zero-prefix outcomes, all-ones retry behavior, and detachment.
- Run bounded coverage-guided fixture and command fuzz targets against the
  oracle. Rebuild disposable variants that program one extra suffix byte,
  clear the marker after a prefix, and acknowledge incomplete media work.
  Each prescribed variant must fail its named test.
- Verify native tests, race, vet, and coverage with Go's standard tools.
  Retain plain package build fronts. Record other architectures separately
  when available.
