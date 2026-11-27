# DB01 — Logical SLC NAND Flash Media

- **Layer** — Database media, first leaf
- **Package** — `nand-flash` with language-native APIs
- **Fixture** — `fixtures/database-media-v1/nand-flash.json`

## Purpose and boundary

This lesson models the logical behavior needed to understand why a database cannot
update flash in place. It is a deterministic, in-memory model of single-level-cell
(SLC) NAND pages. It is not a silicon timing, voltage, ECC, bad-block, wear-leveling,
FTL, filesystem, or database implementation. It has no persistence across Python
processes. Real devices add spare areas, command protocols, ECC, program/erase
latencies, retention and disturb effects, and many vendor-specific restrictions.

## Geometry and address space

A device has positive integer `block_count`, `pages_per_block`, and
`page_size_bytes`. `max_erase_cycles` is an integer in 0..9007199254740991 or `None` (no user-specified ceiling). Erase counts are exact safe integers capped at 9007199254740991; a further erase is rejected with `EraseLimitError` even when the configured ceiling is `None`.
Geometry values are bounded: 1..1024 blocks, 1..256 pages per block, 1..65536 bytes
per page; the product may not exceed 16 MiB. Booleans are not integers for this
contract. Block and page indices are zero-based and strictly range checked.
Payloads must contain exactly `page_size_bytes` bytes. Python accepts exact
built-in `bytes`. TypeScript accepts plain `Uint8Array` backed by a live,
non-shared `ArrayBuffer`. The source must remain stable for the duration of
the call. Validate the byte count before copying and use the same bytes for
validation and mutation.

Each page is represented as exactly `page_size_bytes` bytes. New and erased pages
contain only `0xFF`. The device exposes read-only geometry properties. Reads return
immutable bytes or an independent copy, according to the language. A state snapshot
returns a detached JSON-compatible mapping with lowercase hex page bytes and
per-block erase counts.

## Operations

* `read(block, page)` returns the complete page bytes without changing any state.
* `program(block, page, data)` writes exactly one complete page. A page can be
  programmed once between erases, regardless of its content. At bit level, only
  1-to-0 transitions are representable (`old & new == new` bytewise); this follows
  from replacing an erased page with arbitrary data. A second program is rejected,
  including a second all-`0xFF` program. The bit-transition rule is stated as a
  physical invariant; under this strict one-full-page-program model the erased page
  is all ones, so any first payload is representable. Rejection never changes bytes,
  program markers, or erase counters.
* `erase_block(block)` atomically restores every page in the block to `0xFF`, clears
  all page-program markers, and increments that block's erase count by one. If the
  configured maximum has already been reached, erase is rejected without mutation.

This is deliberately a strict full-page, one-program model; partial-page programming
and multi-step programming modes vary between NAND devices and are outside DB01.

## Errors

Errors are typed and stable. `GeometryError` covers invalid dimensions/limits;
`AddressError` covers invalid block/page indices; `PayloadError` covers non-exact-bytes or
wrong-size data; `ProgramError` covers a page already programmed; and `EraseLimitError` covers an exhausted per-block erase budget. Error
messages are part of the shared vector contract:

| Condition | Message |
|---|---|
| invalid geometry | `geometry values are outside supported bounds` |
| invalid block | `block index out of range` |
| invalid page | `page index out of range` |
| non-exact-bytes payload | `page data must be bytes` |
| wrong payload size | `page data must match page_size_bytes` |
| second program | `page has already been programmed since erase` |
| exhausted erase budget | `block erase limit reached` |

Validation occurs before mutation. When multiple arguments are invalid, validation
order is geometry at construction. For page commands, validate block then page.
For program, validate payload type and size before checking the program marker.
For erase, validate the block before checking its erase budget.

## Teaching invariants

1. Erased bits are all ones; programming can clear bits but never set them.
2. Erase is block-granular, while read/program are page-granular.
3. A logical rewrite requires copying live pages elsewhere, erasing a whole block,
   then writing the new contents. DB01 does not perform this translation.
4. A rejected command is observationally atomic: snapshots before and after match.
5. Erase counts are per block and count successful block erases only.

## Shared conformance fixture

The JSON fixture records geometry, an operation sequence, expected result or exact
error for each operation, and the complete expected state after each step. Consumers
must compare returned bytes as lowercase hex and compare state exactly. The fixture
is language-neutral so later implementations can share behavior without inheriting
Python internals.
