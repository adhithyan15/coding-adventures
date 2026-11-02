# Layer 07bd — LatticeMico32 Functional Simulator

**Status:** `RCPU-083` is complete locally from the independently accepted
specification: 34 release integration tests, 712 checked-in evidence rows,
491 generated exact full-state/full-transaction transitions, 56 generated
lifecycle/fault rollback rows, strict checks, and 94.33% measured release line
coverage (882/935) pass. A different reviewer must still accept
the exact clean implementation head before publication. `RCPU-084`, the
dependent gate-level cell, begins only after the functional cell merges.

## 1. Authority and bounded target

This layer defines a checked instruction-level model of one immutable
LatticeMico32 configuration. It is a CPU and explicit transaction oracle, not
a LatticeMico System, board, cache-timing model, Wishbone fabric, FPGA timing
model, or transistor recreation.

The normative architectural source is Lattice Semiconductor,
*LatticeMico32 Processor Reference Manual*, archive release 3.8, posted
7 September 2016:
<https://www.latticesemi.com/-/media/LatticeSemi/Documents/UserManuals/JL/LatticeMico32ProcessorReferenceManual38.ashx?document_id=51739>.
The archive posting is the version pin; the PDF body retains December 2012 and
2014 publication metadata, so 7 September 2016 is not presented as its cover
date.
The later gate cell's immutable implementation source is the vendor-origin
Lattice RTL in the M-Labs fork at commit
[`84b3e3ca0ad9535acaef201c1482342871358b08`](https://github.com/m-labs/lm32/tree/84b3e3ca0ad9535acaef201c1482342871358b08).
That fork describes itself as the original Lattice soft processor plus
bugfixes and portable additions, and explicitly says it is a processor core,
not a complete SoC. The commit's `lm32_cpu.v`, `lm32_decoder.v`,
`lm32_adder.v`, `lm32_addsub.v`, `lm32_logic_op.v`, `lm32_shifter.v`,
`lm32_multiplier.v`, `lm32_mc_arithmetic.v`, `lm32_interrupt.v`, register
file, load/store, and test sources are evidence, not production oracle code.

The immutable profile is `Lm32BaseFunctionalV1`:

| Property | Pinned value |
|---|---|
| Word / address width | 32 bits |
| Instruction width | fixed 32 bits |
| Byte order | big-endian for instruction and data words |
| Register file | 32 writable 32-bit GPRs; r29=ra, r30=ea, r31=ba |
| Owned storage | one 65,536-byte memory at addresses 0x0000_0000..0x0000_FFFF |
| Shifter | pipelined barrel shifter enabled |
| Multiply / divide | pipelined multiply and multi-cycle unsigned divide/modulus enabled |
| Sign extension | `sextb` and `sexth` enabled |
| Interrupts | 32, latched-pending profile (`CFG_LEVEL_SENSITIVE_INTERRUPTS` absent) |
| Bus-error exceptions | enabled |
| Software debug | `break`, `bret`, DEBA and IE.BIE enabled |
| Reset bases | EBA=0 and DEBA=0 |
| Disabled options | caches, inline memories, cycle counter, user opcode, MMU, JTAG UART, hardware break/watchpoints, external break and ROM debug |

The CFG and CFG2 CSRs report exactly those named choices and RTL revision 2:
`CFG=0x0802_010F` and `CFG2=0x0000_0000`. They are derived constants, never
mutable duplicate configuration state.

## 2. Complete authoritative state

`Lm32State` is an owned frozen value containing exactly:

- `gpr: [u32; 32]`, including mutable r0, ra, ea, and ba;
- aligned `pc: u32`;
- interrupt-enable bits `ie`, `eie`, and `bie`, plus debug exception-remap bit
  `dc_re`;
- 32-bit interrupt mask `im`, latched pending bits `ip`, and a 32-bit
  logical asserted-line sample `interrupt_lines`;
- 256-byte-aligned exception bases `eba` and `deba`;
- all 65,536 memory bytes; and
- installed-image `origin: Option<u32>` and `length: u32`.

The manual says the GPR values are undefined after hardware reset and directs
software to initialize r0. Therefore r0 is not hard-wired by this model.
For reproducibility, `new` and successful `load` choose zero for all GPRs and
resettable state. `reset` zeros GPRs, PC, IE/IM/IP, DC.RE and sampled interrupt
lines, restores EBA/DEBA to zero, and preserves memory plus installed-image
metadata. There is no invented halt latch: LM32 has no architectural halt
instruction.

`restore` validates the complete candidate before commit. PC and both base
registers must be aligned as above; `dc_re` is a single Boolean; image metadata
must describe a nonempty, word-aligned, nonwrapping span in owned memory or be
the canonical unloaded pair `(None, 0)`. No state field is reconstructed from
a trace or hidden host mirror.

## 3. Transport, memory, and transaction traces

Programs are raw big-endian words. `load(program, origin)` requires nonempty
input whose length and origin are multiples of four and whose half-open range
fits owned memory without wrapping. It validates first, clears memory and CPU
state, copies the image, records origin/length, and sets PC to origin.

Fetch requires an aligned PC. A valid four-byte request outside owned memory
becomes architectural InstructionBusError; a request inside memory but not
wholly inside the installed image is typed `FetchOutsideProgram`. Data access
may use the complete owned memory, must be naturally aligned to byte,
half-word, or word width, and uses big-endian assembly. The effective address
is `gpr[rX] + sign_extend(imm16)` modulo 2^32. A valid-width data request
outside owned memory becomes architectural DataBusError; an unaligned request
is a typed atomic rejection because the manual declares unaligned behavior
undefined rather than defining an exception result. Self-modifying stores are
visible to later fetches.

Every successful `step` returns one complete `Lm32StepTrace` with:

- PC before and after, plus exact four fetched bytes and raw word when a fetch
  occurred (absent for an accepted boundary interrupt or instruction-bus
  error before a word was returned);
- one stable lowercase architectural mnemonic (pseudo-instructions are never
  separate decoded operations);
- source indices/values, optional destination index/value, and branch/call
  decision and target;
- ordered instruction-read and optional data-read/data-write transactions,
  each with address, width, value, byte-enable mask, and success/bus-error;
- optional interrupt sample and optional exception transition with ID, saved
  PC register, DC.RE value, selected EBA/DEBA base, vector, and IE transition;
  and
- the complete post-state.

Instruction timing, stalls, retries, caches, arbitration, and Wishbone signal
waveforms are excluded. The transaction order is nevertheless normative so a
later SoC scheduler can compose instruction and data movement without hidden
callbacks.

## 4. Exact instruction and decode surface

The decoder classifies all 64 primary opcodes and validates every field that
the sources define as reserved before execution. The pinned decoder selects
`rcsr`, `sextb`, and `sexth` solely by primary opcode: bits 20:16 and 10:0 are
unused for those forms and are accepted/ignored in every combination. They
are not mislabeled as reserved encodings. The decoder implements the four
manual formats—RI, RR, CR, and I—and the following complete profile surface:

- arithmetic: `add`, `addi`, `sub`, `mul`, `muli`, `divu`, and `modu`;
- logic: `and`, `andi`, `andhi`, `or`, `ori`, `orhi`, `xor`, `xori`, `xnor`,
  `xnori`, `nor`, and `nori`;
- comparisons: register and immediate forms of `cmpe`, `cmpne`, `cmpg`,
  `cmpge`, `cmpgu`, and `cmpgeu`;
- shifts/extension: `sl`, `sli`, `sr`, `sri`, `sru`, `srui`, `sextb`, and
  `sexth`, with shift counts masked to five bits;
- memory: `lb`, `lbu`, `lh`, `lhu`, `lw`, `sb`, `sh`, and `sw`;
- control flow: `b`, `bi`, `be`, `bne`, `bg`, `bge`, `bgu`, `bgeu`, `call`,
  and `calli`;
- control/exception: `rcsr`, `wcsr`, `break`, `scall`, `eret`, and `bret`.

Arithmetic wraps modulo 2^32. Signed comparisons and `sr` interpret two's
complement; unsigned comparisons and `sru` do not. Logical immediates are
zero-extended, high forms occupy bits 31:16, and arithmetic/compare/address
immediates are sign-extended exactly as the manual specifies. Loads extend by
their mnemonic; stores use the low byte/half-word/word. Unsigned division or
modulus by zero enters exception ID 5 without writing the destination.

Conditional branch targets are the instruction PC plus
`sign_extend(imm16 << 2)`; `bi` and `calli` use the sign-extended 26-bit word
offset. `call`/`calli` save PC+4 to ra. Register `b`/`call` targets and every
computed target must be word aligned. `eret` is the architectural `b ea`
special case and restores IE from EIE; `bret` is `b ba` and restores IE from
BIE. All source operands are captured before any destination write, including
ra, ea, ba, and same-register overlaps.

The manual opcode table names signed `div` (0x27) and `mod` (0x35) but supplies
no instruction semantics, and the pinned Lattice decoder implements only
`divu`/`modu`. Those two opcodes are `UnsupportedInstruction` in V1. Primary
opcodes 0x2A and 0x33 are reserved; 0x33's configurable user operation is
disabled. Opcode 0x2B accepts only the manual canonical `break` and `scall`
encodings. Noncanonical reserved bits and every other unassigned form return
`UnknownInstruction`, unchanged.

Assembler aliases `ret`, `mv`, `mvhi`, `not`, `mvi`, and `nop` are tested as
the documented encodings but traces show the underlying real instruction.
Their correctness deliberately depends on software keeping mutable r0 zero.

## 5. CSRs and fixed configuration

`rcsr`/`wcsr` implement the exact profile-visible indices:

| Index | Name | V1 behavior |
|---:|---|---|
| 0x0 | IE | read/write bits IE, EIE, BIE; all other bits read zero |
| 0x1 | IM | read/write all 32 interrupt masks |
| 0x2 | IP | read pending; writing ones clears corresponding latched bits |
| 0x6 | CFG | read-only derived profile constant |
| 0x7 | EBA | read/write; any write with a nonzero low byte stores zero |
| 0x8 | DC | write-only; every write replaces DC.RE from bit 1; all other bits are ignored in this profile |
| 0x9 | DEBA | read/write with the same 256-byte alignment rule as EBA |
| 0xA | CFG2 | read-only derived profile constant |

`rcsr DC` is unsupported. DC bit 1 is implemented even though ROM debug,
hardware debug, single-step, breakpoints, and watchpoints are disabled: bit 0,
bits 3:2, and every upper bit have no storage or behavior and are ignored on
write. ICC, DCC, and CC are absent because caches and the cycle counter are
disabled.
JTX/JRX, breakpoint/watchpoint, MMU, and all other indices are absent. Reads or
writes to an absent CSR and writes to read-only CFG/CFG2 are typed atomic
`UnsupportedCsr` errors.

The functional state follows the manual's EBA rule: the complete low byte is
zero, and a write with any low-byte bit set stores zero. DEBA uses the same
256-byte table alignment. The pinned RTL instead defines
`LM32_EBA_RNG=31:9`, stores only those bits, and therefore has a 512-byte
physical base granularity. This is a known evidence discrepancy, not an
implicit equivalence. The functional corpus must include EBA/DEBA values with
bit 8 set. Before `RCPU-084` can be accepted, its spec must explicitly choose
and review a manual-visible bit-8 adapter or a revised boundary; it may not
claim unclassified full-state equality to the raw RTL base registers.

The cache-control no-op behavior of a different hardware configuration is not
silently borrowed. Configuration registers are validated in decode and
differential matrices so optional instructions cannot accidentally execute
when their advertised CFG bit is clear.

## 6. Exceptions and interrupts

The model uses the manual exception IDs and priority order: reset 0,
breakpoint 1, instruction bus error 2, watchpoint 3, data bus error 4, divide
by zero 5, interrupt 6, and system call 7. Hardware reset is the explicit
`reset` lifecycle operation, and hardware watchpoints are disabled. The other
six surfaces are modeled.

A non-debug exception atomically writes the faulting instruction PC to ea,
copies IE to EIE, clears IE, and sets PC to
`(DC.RE ? DEBA : EBA) + ID*32`. This remapping applies to instruction/data bus
errors, divide by zero, interrupt, and system call. A `break` always writes the
faulting PC to ba, copies IE to BIE, clears IE, and sets PC to
`DEBA + 1*32`, independent of DC.RE. `eret` and `bret` restore PC and the
corresponding saved enable bit. Exception-vector range/alignment is validated
before commit; a vector that cannot be fetched later is not itself a failed
exception transition.

`set_interrupt_lines(asserted_mask)` is an explicit atomic environment action
whose bits use logical one for asserted, independent of package-pin polarity.
Before each instruction boundary the sampled mask is OR-latched into IP.
If `IE && (IM & IP) != 0`, interrupt ID 6 is taken instead of fetching or
executing an instruction. IP remains pending until software clears it with
`wcsr`; a line that stays asserted re-latches on the next boundary. The trace
records the sampled, newly latched, masked, and accepted masks. There is one
architectural interrupt vector, not a fabricated per-line vector.

Instruction/data bus errors and divide by zero are successful architectural
exception steps. Missing images, fetch outside the installed image,
misalignment, malformed encodings, and unsupported configuration are host
contract errors and do not enter an invented LM32 exception.

## 7. Checked lifecycle and atomicity

The public checked API provides `new`, `reset`, `load`, `snapshot`, `restore`,
validated memory helpers, `set_interrupt_lines`, `step`, `run`, and `execute`.
There is no unchecked state setter or hidden device callback.

`new`/`Default` perform the one-time fixed 64-KiB memory allocation through
Rust's ordinary infallible allocation API and therefore are intentionally
outside the typed `AllocationFailed` contract. Once an instance exists, every
allocation performed by checked snapshot/load/execution/trace/result work is
fallible, preflighted, deterministic under the test seam, and atomic. This is
the precise boundary of the package's checked-allocation claim.

Because LM32 has no halt instruction, `run(max_steps)` stops successfully at
the first exception transition and returns its traces plus typed
`StopReason::Exception(id)`. If no exception occurs before the nonzero budget
is exhausted, it returns `StepLimitExceeded` and restores the complete
call-entry state. `execute(program, origin, max_steps)` is atomic across load
and run. Callers that want a fixed number of ordinary instructions use
repeated `step`, not a fabricated halt encoding.

Complete-state traces are retained up to a documented fixed maximum of 64.
All required trace, mnemonic, effect, and result allocations are preflighted.
Deterministic test-only allocation seams must fail before commit at each
fallible stage and after at least one candidate instruction inside both `run`
and `execute`; both operations restore their exact call-entry states.

Stable typed errors distinguish empty, unaligned, oversized, overflowing, and
out-of-range images; no image; fetch outside image; misaligned fetch/data/
control flow; unknown and unsupported instructions; unsupported CSR;
invalid restored state; trace limit; zero or exhausted step budget; allocation
failure; and a source-preserving execute wrapper. Validation, fetch, decode,
all reads, effects, trace construction, and candidate updates precede commit.
Every error preserves GPRs, PC, DC.RE, CSRs, interrupt inputs/pending state,
memory, and installed-image metadata bit-for-bit.

## 8. Independent decode and differential evidence

Production code must not call, embed, or translate the pinned Verilog or any
other LM32 simulator. The implementation owns a table-driven decoder and a
separate test-only encoding/classification oracle. Completion requires these
reproducible matrices:

1. all 64 primary opcodes, every opcode whose high bit selects immediate vs
   register form, all canonical raise encodings, all reserved-field variants,
   0x27/0x35, 0x2A/0x33, and each optional-feature gate;
2. every implemented mnemonic with r0/ra/ea/ba and source/destination overlap,
   immediate endpoints, wraparound, shift counts 0/1/31/32/63, signed extrema,
   comparison equality boundaries, multiplication low-word overflow, and
   unsigned divide/modulus including zero;
3. byte/half/word big-endian loads and stores at both ends of memory, every
   sign-extension boundary, alignment/range faults, self-modifying code, and
   fetch at image boundaries;
4. every branch predicate true/false, positive/negative maximal offsets,
   aligned/misaligned register targets, call link overlap, PC wrap, and all
   pseudo-instruction equivalences;
5. every CSR read/write/ignored/reserved-bit rule, EBA/DEBA low-byte-zero rule
   including bit 8, CFG/CFG2 exact fields, DC.RE writes, break/scall/eret/bret,
   every modeled exception ID through both EBA and DEBA remap states,
   exception priority, and all IE/EIE/BIE transitions;
6. each of 32 interrupt lines independently and in combinations, masked and
   globally disabled states, clear-versus-reassert behavior, acceptance before
   fetch, exception overlap, and snapshot/restore of input and pending masks;
7. exact full-state and full-transaction-trace comparisons against vectors
   generated independently from the manual, plus a separately reproducible
   RTL evidence corpus from the pinned commit covering every implemented
   family and exception/interrupt surface; and
8. reset/load/restore/step/run/execute and every typed fault, including
   deterministic allocation failure before work and after prior candidate
   steps, with exact rollback equality for DC.RE=false and DC.RE=true states.

The pinned RTL corpus may be generated in developer tooling or tests, but it
must be checked in as bounded vectors and must never become a build-time,
runtime, or network dependency. Differences between manual semantics and the
pinned RTL must be classified explicitly; the manual controls architectural
behavior, while the commit controls only the later educational topology.

The completed evidence path is intentionally asymmetric rather than a renamed
copy of the manual matrix: it re-parses the hash-pinned RTL decoder-wire roster
and validates concrete operand-mux, narrow-store, lane-select, interrupt,
exception, and DC.RE constructs. Those constructs select 183 topology cases;
the separate manual oracle supplies expected architectural results. The 308-row
manual corpus independently owns semantic boundary selection.

## 9. Documentation and completion gate

Implementation is incomplete until all of the following are true:

1. the accepted spec commit precedes every implementation commit;
2. one publishable Rust package owns checked state, decode, execution,
   transaction traces, typed errors, and deterministic allocation seams;
3. the complete matrices in section 8 pass in release mode with stable corpus
   counts and reproduction commands documented in the package README;
4. Rustfmt, strict Clippy (`-D warnings`), rustdoc (`-D warnings`), all workspace
   and affected BUILD checks, lessons validation, and greater than 80% measured
   release line coverage pass;
5. package README, changelog, BUILD integration, workspace membership, and the
   expansion backlog report exact behavior, test counts, vector counts, and
   coverage rather than aspirational claims; and
6. a different reviewer gives an exact-clean-head correctness/security PASS
   before publication.

## 10. Explicit exclusions and dependent work

V1 excludes signed `div`/`mod`, FPU/DSP/vector/custom user operations, MMU and
privilege extensions, caches and inline memories, cycle/pipeline timing,
hardware debug/JTAG, hardware breakpoints/watchpoints, bus retries/stalls,
peripherals, DMA, arbitration, and SoC address decoding. Unsupported encodings
fail explicitly; exclusions are not silently treated as no-ops.

`RCPU-084` may proceed only after this functional oracle merges. It must use
the pinned vendor-origin RTL to define a source-backed educational decoder,
register/CSR storage, arithmetic/logic/shift/multiply/divide, interrupt, and
control topology and must differentially match every V1 state and transaction.
It must not claim the RTL is a transistor netlist. A later LatticeMico SoC cell
may compose Wishbone interconnect, memories, interrupt sources, DMA and
peripherals. Electrical timing and transistor descent wait for the separate
SPICE/device-model wave.
