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
83.49% line coverage (1,254/1,502); the completion floor must be rechecked after
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

The direct-I/O path follows the separate card and paper-tape subsystem manuals.
Card starting addresses are restricted to the hardware's 128-word boundaries
below location 2048. `RCD` and `RCB` rotate through their documented four- and
two-area continuous buffers; `RCF` and optional `RCM` read one 80-column card;
all modes write their synchronization word and status bits. `HCR`, the three
punch modes, reader/punch ready branches, not-ready alarms, and automatic
address modification are modeled explicitly. Host card queues and punch output
remain bounded, and failed validation cannot partially consume a card or change
memory.

The N-register device selector makes octal `2500006` mean `TYP`, `RPT`, or
`WPT` according to whether `TON`, `RON`, or `PON` selected the typewriter,
paper-tape reader, or paper-tape punch. Reader and keyboard input advance
through deterministic host events, making N-ready transitions and unread-frame
overrun reproducible without wall-clock sleeps. Paper-tape parity is latched in
the architectural parity indicator and can honor the console's stop-on-parity
setting; `HPT` stops tape or enables optional
keyboard input, and `OFF` disconnects all three devices. Input and output queues
have explicit limits.

The primary reference is General Electric's corrected 1966 printing of the
[GE-225 Programming Reference Manual](https://www.bitsavers.org/www.computer.museum.uq.edu.au/pdf/CPB-252A%20GE-225%20Programming%20Reference%20Manual%201966.pdf),
which resolves the earlier printing's inconsistent SXG opcode as `2506YY3`.
Direct peripheral behavior is checked against General Electric's
[GE-200 Series Punched Card Subsystems Reference Manual](https://ftpmirror.your.org/pub/misc/bitsavers/www.computer.museum.uq.edu.au/pdf/GE-200%20Series%20Punched%20Card%20Subsystems%20Reference%20Manual.pdf)
and [GE-225 Paper Tape Subsystem Reference Manual](https://ftpmirror.your.org/pub/misc/bitsavers/www.computer.museum.uq.edu.au/pdf/GE-225%20Paper%20Tape%20Subsystem.pdf).

Run the package verification from `code/packages/rust`:

```sh
cargo test -p coding-adventures-ge225-simulator
cargo clippy -p coding-adventures-ge225-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding-adventures-ge225-simulator --no-deps
```
