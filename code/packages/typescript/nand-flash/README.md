# NAND Flash

A logical single-level-cell (SLC) NAND media leaf for the database learning stack.
New pages read as `FF`; a full page can be programmed once between block erases;
erasing restores every page in that block. This makes the physical constraint behind
page translation visible before a later lesson builds a flash translation layer.

It does not simulate silicon timing, voltages, ECC, bad blocks, wear leveling, FTL,
persistence, filesystems, or databases. See the [DB01 contract](../../../specs/DB01-nand-flash.md)
and shared [conformance vectors](../../../specs/fixtures/database-media-v1/nand-flash.json).

```ts
import { NANDFlash, ProgramError } from "./src/index.js";

const media = new NANDFlash({
  blockCount: 2,
  pagesPerBlock: 4,
  pageSizeBytes: 16,
  maxEraseCycles: 10,
});
const firstPage = new Uint8Array(16).fill(0xff);
firstPage.set(new TextEncoder().encode("hello"));
media.program(0, 0, firstPage);
try {
  const replacement = new Uint8Array(16).fill(0xff);
  replacement.set(new TextEncoder().encode("world"));
  media.program(0, 0, replacement);
} catch (error) {
  if (error instanceof ProgramError) media.eraseBlock(0);
}
console.log(media.snapshot());
```

Geometry is bounded to 16 MiB, with at most 1,024 blocks, 256 pages per block, and
65,536 bytes per page. Numeric inputs and erase counts use safe JavaScript integers;
`maxEraseCycles` accepts 0 through `Number.MAX_SAFE_INTEGER`, or `null`/omission for
no user-specified ceiling. Even without a configured ceiling, the portable erase
counter stops at `Number.MAX_SAFE_INTEGER`. Program payloads must be plain
`Uint8Array` instances over a live, non-shared `ArrayBuffer`; input is validated and
copied before mutation, so a concurrent shared-memory source is excluded. Reads and
snapshots are detached copies.

## Evidence and coverage map

The shared fixture test pins the portable command sequence, exact errors, read bytes,
and state after every operation. Unit tests pin geometry, address and payload
validation, error precedence, copy boundaries, erase budgets, and failure atomicity.
The length-three exhaustive test compares all 64 tiny-device traces with a separate
integer oracle. The seeded 250-command test (`0xDB01`) compares state after every
command with a byte-list oracle and preserves the seed for deterministic replay. The
seeded malformed-argument pass (`0xBADDA7A`) probes invalid addresses and data shapes.
Additional boundary tests cover spoofed typed-array prototypes, proxies, detached and
shared views, and caller-defined `length` accessors. This evidence is bounded and
supports future cross-language/formal checks; it does not prove physical silicon
behavior or every possible geometry.
