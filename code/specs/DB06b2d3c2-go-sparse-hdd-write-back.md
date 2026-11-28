# DB06b2d3c2 — Drain a sparse HDD write-back cache through its medium

Make pending block writes visible to readers. Plan the entire
dirty-set barrier, then apply only the selected media writes
and preserve the exact durability receipt.
[Barrier plan](DB06b2d3c1-go-hdd-barrier-plan.md) · [Timed medium](DB06b2d3b-go-sparse-timed-hdd-media.md)

Add the Go package `hdd-sparse-block-device`.

```go
func New(geometry hddgeometry.Geometry) (*Device, error)
func (device *Device) Geometry() blockdevicecontract.Geometry
func (device *Device) ReadBlock(block int64) ([]byte, error)
func (device *Device) WriteBlock(block int64, data []byte) error
func (device *Device) Flush() error
func (device *Device) FlushWithCut(cut hddwritebackcache.BarrierCut) (hddwritebackcache.BarrierReceipt, error)
func (device *Device) Restart() (*Device, error)
```

New admits and owns a timed medium and its fixed logical geometry.
A nil or unconfigured receiver advertises zero geometry and
operations return the block contract's geometry error. Keep pending
complete eight-byte blocks in a sparse overlay. Check the signed
block and exact data width before copying a write. The last
pending write to a block is visible. Return detached bytes from
ReadBlock. A pending read uses the overlay. A cache miss reads the
timed medium and preserves its exact error.

FlushWithCut captures the dirty block IDs and sorts them ascending.
Capture the head and clock through Position. Use the barrier planner
to admit the complete cut and current service sequence before any
write or pending-state change. A preflight refusal returns the
planner's exact error and zero receipt.

Apply each planned step to its pending block and the exact medium
cut. Under the admitted exclusive interval, the medium's geometry
and clock match the planned steps. A successful barrier command
clears the captured volatile overlay. The durable image contains
the exact prefix of accepted sector effects. Return the planner's
exact preceding-sector count, terminal prefix and acknowledgement.
Flush uses BarrierComplete and preserves its error.

Restart returns an independent device built from the timed
medium's restart. Pending data is discarded and the durable
sector image survives. Supply a stable, serialized device
interval and stable write bytes.

Compare the two small HDD profiles with a separate visible and
durable block model. Cross dirty-set order, overwrite, read hit and
miss, all terminal cuts, complete media with lost response,
no-effect refusal at a later service clock, and repeat restart.
Verify actual 512-block page I/O over both cache paths. Run both
lists, race, vet, guided inputs and clean controls for writing
durable bytes before Flush and retaining pending bytes after a cut.

## Local validation

Commit `d945343ead1` matched 19,968 independent physical barrier images
on 32- and 512-sector geometries. The model checked every dirty mask of
eight addresses, each terminal byte prefix, last-write visibility and
detached ownership before the cut, and exact survivors after repeated
restart. A later sector that exceeded the clock ceiling refused the
whole barrier before any media effect. Native tests also exercised
actual 512-sector page I/O and its visible and durable images.

Both build lists reached 93.4% statement coverage on Linux. The remaining
branches forward defensive lower errors excluded by valid fixed geometry
and serialized application of the admitted timing plan. Race, vet,
tidy, Linux/386 and 100,000 guided inputs passed. The clean controls
passed and both mutations failed at their intended assertions. The graph
added seven expected dependency edges while preserving every prior row
and recipe.
