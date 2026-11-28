# DB10a — Compute a portable CRC-32C for storage records

A storage checksum lets recovery recognize many changes to a durable
record, including a changed bit after a torn write. Build the checksum
before fixing a record format so the same arithmetic can protect WAL
records, slotted pages and sorted-run blocks. Frame length and trailer
placement will belong to the respective formats.

Add the dependency-free Go package `storage-crc32c` with two pure
operations.

```go
func Checksum(data []byte) uint32
func Update(previous uint32, data []byte) uint32
```

Use the reflected Castagnoli polynomial `0x82f63b78` over unsigned
32-bit state. Start the register at the bitwise complement of
`previous`. Process bytes in order, least significant bit first
within each byte. For one bit, apply the reflected polynomial when
the low register bit differs from the incoming bit. Complement the
register at return. `Checksum(data)` is `Update(0, data)`.
`Checksum(nil)` and `Checksum([]byte{})` are zero.
`Update(previous, nil)` returns `previous`.

Use the register recurrence directly for the reference implementation.
It has fixed scratch space, allocates zero bytes and reads the input
without taking ownership. The caller keeps bytes stable during the
call. Support all `uint32` previous values. The result after processing
any chunk boundary equals the result for the concatenated bytes.
The caller chooses how to serialize the scalar result. Collision
behavior follows CRC-32C. Frame checks use it alongside structural
and length validation.

Retain portable byte and checksum vectors, including the standard
`123456789` check value `0xe3069283`. Compare against Go's independent
`hash/crc32` Castagnoli implementation in tests. Check all one-byte
and two-byte inputs, every split of short known vectors, mixed seeds,
empty chunks and fixed generated buffers. Compare one-bit changes
in each of the short vectors and check zero allocations on both
entry points. Run both command lists, race, vet, a 100,000-input
guided comparison and clean controls for changing the polynomial
or resetting the register between chunks.

## Local verification

The implementation is in `73c06e3ec8c`. An independent normal-order
polynomial calculation and the Go standard library matched 131,328
finite byte and seed choices and 8,192 generated buffers with chunk
splits. Known vectors, changed bits and zero allocations also passed.
The independent caller passed race and vet.

Both command lists passed at 100% statement coverage with race and
vet. Module tidy, 100,000 guided comparisons and both mutation
controls passed. The exported graph and recipe added only this
dependency-free package.
The Linux/386 test suite passed under QEMU. The Windows/amd64 test
binary compiled.
