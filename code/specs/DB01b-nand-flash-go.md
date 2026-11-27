# DB01b — Go Profile for Logical SLC NAND Flash

This document defines the Go application profile of [DB01](DB01-nand-flash.md).
It preserves the language-neutral operation rules and JSON trace in DB01. The
package will be `code/packages/go/nand-flash`, with the import package name
`nandflash` and no third-party dependencies.

## Public API

The package exposes:

```go
type Geometry struct {
    Blocks        int
    PagesPerBlock int
    PageSizeBytes int
}

type Config struct {
    Geometry       Geometry
    MaxEraseCycles *uint64 // nil means no user-specified ceiling
}

type Device struct { /* opaque state */ }

func New(config Config) (*Device, error)
func (d *Device) Geometry() (Geometry, error)
func (d *Device) Read(block, page int) ([]byte, error)
func (d *Device) Program(block, page int, data []byte) error
func (d *Device) EraseBlock(block int) error
func (d *Device) EraseCount(block int) (uint64, error)
func (d *Device) IsProgrammed(block, page int) (bool, error)
func (d *Device) Snapshot() (Snapshot, error)
```

`Geometry()` returns a value copy and does not expose mutable device state. A
zero-value `Device` is invalid; each method on a nil or zero-value device
returns `ErrInvalidArgument`. Callers must keep the value pointed to by
`Config.MaxEraseCycles` stable while `New` reads it. `New` copies its value and
retains no configuration pointer, so a caller may change the original
configuration after construction without changing the device. `Device` must
not be copied after first use.

`Read` returns a new independently owned byte slice on success. `Program`
accepts a caller-owned `[]byte`, validates nilness and exact length before
copying it, and copies the validated bytes into device-owned page storage. The
device retains no caller slices. The caller must keep the source stable for the
duration of the call; concurrent mutation of a slice while it is being read is
a caller data race. A nil slice maps to DB01's `PayloadError`; a non-nil slice
with the wrong length maps to `PayloadError` with the size message. Go's static
type system means there is no separate non-byte payload type at this boundary.

Like the other DB01 profiles, the device models serialized commands. Callers
must serialize operations on the same device and provide stable input during
each call. Separate devices own disjoint state. Concurrent scheduling and
threaded access have separate contracts higher in the database backlog.

## Geometry, arithmetic, and state

The Go profile uses `int` for geometry and zero-based address arguments, and
`uint64` for erase counts. It accepts exactly DB01's bounds: 1..1024 blocks,
1..256 pages per block, 1..65536 bytes per page, at most 16 MiB of page
storage, and an optional erase ceiling in 0..9007199254740991. Geometry values
are range-checked before multiplication. Products are formed only after
checking the bounded page count and use division-before-multiplication against
the storage cap, so integer overflow cannot produce an undersized allocation.
All later page offsets are derived from this validated geometry.

New pages are `0xFF`, all program markers are false, and erase counters are
zero. A page may be programmed once between erases, even if the bytes are all
`0xFF`. A successful erase restores every page in one block, clears each
marker, and increments only that block's count. The count is capped at the
DB01 safe-integer ceiling even when `MaxEraseCycles` is nil. A rejected
operation leaves bytes, markers, counters, and caller-visible results
unchanged.

`Snapshot` returns a detached JSON-compatible value:

```go
type Snapshot struct {
    Pages       [][]string `json:"pages"`
    EraseCounts []uint64   `json:"erase_counts"`
    Programmed  [][]bool   `json:"programmed"`
}
```

Each page string is lowercase hexadecimal with exactly two characters per
byte. The nested slices and strings in the result share no mutable storage with
the device or with another snapshot. Mutating snapshot slices cannot alter
subsequent reads.

## Errors and refusal precedence

The package exposes stable sentinel errors. Callers can compare them with
`errors.Is`; their `Error()` strings are part of the DB01 cross-language
contract:

| Sentinel | Error message |
|---|---|
| `ErrGeometry` | `geometry values are outside supported bounds` |
| `ErrBlock` | `block index out of range` |
| `ErrPage` | `page index out of range` |
| `ErrPayload` | `page data must be bytes` |
| `ErrPayloadSize` | `page data must match page_size_bytes` |
| `ErrProgrammed` | `page has already been programmed since erase` |
| `ErrEraseLimit` | `block erase limit reached` |
| `ErrInvalidArgument` | `invalid API argument` |

The first seven messages exactly match DB01's typed domain errors. The final
sentinel covers nil/zero-value devices and other invalid Go API state; it is
outside the shared media-vector error set. `New` returns `ErrGeometry` for an
invalid dimension, storage product, or erase ceiling.

For device operations, refusal order is stable: reject a nil or zero-value
device first; then validate block, then page where applicable. `Program` then
checks nil payload, exact length, and finally whether the page was already
programmed. `EraseBlock` checks block before the erase budget. Query operations
return values rather than accepting mutable output pointers. No failed call
changes device state.

## Runtime boundary and fixture reader

The runtime uses only Go's standard library for errors, byte copying,
and hexadecimal encoding. Its module declares no third-party dependencies and
does not read files, environment variables, or the network. The explicitly run
test suite reads the canonical fixture
`code/specs/fixtures/database-media-v1/nand-flash.json` using standard-library
JSON support. The fixture adapter is test-only; JSON is not part of the runtime
API or dependency graph.

The adapter bounds input size and nesting before decoding the tracked fixture
into test records. It validates field presence and value kinds, rejects
duplicate object keys, trailing JSON, and unknown operation names, and compares
operation results, error sentinels and messages, returned bytes, and full state
after every step. A successful program or erase has an explicit JSON null
result. The optional erase ceiling also permits null. A bounded
token walk may detect repeated keys before typed decoding; typed decoding uses
`UseNumber` and `DisallowUnknownFields`, followed by exact integer and value
kind checks. Require a concrete value wherever the fixture expects a scalar.
The canonical fixture is separately
validated as part of the shared corpus. It remains the oracle; test code must
not synthesize expected state from the device under test.

## Conformance and validation plan

The package uses a dependency-free Go module, package-local `BUILD` and
`BUILD_windows` fronts, a README, and a changelog. Native tests and repository
validation will cover:

- canonical trace replay, including every detached state snapshot;
- table-driven geometry endpoints, one-past-limit values, invalid dimensions,
  product boundaries, and large `int` inputs without overflowing;
- exact sentinel identity/message, nil and wrong-length payloads, and every
  refusal-precedence collision;
- copied `Config.MaxEraseCycles`, copied program data, independent read data,
  and detached snapshot slices;
- block/page addressing, one-program semantics, all-`0xFF` programs,
  successful erase counters, zero and exhausted erase ceilings, and atomic
  refusal snapshots;
- deterministic property-style operation sequences checked against an
  independent byte-array model;
- bounded exhaustive histories for a one-block, one-page device; and
- standard-library `testing.F` targets for malformed addresses, nil/wrong-size
  payloads, and mixed command streams that assert state invariants after each
  refusal. Fuzz inputs and per-input history lengths remain explicitly capped.

The package targets at least 95% statement coverage and records any unreached
conditions. Validation uses
`go test -count=1 ./...`, `go test -race -count=1 ./...`, `go test -cover ./...`,
and `go vet ./...`, run from the package module directory. Include a concurrent
test over independent devices to check state isolation under the race detector.
Report fuzz campaign budgets alongside the deterministic seeds run by ordinary
`go test`.

## Local release verification

The independent Go runtime model checked 106,036 commands with
424,144 page comparisons. It matched 33,200 admitted commands
and 72,836 typed refusals. Four clean-controlled mutations caught
an omitted marker, absent erase count, aliased read bytes and an
incorrect erased value. The repaired fixture boundary accepts the
exact canonical bytes and refusals listed in DB-E117.
[Exact Go fixture admission](DB-E117-go-nand-fixture-exact-objects.md)
