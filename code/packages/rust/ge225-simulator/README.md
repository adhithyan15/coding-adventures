# GE-225 Simulator (Rust)

Behavioral Rust simulator for the **GE-225 instruction repertoire**.

The Rust implementation is the fidelity-first functional reference for the
repository's GE-225 work. It covers the documented central processor,
direct-I/O, controller selector, Automatic Program Interrupt, and optional
Auxiliary Arithmetic Unit (AAU) instruction families. Cycle-accurate core
timing, the complete historical peripheral catalog, and DTSS remain outside
this CPU simulator's scope; the separate gate-level partner is tracked in
`code/specs/RUST-CPU-SIMULATOR-BACKLOG.md`.

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
shifts remains latched until `BOV` or `BNO` tests it. A shared pre-execution
check validates pair operands, raw and X-word addresses, `MOV` ranges, branch
targets, and exact CPU/controller/AAU decision skips before I, P, hold, or
operand state changes. The
completed core has 88.81% line coverage (2,152/2,423), above the Rust
completion floor after the optional AAU instruction family.

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

The controller selector exposes its eight fixed-priority plugs through a
bounded deterministic adapter. `SEL P,X` delivers the following two words as
an opaque controller command, skips both words, clears the selected
controller's error state, and requires an explicit selector service event
before another selection. A busy or offline selection produces the documented
alert halt. Device controllers publish their own status bits for the
`2514PCC`/`2516PCC` `BCS` families and complete operations through explicit
not-ready-to-ready events.

The optional Automatic Program Interrupt path latches enabled controller, card
reader, and card punch ready transitions even while interrupts are disabled.
At an instruction boundary it saves P at octal 0201, selects special X-group
32, vectors to octal 0204, and disables further interrupts while the priority
routine runs. `SET PST` followed by a modified `BRU` returns through the saved
X word and restores the interrupted X group; an intervening `SET PBK` returns
with interrupts disabled. The target of a `BRU` is never interrupted, matching
the manual's explicitly uninterruptible branch rule.

The optional AAU is modeled as separate 40-bit AX, BX, QX, and IX registers.
Exact general words select fixed-point, normalized floating-point, or
unnormalized floating-point operation; transfer and reset instructions preserve
the unit's distinct transient overflow/underflow indicators and persistent hold
indicators. `FLD`, `FST`, `FAD`, `FSU`, `FMP`, and `FDV` use the manual's paired
memory-word formats, including odd-address behavior and CPU X-word
modification. Floating arithmetic uses integer mantissa/exponent operations,
not host `f64`, so normalization, exponent alerts, minor results, and `NOX` are
deterministic. Plug-7 `BAR` words expose readiness, sign, zero, transient alert,
hold-alert, and combined-error tests with the documented skip rule.

The primary reference is General Electric's corrected 1966 printing of the
[GE-225 Programming Reference Manual](https://www.bitsavers.org/www.computer.museum.uq.edu.au/pdf/CPB-252A%20GE-225%20Programming%20Reference%20Manual%201966.pdf),
which resolves the earlier printing's inconsistent SXG opcode as `2506YY3`.
Direct peripheral behavior is checked against General Electric's
[GE-200 Series Punched Card Subsystems Reference Manual](https://ftpmirror.your.org/pub/misc/bitsavers/www.computer.museum.uq.edu.au/pdf/GE-200%20Series%20Punched%20Card%20Subsystems%20Reference%20Manual.pdf)
and [GE-225 Paper Tape Subsystem Reference Manual](https://ftpmirror.your.org/pub/misc/bitsavers/www.computer.museum.uq.edu.au/pdf/GE-225%20Paper%20Tape%20Subsystem.pdf).
The optional arithmetic-unit contract follows General Electric's
[GE-225 Auxiliary Arithmetic Unit manual](https://www.bitsavers.org/www.computer.museum.uq.edu.au/pdf/CPB-325A%20GE225%20Auxiluary%20Arithmetic%20Unit.pdf).

Run the package verification from `code/packages/rust`:

```sh
cargo test -p coding-adventures-ge225-simulator
cargo clippy -p coding-adventures-ge225-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding-adventures-ge225-simulator --no-deps
```
