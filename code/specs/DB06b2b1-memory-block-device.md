# DB06b2b1 — Build the memory-backed logical block device

Implement the [logical block contract](DB06b2a-block-device-contract.md)
in a pure Go package, `memory-block-device`. This reference profile
models power loss between calls. It gives storage-engine tests a
configurable block width and geometry with a deterministic restart.

```go
func New(geometry blockdevicecontract.Geometry) (*Memory, error)
func (device *Memory) Geometry() blockdevicecontract.Geometry
func (device *Memory) ReadBlock(block int64) ([]byte, error)
func (device *Memory) WriteBlock(block int64, data []byte) error
func (device *Memory) Flush() error
func (device *Memory) Restart() (*Memory, error)
```

Keep a sparse durable image and a sparse pending image. New valid
devices contain zero bytes logically at every block. Geometry alone
allocates no block contents. A read prefers the pending block, then
the durable block, then a fresh zero block. Every successful read
returns owned bytes. A successful write copies the complete caller
payload and replaces the pending version.

Flush makes every pending block durable and clears the pending image.
An empty flush succeeds. Restart creates a new device from the current
durable image with an empty pending image. Restart preserves the live input device. Later writes, flushes or restarts on one device
cannot change the observations of another. The implementation can
share immutable block bytes privately while keeping all maps and
public byte results independently owned.

Use the shared geometry and refusal order for reads and writes.
Construction and restart preserve the shared geometry refusal.
A nil receiver and a zero device value report zero geometry and
give the shared invalid-geometry refusal for operations. Successful
flushes in this profile persist the complete set between successive
calls. The HDD profile will exercise torn writes and incomplete
flushes using the same client operation vocabulary.

Compare histories against an independent byte map. Exercise
pending write visibility, a second write to the
same block, empty flushes, restart before and after flush, retained
prior devices, source and read-buffer mutation, and valid enormous
logical geometries with a small number of populated blocks. Cross
bad addresses and payload sizes for exact refusal order. Run both
command lists, race, vet, bounded guided histories and mutation
controls for persisting before flush and sharing caller buffers.


## Local verification

The implementation is in `77aeeb048fc`. An independent caller matched
84,035 visible-byte and refusal decisions over 16,807 histories
through the public interface. It checked source and output ownership,
the maximum logical geometry, nil and zero values, and durable
images retained across later writes. The caller passed race and vet.

The emitted recipe passed at 95.0% statement coverage with race and
vet. Both command lists ran on Linux and module tidy passed. The
package tested 625 bounded histories and 65,536 seeded commands,
passed a 100,000-input guided campaign and detected both mutation
controls. The graph and Linux recipe inventory gained only this
package, with the Go logical block contract as its sole dependency.
