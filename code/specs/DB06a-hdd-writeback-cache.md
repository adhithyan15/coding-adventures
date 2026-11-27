# DB06a — Model volatile writes and durable sector flushes

Use the Go HDD cut model to show when an accepted write becomes
durable. The write-back cache overlays a fixed durable sector image.
A successful cache write makes bytes visible to a reader. A successful
flush makes those bytes survive power loss.
[HDD sector cuts](DB04a1-hdd-write-crash-cuts-go.md)

Implement the pure Go package `hdd-writeback-cache` with this first
fixed-geometry profile. Reuse the 32 sectors of eight bytes and the
signed logical block addresses from the cut model.

```go
type State struct { /* durable image and pending cache entries */ }

type Snapshot struct {
    Durable [32][8]byte
    Visible [32][8]byte
    Dirty   [32]bool
}

type FlushResult struct {
    Acknowledged         bool
    PersistedPrefixBytes int
}

func (state State) Snapshot() Snapshot
func (state State) ReadSector(lba int64) ([8]byte, error)
func (state State) WriteSector(lba int64, data []byte) (State, error)
func (state State) FlushSector(lba int64, cut hddwritecrashcuts.Cut) (State, FlushResult, error)
func (state State) PowerLoss() State
```

The zero state is a fresh disk with zeroed durable sectors and an empty
cache. Read one pending sector if present, otherwise read its durable
bytes. Write replaces that LBA's pending bytes in a new state. Validate
the LBA before the exact payload length. Use the cut model's address
and length errors. The state copies all accepted input bytes and the
caller keeps the input stable until return. Every snapshot and read
returns bytes independently of the device.

Flush validates the address and then requires a dirty sector. Return
`ErrSectorClean` for a clean sector. Apply the cut model to the current
durable image and the pending bytes for the selected LBA. Preserve
its exact validation errors. Read acknowledgement and persisted length
from its result. On an acknowledged flush, the next state clears the
selected pending entry and preserves other pending entries. Each valid
unacknowledged cut represents power loss during this command. Its
durable result preserves the selected prefix and the next state drops
all pending entries. `PowerLoss` models the same cache loss between
commands and preserves the current durable image. Refusals preserve the
entire supplied state and return a zero read or flush result.

Exercise every supported cut and exact invalid-cut precedence on
asymmetric old and new sector contents. Cross a second dirty sector
and an already durable neighbor. Verify same-sector overwrite, reads
before and after a flush, successive crashes, old-state retention,
detached outputs, signed extremes and a clean-sector refusal. Compare
short histories against a separate array oracle. Run both command
lists, coverage, race, vet, guided bounded histories and clean mutations
for persisting pending data early, keeping unrelated pending data
after a crash and reporting early acknowledgement.

The later block interface defines multi-sector flush ordering,
capabilities, and adapter behavior across this profile and other
media. The fixed geometry keeps the first write/flush/power-loss
traces small enough to enumerate.
