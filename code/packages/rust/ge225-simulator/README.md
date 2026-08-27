# GE-225 Simulator (Rust)

Behavioral Rust simulator for the **GE-225 instruction repertoire**.

The Rust implementation is the fidelity-first reference for the repository's
GE-225 work. It currently covers the integer, control-flow, shift, console
typewriter, and card-reader core inherited from the original compiler-target
model. Its remaining documented CPU, I/O-controller, and optional Auxiliary
Arithmetic Unit (AAU) families are tracked in
`code/specs/RUST-CPU-SIMULATOR-BACKLOG.md`; the package must not be described as
a complete historical simulator until those items close.

The current memory model follows the machine's architectural rules:

- words are 20-bit patterns and addresses are word addresses;
- installed core is explicitly bounded to the documented 4K through 16K range;
- modification words (X words) are reserved core locations, not detached host
  registers;
- a modified or direct address outside installed memory returns an error rather
  than wrapping around;
- multiword loads, card reads, and block moves validate their whole range before
  mutating state; and
- `set_program_counter` selects a checked program origin, which is useful because
  locations 0 through 3 are the base modification-word group.

The central-processor double-length path also follows the manual's unusual
numeric layout: A supplies one sign and 19 high data bits, Q supplies 19 low
data bits, and Q's sign is duplicated or ignored as specified by each
instruction. `DAD`, `DSU`, `DCB`, `MPY`, `DVD`, and the double-register shifts
therefore operate on one sign plus 38 data bits rather than a host-style 40-bit
integer. The manual's published arithmetic and shift examples are executable
regression vectors.

The modification path uses core-resident X words for both addresses and
instructions. `SXG Y` decodes the corrected `2506YY3` form and selects the
encoded group (00 through 31); it does not take a group number from A. A fixed
or shift instruction carrying an X selector adds the selected X word to its
operand field before execution, with modified shift counts rejected above the
architectural 31-place limit. Overflow from single-length arithmetic and left
shifts remains latched until `BOV` or `BNO` tests it. The current core has
83.44% line coverage (902/1,081); the completion floor must be rechecked after
the remaining optional CPU, controller, and AAU instruction families land.

The optional central-processor arithmetic path models decimal words as the
manual's three BCD digits plus sign and end-of-field flag. In decimal mode,
`ADD`, `SUB`, `DAD`, `DSU`, `ADO`, and `SBO` use ten's-complement signed
fields, preserve a carry or borrow across unflagged lower fields, and turn a
flagged-field carry into latched overflow. Invalid BCD nibbles are rejected
instead of being treated as binary. The optional 19-bit real-time clock is
host-advanced in deterministic sixth-second ticks; `LAC` and `LCA` implement
the documented C-register transfers and 24-hour wrap. Opcode 24 is spelled
`MOV`, matching the corrected manual.

The current card reader is intentionally a deterministic development abstraction,
not a completed GE-225 controller model: callers may queue at most 64 records of
at most 27 words each, and `RCD` transfers the next record after validating the
whole destination range. Exact 27-word card/status rotation, alignment, ready
indicators, and the rest of the controller instruction family remain in the
RCPU-005 I/O slice.

The primary reference is General Electric's corrected 1966 printing of the
[GE-225 Programming Reference Manual](https://www.bitsavers.org/www.computer.museum.uq.edu.au/pdf/CPB-252A%20GE-225%20Programming%20Reference%20Manual%201966.pdf),
which resolves the earlier printing's inconsistent SXG opcode as `2506YY3`.

Run the package verification from `code/packages/rust`:

```sh
cargo test -p coding-adventures-ge225-simulator
cargo clippy -p coding-adventures-ge225-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding-adventures-ge225-simulator --no-deps
```
