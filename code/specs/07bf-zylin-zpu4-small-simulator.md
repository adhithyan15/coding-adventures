# Layer 07bf — Zylin ZPU4 Small Functional Simulator

**Status:** proposed spec-only `RCPU-087`; duplicate/evidence audit complete;
independent exact-head review is required before implementation. The dependent
gate-level cell is `RCPU-088` / Spec 07bf2 and cannot begin until the
functional oracle merges.

## 1. Discovery, priority, and authority

This layer defines a checked instruction-boundary model of one immutable
configuration of Zylin's 32-bit, zero-operand ZPU stack CPU. It is a CPU plus a
deterministic external-word-bus adapter. It is not a ZPUino, a board, an SoC,
a cycle-accurate VHDL simulator, or a transistor recreation.

The discovery audit was reconciled with `origin/main`
`f6804fc7dc07338d0ee04338e6120a08abb280ec` on 2026-10-26. Repository
packages, specifications, history, local/remote branches, worktrees, and every
open or merged pull request contained no ZPU or Zylin CPU claim. The parallel
ZipCPU cell already owns Layer 07be and `RCPU-085` / `RCPU-086`; this distinct
ZPU cell therefore uses the next unique layer and IDs. J1 is a separate,
supportable 16-bit Forth stack-machine candidate, but ZPU runs first: Zylin
publishes a stable architecture/programming document, an official instruction
simulator and GCC toolchain, and multiple matching VHDL cores in one vendor
repository. J1 remains a later evidence/profile audit rather than being
conflated with ZPU. The comparison checked James Bowman's public
[*J1: a small Forth CPU Core for FPGAs*](https://www.excamera.com/files/j1.pdf)
and BSD Verilog repository at immutable commit
[`fc30901a637be8208b83bc2b05b06c11ced61bb9`](https://github.com/jamesbowman/j1/tree/fc30901a637be8208b83bc2b05b06c11ced61bb9).
Those sources make J1 supportable, but its 16-bit Forth machine, memory map,
and host/target lifecycle need their own profile audit; they do not displace
the older, 32-bit, GCC-backed ZPU boundary selected here.

All normative material is pinned to Zylin's immutable upstream commit
[`e29aef5f7aa741557d0eb16ecf13e974d55789ff`](https://github.com/zylin/zpu/tree/e29aef5f7aa741557d0eb16ecf13e974d55789ff)
(2015-04-21):

| Source | Role | SHA-256 |
|---|---|---|
| [`zpu/docs/zpu_arch.html`](https://github.com/zylin/zpu/blob/e29aef5f7aa741557d0eb16ecf13e974d55789ff/zpu/docs/zpu_arch.html) | architecture, programming model, instruction families, interrupts, transport | `44661240d60ffdbcb6c7a88113a866de17ffa7875c5b2147a60814e4f57c752f` |
| [`zpu/hdl/zpu4/core/zpu_config.vhd`](https://github.com/zylin/zpu/blob/e29aef5f7aa741557d0eb16ecf13e974d55789ff/zpu/hdl/zpu4/core/zpu_config.vhd) | exact word/address/RAM/stack configuration | `e6b65d0d7cf41748c41444fbc2dd42980a48ee49ee264c79cab4c4240193ed7b` |
| [`zpu/hdl/zpu4/core/zpupkg.vhd`](https://github.com/zylin/zpu/blob/e29aef5f7aa741557d0eb16ecf13e974d55789ff/zpu/hdl/zpu4/core/zpupkg.vhd) | opcode constants and widths | `f7b36298967779778a4eb95ab4efb84035b46fa9d77c24181fbdf660298a94be` |
| [`zpu/hdl/zpu4/core/zpu_core_small.vhd`](https://github.com/zylin/zpu/blob/e29aef5f7aa741557d0eb16ecf13e974d55789ff/zpu/hdl/zpu4/core/zpu_core_small.vhd) | selected implementation decode, reset, stack, bus, interrupt, and break behavior | `2b9cca96b3e59204796a57866ff74a166db343846ae449ff91d193033d88fb70` |
| [`zpu/sw/simulator/com/zylin/zpu/simulator/Simulator.java`](https://github.com/zylin/zpu/blob/e29aef5f7aa741557d0eb16ecf13e974d55789ff/zpu/sw/simulator/com/zylin/zpu/simulator/Simulator.java) | secondary instruction-semantic cross-check, never authority over configuration-specific RTL | `7cefccdc3e7567acdb062054ae01a54f690cd392c51c8527c4eb489f54d1fa71` |
| [`zpu/COPYING`](https://github.com/zylin/zpu/blob/e29aef5f7aa741557d0eb16ecf13e974d55789ff/zpu/COPYING) | upstream architecture/tool and HDL licensing boundary | `8bc96909cb1b0dea55f1c1f98d3935085f2f1d9471e5b539782ef1de5a7f4eae` |

The selected RTL wins where the general architecture document or configurable
Java simulator admits another implementation choice. In particular, this
profile implements the ZPU4 **small** hardware decode: opcodes 0x20–0x3f take
the EMULATE vector, and unassigned short encodings 0x01, 0x03, 0x0e, and 0x0f
are NOP aliases. It does not silently substitute the Java simulator's optional
native implementations.

## 2. Immutable profile

The package profile is `Zpu4Small32x15V1`:

| Property | Pinned value |
|---|---|
| Package name | `zpu-simulator` |
| Data word | 32 bits, wrapping two's-complement arithmetic |
| Opcode | one byte |
| Internal program/data address | 15 bits, byte addresses 0x0000–0x7fff |
| External data-bus address | 16-bit aligned byte addresses 0x8000–0xfffc |
| Internal RAM | 32,768 bytes |
| Reset PC | 0x0000 |
| Reset SP | 0x7ff8 |
| Stack direction | push subtracts four; pop adds four; 15-bit word-address wrapping |
| Immediate latch | one `idim` bit |
| Interrupt latch | one level-rearm `in_interrupt` bit |
| Break output | sticky until reset/load |
| Upstream configuration | `wordPower=5`, `maxAddrBitIncIO=15`, `maxAddrBitBRAM=14`, `Generate_Trace=false` |
| Deterministic simulation override | `DontCareValue='0'` |

The functional model uses two-state values. The pinned configuration file's
default `DontCareValue='X'` marks unobserved internal wires, not architectural
state, and its own comment directs simulation users to select zero for
matching traces. This profile therefore pins the one-token `X`→`0` simulation
override; the otherwise byte-identical derived configuration has SHA-256
`166a3f446015b0efe4aad59728be76a297e6a70686ef1510fd33920147f1cdef`.
Where the RTL writes only address bits into a 32-bit stack word (for example
PUSHSP), this makes the documented byte address zero-extended and prevents an
unknown four-state value from leaking into the checked API. No other source
or configuration substitution is permitted.

## 3. Complete checked state

`ZpuState` is a frozen owned value containing:

- `pc: u16`, constrained to 0x0000–0x7fff;
- `sp: u16`, constrained to 0x0000–0x7ffc and divisible by four;
- `idim: bool`;
- `in_interrupt: bool`, the accepted-level latch that prevents repeated
  interrupt entry while the external level remains asserted;
- `interrupt_level: bool`, the deterministic sampled external input;
- `break_asserted: bool`;
- `internal_memory: [u8; 32768]`;
- `external_bus_words: [u32; 8192]`, a deterministic always-ready test adapter
  for aligned addresses 0x8000–0xfffc; and
- installed-image `origin: Option<u16>` plus `length: u32`.

The external array is environment state, not on-core RAM and not part of a
future core gate inventory. It exists so LOAD/STORE and interrupt-driven
software can complete deterministically without a hidden callback. Every
access is also exposed as a bus transaction in the trace. A later SoC cell
replaces this adapter with decoded devices, arbitration, wait states, and
interrupt sources.

At an instruction boundary there is no duplicate top-of-stack register:
`top = read_word(sp)` and `next = read_word(sp + 4)`. VHDL pipeline/state-machine
latches, RAM ports, sampled opcode, and trace wires are microarchitectural
cycle state and do not become restorable functional state.

`new` and `reset` zero both memories and all latches, set PC=0 and SP=0x7ff8,
clear image metadata, and deassert interrupt and break. This deterministic
choice contains the source reset state and zeroes otherwise unspecified RAM.

## 4. Byte transport, words, installation, and traces

Programs are raw opcode bytes. Byte address zero is the most-significant byte
lane of its 32-bit RAM word, matching `tOpcode_sel` in the selected VHDL.
Functional memory helpers read and write 32-bit words in big-endian byte order.
LOAD and STORE ignore address bits 1:0, exactly as the architecture document
and RTL do; they do not invent a misalignment fault.

`load(program, origin)` accepts a nonempty image only when `origin <= 0x7fff`
and the nonwrapping half-open host range lies wholly in internal RAM. A
successful load performs deterministic reset, installs bytes, records
origin/length, and sets PC to origin. Setting PC to a nonzero origin is an
explicit debugger/load convenience; hardware reset remains PC=0.

Instruction fetch is one byte and must lie in the installed image. A fetch in
internal RAM but outside that image is `FetchOutsideProgram`. Every one-byte
encoding is classified, so this profile has no truncated or unknown-opcode
case. Data and stack accesses are independent of installed-image membership.
Stores may self-modify installed code; traces retain the byte fetched before
the store commits.

An instruction trace contains PC and SP before/after, exact raw byte, stable
uppercase mnemonic, `idim`, interrupt latch/level and break before/after, top
two stack words before/after, optional aligned internal-memory access, optional
external `BusRead`/`BusWrite { address, value }`, and the complete post-state.
An accepted interrupt is a separate successful trace with no raw opcode and
mnemonic `INTERRUPT`; it records the pushed return PC and vector 0x0020.

## 5. Exhaustive decode

An independent frozen 256-entry table must classify every byte. The production
decoder implements exactly these disjoint rows:

| Bytes | Operation |
|---|---|
| 0x80–0xff | IM |
| 0x60–0x7f | LOADSP |
| 0x40–0x5f | STORESP |
| 0x20–0x3f | EMULATE |
| 0x10–0x1f | ADDSP |
| 0x00 | BREAK |
| 0x02 | PUSHSP |
| 0x04 | POPPC |
| 0x05 | ADD |
| 0x06 | AND |
| 0x07 | OR |
| 0x08 | LOAD |
| 0x09 | NOT |
| 0x0a | FLIP |
| 0x0b | NOP |
| 0x0c | STORE |
| 0x0d | POPSP |
| 0x01, 0x03, 0x0e, 0x0f | configuration-specific NOP aliases |

The trace mnemonic is `NOP` for every NOP alias; raw distinguishes them.
Parameterized mnemonics are `IM`, `LOADSP n`, `STORESP n`, `ADDSP n`, and
`EMULATE n`, where `n` is the exact effective word offset or vector index.

## 6. Stack and operation semantics

All reads use the pre-state and all changes commit together:

- A first IM sign-extends raw bits 6:0 to 32 bits, pushes it, and sets `idim`.
  A consecutive IM replaces top with `(top << 7) | (raw & 0x7f)` and retains
  `idim`. Every accepted interrupt and every non-IM instruction clears it.
- `offset5 = (raw & 0x1f) XOR 0x10`. LOADSP pushes the word at
  `pre_sp + 4*offset5`; STORESP writes pre-state top there and pops once.
- ADDSP uses `raw & 0x0f`, adds the word at `pre_sp + 4*offset` to top modulo
  2^32, and replaces top without changing SP.
- EMULATE pushes `pre_pc + 1` and sets PC to `(raw & 0x1f) << 5`. It is a real
  control transfer, not a typed unsupported fault.
- PUSHSP pushes the zero-extended pre-state byte SP. POPSP replaces SP with
  top bits 14:2, thereby enforcing word alignment. POPPC pops top into PC bits
  14:0. PC and stack address arithmetic wrap at their selected RTL widths.
- ADD, AND, and OR pop `a=top` and `b=next`, then leave the wrapped/result word
  at the new top. NOT and FLIP replace top in place; FLIP reverses all 32 bits.
- LOAD replaces top with the aligned addressed word. STORE pops address then
  value and writes that value to the aligned address. Addresses with bit 15
  clear select internal RAM at `address & 0x7ffc`; addresses with bit 15 set
  select the external adapter at `address & 0xfffc` and produce the
  corresponding transaction. Bits above 15 and alignment bits 1:0 are
  discarded exactly at the selected core's ports.
- BREAK advances PC, asserts the sticky break output, and produces a successful
  trace. It does not fabricate an ISA halt. `run` stops after observing the
  asserted output; explicit `step` remains allowed, matching the core's ability
  to continue if its environment does not stop the clock. NOP only advances PC.

Word-address stack calculations wrap modulo 8192 words. Tests must exercise
push/pop wrap, source/destination aliases, stack writes overlapping code, and
STORE where address/value stack slots alias the destination word.

## 7. Interrupt boundary

`set_interrupt_level(bool)` is a validated explicit environment action included
in snapshots; it changes only the input level. `step` first validates that the
current PC has a fetchable installed-image byte, then samples the interrupt
input. A high level is accepted only when `in_interrupt=false` and
`idim=false`. Acceptance occurs before executing the already-addressed opcode:
push the current PC, set PC=0x0020, clear `idim`, set `in_interrupt`, and return
an `INTERRUPT` trace. The preflighted byte is not consumed or reported as raw.
Thus `FetchOutsideProgram` has precedence and cannot partially accept an
interrupt.

While the level remains high, no second interrupt is taken. Sampling a low
level clears `in_interrupt`, rearming a later rising/high level. Consecutive IM
bytes defer interrupt acceptance until a non-IM boundary clears `idim`. Tests
must cover reset-high, low/high rearm, sustained high, every first/continued IM
position, stack wrap, vector overlap with installed code, and a fetch fault
that must not partially accept an interrupt.

This is the selected core's single interrupt pin. Priorities, masking devices,
timers, UARTs, and nested software policy belong to later SoC work.

## 8. Lifecycle, faults, and allocation

The checked API provides new/reset/load/snapshot/restore, internal/external
memory helpers, interrupt-level control, `step`, `run(max_steps)`, and
`execute(program, origin, max_steps)`. Internal word helpers accept only
aligned byte addresses 0x0000–0x7ffc; external-adapter helpers accept only
aligned byte addresses 0x8000–0xfffc. Helper misuse is a typed
`MisalignedAddress` or `AddressOutOfRange` and never applies the ISA's masking
rule. `run` stops successfully when BREAK asserts; if it starts with BREAK
already asserted, it returns no traces. `execute` is atomic across load and
run. Retained complete-state traces are capped at 64; requesting a larger
limit is `TraceLimit` before execution, while exhausting an allowed positive
limit without BREAK is `StepLimit`. A zero limit is an unchanged-state
`StepLimit`.

Stable typed errors distinguish empty image, program out of range, no image,
fetch outside the installed image, invalid restored state, misaligned helper
address, helper address out of range, step limit, trace limit, allocation
failure, and a source-preserving execute wrapper. A step performs, in order:
complete-state validation; image-presence and installed-range fetch
validation; interrupt qualification; frozen-table decode; all pre-state
stack/data/bus reads; complete candidate-state and trace construction; then
one commit. Allocation failure during candidate/trace construction therefore
precedes commit. There is no unknown-opcode or ISA misalignment fault in this
profile. Every error preserves PC, SP, latches, interrupt input, break output,
both memories, and image metadata exactly.

All potentially fallible allocations are deterministic test seams. Tests must
inject failure during snapshot construction, trace reservation, `run` after at
least one successful instruction, and `execute` after load/progress, proving
complete entry-state rollback.

## 9. Differential and completion gate

Implementation is incomplete until it provides:

1. a checked-in immutable URL/SHA-256 manifest for the six pinned files. The
   explicit regeneration command fetches into a disposable directory and
   refuses any mismatch before an oracle run; ordinary offline tests consume
   the frozen corpus and never require the network;
2. a production-independent frozen 256-byte classification table and execution
   witnesses for every parameter value, all NOP aliases, and every control
   target;
3. arithmetic/bit-reversal truth oracles, first/continued IM boundaries,
   every LOADSP/STORESP/ADDSP offset, stack wrapping and alias matrices,
   big-endian byte lanes, alignment-bit ignoring, and self-modification;
4. all interrupt/IDIM/rearm cases and every internal/external bus selection,
   value, transaction, and rollback boundary;
5. a deterministic GHDL harness around checksum-verified `zpu_config.vhd`
   (with only the pinned `DontCareValue='0'` substitution), exact `zpupkg.vhd`,
   and exact `zpu_core_small.vhd`, plus a dual-port RAM and always-ready external
   adapter; the harness must emit frozen instruction-boundary vectors for all
   256 bytes and representative multistep/interrupt programs. The checked-in
   command must use GHDL's explicit non-stopping assertion threshold for the
   source's intentional BREAK `severity failure`. Harness failures must call
   `std.env.stop(1)` (not rely on that disabled threshold), and the runner must
   require the final PASS sentinel, reject unexpected diagnostics, and reject
   every nonzero process result;
6. two independent consumers of that corpus: the Rust functional package and
   a small source-independent schema/semantic witness. A missing GHDL tool is a
   reported blocker, never a fabricated differential pass;
7. reset/load/restore/step/run/execute and every typed fault with exact complete
   state equality, including deterministic post-progress allocation failures;
8. package README/CHANGELOG/BUILD/workspace integration, Rustfmt, strict Clippy,
   rustdoc, release tests, lessons validation, affected builds, and greater
   than 80% package-source line coverage; and
9. a different reviewer's exact-clean-head correctness/security PASS before
   implementation publication.

The upstream Java simulator is a secondary oracle only for operations shared
with this small configuration. It may not override the VHDL's EMULATE and NOP
choices, memory widths, reset values, or interrupt timing.

## 10. Dependent gate and system work

After this functional oracle merges, Spec 07bf2 may define a source-backed
educational gate model of this exact ZPU4-small VHDL profile. That specification
must derive—not guess—the complete stable-state and state-machine inventory;
make repository gates/DFFs authoritative for decode, stack/data paths, PC/SP,
IDIM, interrupt, break, memory ports, and commit; run cycle and
instruction-boundary differentials against the pinned GHDL harness; and reject
host/oracle bypasses with parsed structural locks and compiler-valid hostile
fixtures. The claim is a translation of published RTL, not Zylin's physical
netlist or transistor topology.

Separate SoC cells may add ROM/RAM wrappers, Wishbone, UART, timer, debug,
interrupt control, DMA/interconnect movement, wait states, and board profiles.
Cache/MMU behavior, analog timing, FPGA primitive timing, transistor counts,
and SPICE claims are excluded until independently sourced lower-level models
land.
