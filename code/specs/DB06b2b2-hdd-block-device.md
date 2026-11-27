# DB06b2b2 — Adapt the HDD write-back model to logical blocks

Implement the [logical block contract](DB06b2a-block-device-contract.md)
in a Go `hdd-block-device` package. Use the
[ordered flush model](DB06b1-hdd-ordered-flush-barrier.md) for durable
state and the [memory device](DB06b2b1-memory-block-device.md) for
shared conformance histories.

```go
func New() *HDD
func (device *HDD) Geometry() blockdevicecontract.Geometry
func (device *HDD) ReadBlock(block int64) ([]byte, error)
func (device *HDD) WriteBlock(block int64, data []byte) error
func (device *HDD) Flush() error
func (device *HDD) FlushWithCut(cut hddwritebackcache.BarrierCut) (hddwritebackcache.BarrierReceipt, error)
func (device *HDD) Restart() (*HDD, error)
```

The geometry is 32 blocks of eight bytes, directly matching the
admitted cache model. A zero HDD value is a fresh device. A nil
receiver reports zero geometry and the shared invalid-geometry
refusal on each operation. Reads and writes use the shared
address and payload checks in their specified order. Each read
returns a detached block, and each write copies the caller's bytes
into cache state.

`Flush` uses the complete barrier. Its success acknowledges the
durable set. `FlushWithCut` is the explicit simulation operation.
Return the exact barrier receipt and lower errors. Store the returned
cache state for every successful transition, including a valid cut
that loses acknowledgement. A refused cut preserves the live device
and returns a zero receipt. `Restart` returns a separate HDD with the
current durable contents and empty cache. The live input remains
available for the caller's comparison.

Run common block read, write, flush and restart histories against
the HDD and memory devices with the same geometry. Compare full
visible bytes, error identities and results retained after later
operations. For the HDD profile, replay every barrier cut across
distinct dirty blocks, checking the exact persisted prefix against
the lower model. Include nil and zero values, invalid addresses and
payloads, detached input and output bytes, and refused cuts that
leave pending bytes visible. Exercise the public interface,
both build lists, race, vet, guided histories and controlled failures
for flushing only the first dirty sector or retaining volatile
bytes during restart.
