# DB04d — Interrupted NAND block erase

Extend the logical media vocabulary in [DB01](DB01-nand-flash.md) and
[DB04b](DB04b-nand-page-program-cuts.md) with cuts in one erase command.
Use a page-prefix teaching model so that media changes, program markers,
the completed erase count, and acknowledgement can be observed separately.

## State and inputs

A block owns between 1 and 256 pages with a common width of 1 through
65,536 bytes. A page with a clear program marker contains only `0xff`
bytes. A set marker admits any content. The block also records a completed
erase count and a boolean that marks an incomplete erase. The count and
configured ceiling are integers from zero through `9007199254740991`.
The count is initially at or below the configured ceiling.

An accepted erase may be interrupted before work begins, after a selected
prefix of whole pages, or after all media work. In this model only a full
media erase increments the completed erase count. A later model can account
for physical pulse wear consumed by interrupted attempts.

| Cut | Pages and markers | Count | Incomplete erase | Acknowledged |
|---|---|---|---|---|
| `before_accept` | Preserve all pages and markers | Preserve | Preserve | No |
| `after_prefix(n)` | Restore pages 0 through n minus 1 to `0xff` with clear markers. Preserve the suffix. | Preserve | Yes | No |
| `after_media` | Restore every page and clear its marker | Increment | No | No |
| `complete` | Restore every page and clear its marker | Increment | No | Yes |

A prefix length is between zero inclusive and the number of pages
exclusive. The zero-length cut preserves all bytes and markers while
recording an incomplete erase. A valid erased block can still consume an
erase cycle. An interrupted block admits a retry under the same ceiling.
A complete retry resets all pages, including the suffix from the previous
cut, and increments the count once.

The simulator outcome exposes media state for the test oracle. A caller
has its own acknowledgement evidence. A controller adapter will define
when to quarantine an incomplete block and which observations it can
use to allow reads, writes, or reuse.

## First Go profile

Use `code/packages/go/nand-block-erase-cuts` and the standard library.
Represent the state with a `Block` holding `Pages []Page`,
`CompletedErases uint64` and `IncompleteErase bool`. A `Page` holds
`Bytes []byte` and `Programmed bool`. A `Cut` has a kind and prefix
length. Require prefix length zero for the other cut kinds.

`Apply(pageSize int, ceiling uint64, block Block, cut Cut) (Outcome, error)`
returns detached pages and markers in `Outcome.Block`. The outcome also
reports `Acknowledged bool`. A refusal returns the zero outcome and
preserves the caller's input state. Validate size, block shape and
invariants, ceiling and count, cut validity, then exhaustion. Use these
typed refusal classes:

| Error | Condition |
|---|---|
| `invalid_page_size` | Width is outside the supported range |
| `invalid_block_state` | Page count, width or erased-marker invariant fails |
| `invalid_erase_count` | Ceiling exceeds the portable bound or count exceeds the ceiling |
| `invalid_cut` | Unknown cut kind or invalid prefix length |
| `erase_limit` | Completed erase count has already reached the ceiling |

The runtime models one block. A later adapter verifies preservation of
other blocks through normal device calls.

## Acceptance evidence

- Enumerate all cuts and every prefix on one through four pages, using
  different page bytes and program markers. Include programmed all-ones
  pages, prefix zero, a complete media erase with absent acknowledgement,
  and incomplete-to-complete retries. Compare every state and outcome with
  a separate declarative page and counter model.
- Exercise width and count extrema, invalid input precedence, the final
  permitted completed erase, detached aliases, and refusal after the
  ceiling. Check short histories across partial repeats and full retries.
- Replay an exact bounded shared fixture
  `fixtures/database-media-v1/nand-block-erase-cuts.json`. Check its
  shape, numeric tokens, unique decoded keys and complete input. Cap the
  test document at 64 KiB and depth 32. Validate all declarations in the
  independent model before runtime replay.
- Run bounded fixture and history fuzzing, race, vet, and runtime
  coverage. Named mutation controls must catch a cleared suffix marker,
  a count increase on a partial cut, and acknowledgement without complete
  media and protocol work. Require passing clean controls.
- Route the exact fixture separately. Audit composition with DB01
  devices before using the block model from the flash translation layer.
