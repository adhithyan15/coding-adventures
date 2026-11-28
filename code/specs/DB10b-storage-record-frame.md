# DB10b — Encode one bounded checksummed record

A record frame gives an opaque payload a durable byte length, format
version and checksum. Later WAL and page packages can write their own
typed payloads inside this framing. The [CRC-32C reference](DB10a-storage-crc32c.md)
supplies the scalar checksum.

Add the Go package `storage-record-frame`.

```go
const MaxPayloadBytes = 65_520

func Encode(payload []byte) ([]byte, error)
func Decode(frame []byte) ([]byte, error)
```

The complete frame fits in 65,536 bytes. Encode the following fields
in order.

| Offset | Width | Field |
|---|---|---|
| 0 | 4 | ASCII bytes `DBRF` |
| 4 | 1 | Version `0x01` |
| 5 | 3 | Reserved zero bytes |
| 8 | 4 | Payload length as an unsigned little-endian integer |
| 12 | Payload length | Payload bytes |
| 12 plus payload length | 4 | Little-endian CRC-32C of the header and payload |

Encode accepts zero through `MaxPayloadBytes` payload bytes. A larger
payload returns `ErrPayloadLength`. The returned frame owns its bytes.

Decode checks these conditions in order.

1. The supplied frame is 16 through 65,536 bytes long.
   Use `ErrFrameSize` for a refusal.
2. The magic, version and reserved bytes match the format.
   Use `ErrMagic`, `ErrVersion` or `ErrReserved` at the first mismatch.
3. The declared payload length fits and equals the supplied length
   minus 16 bytes. Use `ErrPayloadLength` for a refusal.
4. The checksum of the header and payload matches the stored scalar.
   Use `ErrChecksum` for a refusal.

Every refusal returns nil bytes. A successful decode owns its newly
copied payload. An empty payload decodes to nil with a nil error.
Keep the caller's input stable during each operation. Check bounds
before allocating decoded bytes. Stream parsing and crash-tail policy
will be part of the WAL layer.

Compare exact independent encoded vectors for empty, short and maximum
payloads. Test every single-bit change in short frames, each boundary
length and intersecting decoder refusals. Compare generated frames
with an independent encoder and verify detached results, stored CRC
endianness and input preservation. Run both command lists, coverage,
race, vet, 100,000 guided inputs and clean controls for admitting
trailing bytes or omitting the header from checksum coverage.

## Local verification

The implementation is in `f312f5bbceb`. An independent encoder and
decoder matched 49,664 exact one-bit refusals, all short truncations,
8,192 generated frames, maximum width and detached byte ownership.
The independent caller passed race and vet.

Both command lists passed at 100% statement coverage with race and
vet. Module tidy, 100,000 guided inputs and both mutation controls
passed. The exported graph and recipes added only the record frame
and its dependency on the admitted CRC-32C package.
