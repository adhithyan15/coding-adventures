# DB04a1 — Go profile for HDD sector write crash cuts

Implement the pure transition in [DB04a](DB04a-hdd-write-crash-cuts.md) in
`code/packages/go/hdd-write-crash-cuts`. Use the Go standard library. A timed
HDD snapshot can seed an experiment at the caller boundary. The transition
itself takes detached bytes and consumes no time.

```go
type CutKind string

const (
    BeforeMediaWrite              CutKind = "before_media_write"
    DuringMediaWrite              CutKind = "during_media_write"
    AfterMediaWriteBeforeResponse CutKind = "after_media_write_before_response"
    Acknowledged                  CutKind = "acknowledged"
)

type Cut struct {
    Kind          CutKind
    WrittenPrefix *int
}

type Receipt struct {
    LBA  int64
    Data [8]byte
}

type Result struct {
    Image                [][]byte
    Receipt              *Receipt
    PersistedPrefixBytes int
}

func Apply(before [][]byte, lba int64, data []byte, cut Cut) (Result, error)
```

Validate 32 rows of eight bytes, signed LBA in `0..31`, eight requested
bytes, cut kind, then prefix presence and range. A during-write cut
requires a prefix in `1..7`. Other cuts require an absent prefix, including
when the supplied prefix value would be zero. Use sentinel errors with
portable identities `invalid_image_shape`, `invalid_lba`,
`invalid_data_length`, `invalid_cut`, and `invalid_prefix`. On refusal the
zero result carries no receipt, and the caller retains its earlier image.

The successful result owns the full next image. Its rows are independent
of caller input, of one another, and of the receipt. The input remains
caller-owned and stable for the duration of a call. The five observable
claims are exact persisted prefix length, target splice, unchanged sibling
bytes, receipt present only for acknowledgement, and preservation of the
caller input. Keep these claims separate from the JSON reader and model.

Replay the [shared vector](fixtures/database-media-v1/hdd-write-crash-cuts.json)
with SHA-256 `2a916447ba7327b171b8d64baf8a690b99607fdf1bae68a1e48e637dcf8d2ea1`.
The test reader enforces the shared schema and its 64 KiB byte limit. It
validates complete cases, duplicate keys, integer token kinds and declared
results against its own oracle before invoking the public transition.
Register this reader independently in the build-tool input registry.

Sweep all 320 LBA and valid-cut combinations, including old/requested
sectors with equal positions and indistinguishable final bytes. Add ordered
refusals, ownership checks with independently mutated values, at least
5,000 seeded transitions, and bounded fixture and history fuzz runs. Use the
portable oracle to defend each of the seven mutation witnesses named in
DB04a. Run the two native build command lists, race and vet. Record actual
execution and any cross-compilation separately.

## Native verification

The suite checked all 320 LBA and valid-cut combinations, the 12
canonical fixture cases and 5,000 seeded transitions. An independent
replay matched 25,968 commands in 976 histories through both the
Python and Go public APIs, including 9,134 typed refusals and 4,039
receipts. The Go client also checked 538,688 independent sector
mutations against retained input and successor values.

The native suite passed 97.6% statement coverage, race detection and
vet. Both command lists ran on Linux. The history and fixture fuzz
campaigns passed 110,000 and 45,000 inputs respectively. All seven
clean-controlled mutations reached their intended assertions.
Linux/386 execution under QEMU and Windows/amd64 cross-compilation
passed. Native Windows execution remains pending.
