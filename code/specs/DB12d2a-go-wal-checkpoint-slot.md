# DB12d2a — Stage a checkpoint slot with a separate publication byte

Give the checkpoint reader an explicit byte that distinguishes a
staged frame from one available to recovery. Use the reserved byte
at absolute frame offset 18. A published frame stores zero there,
as required by the existing checksum and payload contract.
[Checkpoint frame](DB12d1-go-wal-checkpoint-record.md) · [HDD write cuts](DB04a-hdd-write-crash-cuts.md)

Add the pure Go package `storage-wal-checkpoint-slot`.

```go
const GuardOffset = 18
const StagedGuard byte = 0xa5

type State uint8

const (
    Vacant State = iota + 1
    Staged
    Published
)

func Stage(record storagewalcheckpointrecord.Record) ([]byte, error)
func Inspect(slot []byte) (State, storagewalcheckpointrecord.Record, error)
```

Stage constructs the exact final checkpoint frame and preserves any
encoding refusal. Then set its guard byte to `StagedGuard`. Keep
the checksum computed over the final frame, which has a zero guard.
Both the staged and the final frame images are caller-owned.

Inspect requires exactly one 40-byte checkpoint slot with
`ErrSlotWidth` for a shape refusal. The staged guard returns Staged
and the zero record. Any other nonzero guard returns `ErrSlotGuard`.
An all-zero slot returns Vacant and the zero record. With a zero
guard and nonzero contents, decode the published frame and preserve
its error. A decode success returns Published and its exact record.
Every refusal has zero state and record.

The later publisher targets the inactive slot. It first writes only
the staged guard and waits for durability. It then writes the staged
frame with its guard still set and waits for durability again. Once
the full frame is durable, it writes only the guard back to zero.
In the HDD prefix-write model, this final write changes one byte of
a sector whose other bytes already match. Every admitted prefix
either preserves the staged value or exposes the complete frame.

Check byte-for-byte staging, all guard byte values, blank versus
invalid frames, exact lower errors and ownership. Compose an
independent prefix model over the publication order. Verify the
original or new admitted record throughout actual HDD cuts on the
inactive slot. Run both build lists, coverage, race, vet, guided
inputs and clean controls for admitting a staged body or recomputing
its checksum with the staged guard.

## Local verification

The implementation is in `1cc818d56eb`. The independent model matched
288 physical HDD cuts across the first guard write, five-sector body
write and final guard publication. It checked the exact entire disk,
the untouched alternate frame and the selected slot observation.

Both command lists passed with 100% statement coverage, race, vet
and tidy. The package passed 100,000 guided inputs. Clean controls
caught premature admission of a staged frame and a checksum computed
over the staged guard. The build graph gained one package with its
seven expected edges.
