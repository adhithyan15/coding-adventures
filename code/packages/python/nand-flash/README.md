# NAND Flash

A first media leaf for the database learning stack. It models logical single-level
cell (SLC) NAND behavior: erased pages read as `FF`, each page accepts one full-page
program until its containing block is erased, and erase restores a whole block.

The package demonstrates why database storage layers need page translation and wear
management. Those policies belong in later lessons: this package does not include an
FTL, ECC, bad-block handling, timing, persistence, or a filesystem. See
[`code/specs/DB01-nand-flash.md`](../../../specs/DB01-nand-flash.md) for the complete
contract and [`code/specs/fixtures/database-media-v1/nand-flash.json`](../../../specs/fixtures/database-media-v1/nand-flash.json)
for portable conformance vectors.

## Example

```python
from nand_flash import NANDFlash, ProgramError

media = NANDFlash(block_count=2, pages_per_block=4, page_size_bytes=16,
                  max_erase_cycles=10)
assert media.read(0, 0) == b"\xff" * 16
media.program(0, 0, b"hello" + b"\xff" * 11)
try:
    media.program(0, 0, b"world" + b"\xff" * 11)
except ProgramError:
    # Copy any live pages elsewhere, then erase the whole block before rewriting.
    media.erase_block(0)

print(media.snapshot())
```

Erase-cycle limits are exact safe integers from 0 through 2^53−1; even when no user ceiling is configured, the portable counter rejects further erases at 2^53−1. Program inputs must be exact built-in `bytes` values. Geometry is bounded to keep accidental allocations small: at most 1,024 blocks,
256 pages per block, 65,536 bytes per page, and 16 MiB total modeled data. Errors
are typed (`GeometryError`, `AddressError`, `PayloadError`, `ProgramError`, and
`EraseLimitError`) and rejected operations leave state unchanged.

## Evidence and coverage map

The test suite's evidence is intentionally bounded and does not claim physical NAND
conformance. `test_shared_vectors_include_result_error_and_state_after_every_step`
pins the language-neutral command/error/state transcript. Boundary tests pin geometry,
addresses, payload validation order, full-page programming, erase limits, and atomic
failure snapshots. The length-three exhaustive trace test checks all 64 command
sequences for a one-page device against an integer-based independent oracle. A seeded
250-command generator (`0xDB01`) checks state after every read/program/erase against an
independent list-of-byte-values model; replaying the seed reproduces the trace. The
seeded malformed-input pass (`0xBADDA7A`) checks common Python type/shape mistakes
and rejected-command atomicity. These small bounded checks are useful when later
languages implement the shared vectors, and can become regression anchors for future
formal state-machine proofs; they do not prove behavior for every geometry or silicon.
