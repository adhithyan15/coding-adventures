# DB06b2a — Define the sequential logical block contract

Connect the verified media transitions to a common Go interface for
WAL, pages and storage engines. A caller addresses complete logical
blocks and decides when to require a durability barrier.
[Ordered flush model](DB06b1-hdd-ordered-flush-barrier.md)

The existing Go `device-driver-framework.BlockDevice` exposes block
reads, writes and geometry. Its interface has no flush operation or
power-loss contract. The first database block contract carries its own
portable boundary. The device-driver and filesystem packages stay in
the [dependency audit queue](DB00-database-backlog.md#acceptance-and-reuse).

Add a pure Go package named `block-device-contract`.

```go
type Geometry struct {
    BlockCount    int64
    BytesPerBlock int64
}

func (g Geometry) Validate() error
func (g Geometry) CheckBlock(block int64) error
func (g Geometry) CheckWrite(block int64, data []byte) error

type Device interface {
    Geometry() Geometry
    ReadBlock(block int64) ([]byte, error)
    WriteBlock(block int64, data []byte) error
    Flush() error
}
```

Accept block counts from 1 through 9,007,199,254,740,991 and block
widths from 1 through 65,536 bytes. They are logical limits, and a
concrete backend can support a smaller geometry. Enforce geometry,
then (0 <= block < BlockCount), then payload width. The respective
typed refusals are `ErrInvalidGeometry`, `ErrInvalidBlock` and
`ErrInvalidBlockData`. The check performs no allocation or device
access. Evaluate signed values before any conversion or multiplication.

A successful read owns exactly one logical block of detached bytes.
A successful write makes its full block visible to future reads and
keeps input storage caller-owned. The caller keeps the input stable
during the write. Address and shape refusals preserve device state.
A successful flush makes preceding successful writes durable. Later
writes may replace a previously flushed block, including a torn
replacement when the backend permits that outcome. An unsuccessful
flush provides no new durability acknowledgement. A backend's recovery
profile describes the actual possible persisted prefixes.

The first interface describes a sequential caller. Publish exact
timing, concurrency, discard and physical atomicity guarantees as
separate backend capabilities when the corresponding adapters are
ready. In the next slice, run portable visibility, flush and restart
traces against a memory device and the HDD write-back model.

Check the geometry limits, every refusal precedence and exact buffer
sizes with unit tests and a finite independent table. Use generated
signed boundaries and guided input, race, vet and controls for an
upper-bound address error and premature length validation. Record
that both command lists compile and run the public package.
