# DB01 — NAND flash fundamentals

Parent: DB00; backlog: DB-003/004/008/009. Language-neutral behavioral contract.

## First finite primitive

`program_byte(old, requested) -> byte | InvalidByte | RequiresErase`.

Inputs are exact integers 0 through 255. Dynamic-language bindings reject
booleans, fractions, NaN and coercible strings. Fixed-width bindings enforce
their domain at the type boundary. Wider integer APIs validate range first.

An erased byte is 255. Programming may change one bits to zero but cannot
restore a zero bit to one. For every bit position, if old is zero and requested
is one, return RequiresErase. Otherwise return requested, exactly. Equal values
are accepted by this primitive. It describes a necessary bit-transition rule,
not permission to repeatedly program a physical page.

Exhaustively test all 256 x 256 pairs against an independently expressed per-bit
oracle. Check invalid inputs and fixed error categories. Negative controls must
reject implementations that allow zero-to-one changes, reject every operation,
or return the old value on a valid change. No hidden state, clock or I/O.

## Next raw-device profile

`Geometry(blocks, pages_per_block, bytes_per_page)` is immutable, positive and
bounded before allocation, with checked products and an explicit memory cap.
State includes bytes and programmed-page flags; endurance/fault state is added
only by the later versioned profile. `read_page`, `program_page`, `erase_block`
and `snapshot` are operations on explicit state.

- New/erased pages contain 255 in every byte.
- Program exactly one whole page once between erases, including all-255 payloads.
  Reject a second program even if the byte transition primitive would allow it.
- Erase resets every page and flag of the selected block; neighbors are intact.
- Validate index, payload length, byte values and page state before mutation.
  Every rejection preserves the complete device state.
- Reads/snapshots cannot expose mutable aliases to internal bytes.
- The baseline is serialized and synchronous, with atomic successful operations
  in model memory. It makes no power-loss or thread-safety guarantee. Later
  fault/queue profiles expose partial program/erase explicitly.

Test bounded operation histories for tiny geometries against a simple independent
state model; print depth/alphabet. Fuzz geometry, addresses, data and histories;
check erase locality, program monotonicity, rejection atomicity and alias safety.

## Fidelity and later cells

This is raw NAND behavior, not an FTL, block-rewritable SSD or transistor proof.
DB-008 adds a discrete stored-charge/threshold bridge to the audited transistor
layer. Analog tunneling/retention models and MLC/TLC/QLC distributions require
their own calibrated assumptions. Page order restrictions, partial program
counts, bad blocks, ECC, endurance, disturb, timings and interrupted operations
are explicit later profiles. Never generalize this profile to every NAND part.
