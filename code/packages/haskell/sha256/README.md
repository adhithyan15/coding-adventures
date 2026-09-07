# sha256

SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch.

The original list-based functions remain available:

```haskell
sha256 :: [Word8] -> [Word8]
sha256Hex :: [Word8] -> String
```

For bounded streaming callers, use the opaque immutable context with strict
`ByteString` chunks:

```haskell
let prefix = sha256Update sha256Init firstChunk
    branch = sha256Copy prefix
    digestBytes = sha256Finalize (sha256Update prefix secondChunk)
    digestHex = sha256FinalizeHex (sha256Update branch alternateChunk)
```

Updates compress complete 64-byte blocks immediately and retain only a copied
remainder shorter than one block. Empty updates do nothing. Finalization is
repeatable and non-destructive, and copied contexts can be extended
independently. The bytes are hashed exactly in update order; the package does
not decode text or add framing. Messages must be shorter than `2^64` bits
(`2^61 - 1` whole bytes maximum); an update beyond that FIPS bound fails
deterministically instead of wrapping the encoded length.

Compression uses a reusable 16-word unboxed rolling schedule for each strict
input region. When an update completes a retained partial block, it copies only
the 64-byte bridge and the final remainder shorter than one block; complete
caller blocks are read directly without retaining their `ByteString` slices.
The optimized allocation gate hashes one million bytes both as one chunk and in
8 KiB chunks, verifies the FIPS digest, and caps each run at 128 MiB allocated.

## Type

library

## Dependencies

- `bytestring` for strict byte chunks and digests
- `array` for the local unboxed compression schedule

No cryptographic framework, process, filesystem, network, environment, or
native-runtime dependency is used.
