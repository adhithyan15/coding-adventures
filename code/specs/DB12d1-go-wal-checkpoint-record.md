# DB12d1 — Encode a WAL checkpoint generation record

Give a checkpoint a durable generation and a reset phase. A later
reader will use these records to distinguish an open log from a
checkpoint whose old log slots still need clearing.
[Record framing](DB10b-storage-record-frame.md) · [WAL recovery](DB12c2c2-go-wal-recovery.md)

Add the pure Go package `storage-wal-checkpoint-record`.

```go
const FrameBytes = 40

type Phase uint8

const (
    Resetting Phase = 1
    Ready     Phase = 2
)

type Record struct {
    Phase       Phase
    Epoch       uint64
    PreviousLSN uint64
}

func Encode(record Record) ([]byte, error)
func Decode(frame []byte) (Record, error)
```

A record contains one versioned 24-byte payload in the admitted
DBRF version-1 frame. Payload offsets 0 through 3 hold ASCII magic
`DBCP`. Offset 4 holds version 1. Offset 5 stores the phase.
Offsets 6 and 7 are zero. Offsets 8 through 15 store the epoch and
offsets 16 through 23 store the previous log's boundary rank,
both unsigned little endian.

Accept the two defined phases. Accept epochs from 1 through
9,007,199,254,740,991 and previous ranks from zero through the
same ceiling. A zero previous rank represents an empty preceding
log. Apply `ErrPhase`, `ErrEpoch` and `ErrPreviousLSN` in that order
when encoding. Refused encodes return nil. Successful bytes are
independently owned.

Decode first through the existing record frame and preserve its
error. Then require exactly 24 payload bytes, the exact magic,
the supported version and zero reserved bytes. Use
`ErrPayloadSize`, `ErrMagic`, `ErrVersion` and `ErrReserved` in
that order. Validate phase, epoch and previous rank in encoding
order. Refused decodes return the zero record.

The next protocol uses two separately stored checkpoint frames.
After replay succeeds, it publishes a new Resetting epoch before
clearing any old WAL slots. A second durable publication makes
the same epoch Ready after all slots are blank. Restart can finish
an interrupted reset while its frame remains durable.

On the existing 32-sector HDD, three seven-sector WAL slots, one
data sector and two five-sector checkpoint frames fit exactly.
The geometry leaves each checkpoint frame in a separate interval.

Compare independent frame bytes, all fields around their portable
limits, exact length and precedence across mixed faults. Flip each
bit of a small valid framed record and check the typed lower
refusal. Run both build lists, coverage, race, vet, guided inputs
and clean controls for admitting a reserved byte or epoch zero.

## Local verification

The implementation is in `28ba54fe3ad`. An independent bitwise
CRC-32C model matched 405 phase, epoch and prior-rank cases.
It matched 84 admitted frames byte for byte and preserved lower
typed refusals for all 320 single-bit changes to one valid
checkpoint frame. Mixed header faults and payload lengths matched,
and race and vet passed.

Both command lists passed on Linux at 100% statement coverage
with race and vet. Module tidy, 100,000 guided decoder inputs and
both clean-controlled mutation tests passed. Build discovery
added only the checkpoint codec and its two expected edges.
