# DB05c2b — Publish flash block reuse after media completion

Connect the [DB05b relocation device](DB05b-ftl-nand-relocation.md),
[DB05c2a page-map operations](DB05c2a-ftl-page-reclamation.md) and
[DB04d block-erase cuts](DB04d-nand-block-erase-cuts.md). A block can
contain stale and already-free slots when the controller starts an
erase. A partial erase quarantines the entire block until a full
media result arrives. The logical map retains every live location.

## Geometry and observations

Extend the existing Go relocation package with a constructor for the
block-enabled profile. It takes the existing logical page count,
physical page count and page size, followed by pages per block and
the completed-erase ceiling per block. Pages per block is a positive
divisor of the physical count. Blocks have equal size and zero-based
indices. The erase ceiling is a nonnegative DB04d integer. Validate
existing geometry and page size followed by block size and ceiling.
Every newly configured block begins with zero completed erases and
a clear incomplete flag.

Keep the existing DB05b construction and public operations compatible.
The new profile exposes a detached observation of block geometry,
completed counts and incomplete flags alongside the established
mapping and media observation. Use the existing DB05b read/write
receipts and signature on both profiles.

## First Go profile

```go
type EraseCut = nandblockerasecuts.Cut
type BlockSnapshot struct {
    CompletedErases uint64
    IncompleteErase bool
}
type ReclamationSnapshot struct {
    PagesPerBlock int64
    MaxCompletedErases uint64
    Blocks []BlockSnapshot
}
type EraseReceipt struct {
    Block int64
    Released bool
    Acknowledged bool
}
func NewWithEraseBlocks(logicalCount, physicalCount int64, pageSize int,
    pagesPerBlock int64, maxCompletedErases uint64) (Device, error)
func (device Device) ReclamationSnapshot() ReclamationSnapshot
func (device Device) EraseBlock(block int64, cut EraseCut) (Device, EraseReceipt, error)
```

Use existing geometry and page-size errors. Add
`ErrInvalidBlockGeometry` for the constructor's divisor check and reuse
DB04d's `ErrInvalidEraseCount` class for its ceiling. The reclamation
snapshot gives zero values on an unconfigured or legacy-profile device.
An erase on a legacy device refuses with `ErrEraseDisabled`. An invalid
index or an ineligible extent uses `ErrInvalidBlockIndex` or
`ErrIneligibleBlock` respectively. The DB04d runtime supplies its
cut and erase-ceiling refusal classes after validating the block.

## Erase progress and reuse

Expose an erase command taking a physical block index and the DB04d
cut. Validate configuration, block profile, block index and eligibility
in that order before the DB04d validations apply. The target extent
contains at least one stale page and every slot is either stale or
free. This eligibility rule also admits retry of a quarantined block
because its map classifications were preserved.

Read the DB04d outcome and copy back exactly the target block's
pages, program markers, completed count and incomplete flag. Return
a receipt with the selected block index, whether the page map
released the extent and whether the erase was acknowledged.
Refused operations return the input device and a zero receipt.
The other blocks and the earlier device value retain their contents.

| Cut | Quarantine | Metadata extent released | Acknowledged |
|---|---|---|---|
| Before accept | Preserve prior flag | No | No |
| After any valid prefix | Set | No | No |
| After media | Clear | Yes | No |
| Complete | Clear | Yes | Yes |

An increase in the DB04d completed count is the explicit media
completion evidence. Call the map's atomic release only with that
evidence. BeforeAccept can return a clear incomplete flag for a
fresh candidate with exactly the same count. An unacknowledged
AfterMedia can return a clear flag with the advanced count.
The receipt preserves the media model's acknowledgement.

A subsequent write supplies an allocation fence for every slot in
every quarantined block. Outside those blocks, destination selection follows the existing DB05b ordering. Compare the raw state
and mapping separately after a partial erase. A stale raw page may
have a clear program marker while its block is incomplete. Existing
live reads remain visible and keep their exact bytes. A full erase
returns stale and previously free slots to the normal allocation pool.
Keep power-loss reconstruction and physical pulse wear for DB05e and
DB05d respectively.

Use the following admitted Go inputs. The selector participates in
the composition test through its public snapshot interface.

| Input | Revision | Runtime source SHA-256 |
|---|---|---|
| DB05b relocation baseline | `4172d858c3ae` | `a2b81c71e605b25099f0ecaf815820a3f385fe3c03e42446b0e515f925dc472a` |
| DB05c2a page map | `dafcd74db3ba` | `517ea6398a4a96eb9af00b82384a5b0c75bbe3bef3d6bb4d02609499caebc1a5` |
| DB04b1 page programs | `11dbaeb25b3` | `6b50bea50d6c28f40452d5316c419f2fa1e2886c15e56780aa49e1f4a8221a4d` |
| DB04d block erases | `652067a85f17` | `3fa29ef4411048cd8a01b4228314aaa542c3306b175c790cb87c049a9629a4ad` |
| DB05c1 erase victim | `0872a35535ea` | `d05fa7151bce7077254007b82ff02a19ab874ed2dfdcdbe4063d4d8bc170f091` |

## Required comparisons

- Use the admitted DB05c1 victim selector on the clean map snapshot
  to choose the first test block, and pass its index to the explicit
  erase. Keep erase eligibility independent of victim policy.
- Model at least two equal blocks and a stale/free target block.
  Compare all erase cuts on each eligible block and every valid
  prefix, and continue with reads and writes after each cut.
  Include a zero-prefix case with identical raw bytes and markers
  to BeforeAccept where the next physical allocation differs.
- Replay a prefix that clears a stale page's marker and leaves a
  programmed stale suffix. Include a second shorter prefix and a
  BeforeAccept retry. Confirm that all quarantined slots remain
  unavailable, newly live pages elsewhere remain readable, and
  the target count is still unchanged.
- Complete the retry with AfterMedia and immediately use the lowest
  newly available slot. Compare a fresh Complete run, a second
  attempt at the erase ceiling, invalid addresses and cuts, zero
  capacity, and earlier-state immutability. Check multiple incomplete
  blocks, including a valid all-fenced capacity refusal.
- Test the unchanged DB05b fixture and all required dependency
  guarantees. Use small exhaustive histories, seeded larger models,
  bounded fixture and history fuzzing, clean-controlled faults for
  a partial release, missing fence and lost-ack release, native tests,
  race, vet and Linux/386 execution.

Use the shared `ftl-block-reclamation.json` vector with schema
`coding-adventures.database-media.ftl-block-reclamation.v1`. Validate
all declarations against the independent model before runtime replay.
Cap the document at 64 KiB and nesting depth 32. Require unique exact
keys, field token kinds and portable exact integers.
