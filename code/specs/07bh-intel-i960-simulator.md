# Layer 07bh — Intel 80960KA Scalar Functional Simulator

**Status:** `RCPU-091` is specification-only and requires independent
pre-implementation review. `RCPU-092` is reserved for a dependent, explicitly
educational synthesized gate model; it cannot begin before the functional
oracle merges and must not claim Intel die or netlist fidelity.

## 1. Discovery, priority, and authority

This layer specifies a checked instruction-boundary model of a bounded Intel
80960KA application-programming profile. It is a CPU and ordered memory-
transaction oracle. It is not an 80960KB floating-point model, an 80960MC
protected architecture, an interrupt controller, an Intel board, a bus-cycle
model, an SoC, or a transistor/netlist reconstruction.

The discovery audit was reconciled with `origin/main`
`e2119773be5b8881191bcb5b8657946d2b51ed5f` on 2026-10-28. Repository
packages, specifications, history, local and remote branches, registered
worktrees, and every open, closed, or merged pull request contained no i960,
80960, or Intel 960 simulator claim. Layer 07bf and `RCPU-087` / `RCPU-088`
belong to the distinct Zylin ZPU cell. Layer 07bg and `RCPU-089` / `RCPU-090`
remain reserved for the parallel MIPS R4000 audit, so this cell uses the next
unique layer and identifiers.

The normative source is Intel, *80960KB Programmer's Reference Manual*, March
1988, order number 270567. The manual explicitly says that its architectural
material applies to the 80960KA and that the KA omits the KB's floating-point
and decimal support. The immutable reviewed archive is:

- <https://bitsavers.trailing-edge.com/components/intel/i960/80960KB_Programmers_Reference_Manual_Mar88.pdf>
- SHA-256
  `12c88f1848863d991ad869de2af54121e1046c7124fc96c598210018b4429a47`
- 445 pages, 20,413,561 bytes.

Intel's *80960KA/KB Specification Update*, release 002, June 1997, is a
required errata cross-check rather than a replacement architecture manual:

- <https://datasheets.chipdb.org/Intel/80960/specupdt/27285102.pdf>
- SHA-256
  `9acf3eb85189e3822d336102ae61da79e97b434b7154bd9279db5c6ff316548b`.

The implementation must record and test every specification-update item that
intersects this profile. Hardware timing, electrical, breakpoint, interrupt,
and register-cache errata outside the admitted boundary remain exclusions;
they may not silently alter architectural results.

MAME 0.289 provides independent secondary execution evidence at immutable
source commit
[`f34f02505e32c1993c6a782b6814232cbfc74e36`](https://github.com/mamedev/mame/tree/f34f02505e32c1993c6a782b6814232cbfc74e36).
Its i960 core is not normative and is not production code. It is useful
because it separately identifies an `i80960ka_device`, selects little-endian
32-bit memory, and implements the four instruction formats and much of the
selected integer surface. The exact evidence files are:

| MAME source | SHA-256 |
|---|---|
| `src/devices/cpu/i960/i960.cpp` | `a1b44f3234a117d956461a50e1ec6eb4c8a02421b19db7906e8ffce17f3b8b19` |
| `src/devices/cpu/i960/i960.h` | `8bd91f9ce8ca2d137723d09d95b2a43eb7a404b9fff8efeed2de3aca9f2d78b5` |
| `src/devices/cpu/i960/i960dis.cpp` | `9dacecda192083d07e1f390d29b65b29565eb64a2f6c0796cebee681a02303d5` |
| `src/devices/cpu/i960/i960dis.h` | `d3a15cb6b50f2bd1b40f36eecb266c430517d007e8414526168a8d85268e8a99` |

MAME has incomplete fault handling and a broader machine environment. Only
rows that are independently shown to lie in the common V1 boundary may be
used as differential evidence. A MAME disagreement must be classified against
the Intel manual; it must never be hidden by changing the Rust result.

The i960 is prioritized after the already active source-backed ZPU and MIPS
cells because it is an otherwise missing, historically important 1988 RISC
family with a complete public Intel programming reference and a separately
implemented open emulator. It ranks ahead of evidence-blocked proprietary
microarchitectures. The absence of public KA RTL or a gate/netlist disclosure
means `RCPU-092` can only be an openly labeled repository-synthesized
educational gate model.

## 2. Immutable V1 profile

The package profile is `I960KaScalarIntegerV1`:

| Property | Pinned value |
|---|---|
| Package | `intel-i960-simulator` |
| Architecture | Intel 80960KA common scalar integer application surface |
| Data/address width | 32 bits |
| Instruction word | 32 bits; selected MEMB forms contain a second 32-bit displacement word |
| Byte order | little-endian instructions and data |
| Register file | sixteen local `r0..r15` plus sixteen global `g0..g15`, each 32 bits |
| Instruction pointer | aligned 32-bit byte address |
| Arithmetic controls | CC bits 2:0, integer-overflow flag bit 8, integer-overflow mask bit 12, no-imprecise-faults bit 15; all other KA-reserved bits read zero |
| Owned storage | 65,536 bytes at `0x0000_0000..=0x0000_FFFF` |
| Data alignment | KA behavior: byte, short, and word accesses may be unaligned |
| Program installation | nonempty, four-byte aligned origin and length, wholly inside owned storage |
| Execution mode | application profile only; no supervisor/kernel transition |
| Stop model | no invented HALT opcode or halt latch; bounded runs finish with `StepBudgetReached` |

The profile preserves the characteristic split local/global register
namespace, literal operands, condition code, four instruction formats, rich
addressing modes, and scalar integer/data/control surface. It intentionally
does **not** claim the complete KA processor.

## 3. Complete authoritative state

`I960State` is an owned frozen value containing exactly:

- `local: [u32; 16]` for `r0..r15`;
- `global: [u32; 16]` for `g0..g15`;
- aligned `ip: u32`;
- three-bit `condition_code`;
- `integer_overflow_flag`, `integer_overflow_mask`, and
  `no_imprecise_faults` Booleans;
- all 65,536 memory bytes; and
- installed-image `origin: Option<u32>` and `length: u32`.

The flattened machine register numbers used by encodings are 0..15 for
`r0..r15` and 16..31 for `g0..g15`. In this call-free V1, `r0`/PFP, `r1`/SP,
`r2`/RIP, and `g15`/FP remain ordinary stored architectural registers; no
hidden procedure-frame behavior is invented. Their reserved calling-
convention roles become active only in a later call/return extension.

The AC read projection is
`cc | (overflow_flag << 8) | (overflow_mask << 12) |
(no_imprecise_faults << 15)`. Every other bit reads zero. `modac` may update
only those projected fields; writes to reserved bits are ignored as the
manual's initialize-to-zero contract requires. There is no duplicate raw AC
word that can disagree with those fields.

`new` chooses zero for registers, AC fields, IP, memory, and image metadata.
`reset` restores registers, AC fields, and IP to those deterministic values
while preserving memory and valid image metadata. `load` performs the same
CPU reset, clears memory, installs the image, records its span, and sets IP to
the origin. `restore` validates the complete candidate before commit: IP and
image endpoints are aligned; metadata is either canonical unloaded
`(None, 0)` or a nonempty, nonwrapping span wholly inside memory; CC is at
most seven. No field is reconstructed from a trace or hidden host mirror.

## 4. Transport, fetch, and memory transactions

Programs are raw little-endian instruction words. `load(program, origin)`
requires nonempty input, four-byte aligned origin and byte length, and a
nonwrapping half-open range wholly inside owned memory. Validation and all
fallible allocation preflight happen before state changes.

A step first fetches the four bytes at IP. A missing image is
`NoProgramLoaded`; an unaligned IP is `MisalignedFetch`; a first word not
wholly inside the installed image is `FetchOutsideProgram`. Decode of that
word determines whether a MEMB displacement word is required. If so, the
next four bytes must also lie in the installed image or the result is
`TruncatedInstruction`. Thus an eight-byte form can never read zero-filled
bytes beyond the image.

Data accesses are independent of installed-image membership and may overlap
code. Byte, short, and word accesses use one, two, or four successive bytes in
little-endian order. The KA permits unaligned data access, so V1 does not
invent a misalignment fault. The complete byte span must fit owned memory
without 32-bit wrap before any byte is read or written. Self-modifying stores
affect later fetches; the current trace retains its already fetched bytes.

Each successful step records ordered transactions. An instruction fetch is
one or two four-byte reads. A scalar data operation adds one read or write
with exact address, width, value, little-endian byte order, and byte-enable
mask (`0x01`, `0x03`, or `0x0f`). `lda`, branches, and register operations do
not fabricate data transactions. Bus cycles, cache hits, burst transfers,
wait states, arbitration, and timing are excluded.

## 5. Exact decode formats

The first word always selects exactly one manual format. Production decode
must validate fixed/reserved fields before operand reads or state changes.

### 5.1 REG

REG has an eight-bit primary opcode in bits 31:24 and a four-bit secondary in
bits 10:7. The combined 12-bit opcode is written below as three hexadecimal
digits. `src1` is bits 4:0, `src2` bits 18:14, and `src/dst` bits 23:19.
Mode bits M1, M2, and M3 select register or literal exactly as Appendix B:
for scalar integer sources a clear mode selects the 32-register namespace and
a set mode selects zero-extended literal 0..31. A destination must be a
register; a literal destination is `InvalidEncoding`.

### 5.2 COBR

COBR uses the eight-bit primary opcode, `src1` in bits 23:19, `src2` in bits
18:14, signed displacement bits 12:2, and fixed-zero bits 1:0. M1 allows
`src1` to be a register or literal; `src2` is always a register. Test forms
use bits 23:19 as a register destination and ignore the otherwise unused
source/displacement fields exactly as the Intel table states. A taken target
is `next_ip + sign_extend(displacement[12:2] << 2)`.

### 5.3 CTRL

CTRL uses the eight-bit primary and signed displacement bits 23:2 with
fixed-zero bits 1:0. Its target is
`instruction_ip + sign_extend(displacement[23:2] << 2)`. This deliberately
differs from COBR's next-IP base and is covered by positive and negative
offset witnesses.

### 5.4 MEMA and MEMB

MEM uses the eight-bit primary and register field bits 23:19. Bit 12 clear is
MEMA: mode bit 13 chooses unsigned `offset12` or `base + offset12`. Bit 12 set
is MEMB: bits 13:10 select the manual mode, bits 9:7 the scale, bits 18:14 the
base, and bits 4:0 the index. Modes are:

| Mode | Effective address |
|---:|---|
| `0100` | base |
| `0101` | instruction IP + signed displacement word + 8 |
| `0110` | reserved, `InvalidEncoding` |
| `0111` | base + (index << scale) |
| `1100` | signed displacement word |
| `1101` | base + signed displacement word |
| `1110` | (index << scale) + signed displacement word |
| `1111` | base + (index << scale) + signed displacement word |

All additions wrap modulo 2^32 before whole-span range validation. Scale
encodings 0..4 mean factors 1, 2, 4, 8, and 16; 5..7 are reserved and invalid
when the selected mode consumes the index. Fields unused by a selected manual
mode are ignored, not retroactively declared reserved.

## 6. Admitted instructions

The implementation transcribes opcodes from manual Appendix A, never from
MAME or from its own tests. The complete V1 surface is:

| Format/opcodes | Instructions |
|---|---|
| CTRL `08`, `0B` | `b`, `bal` (`bal` writes next IP to `g14`) |
| CTRL `10..17` | `bno`, `bg`, `be`, `bge`, `bl`, `bne`, `ble`, `bo` |
| COBR `20..27` | `testno`, `testg`, `teste`, `testge`, `testl`, `testne`, `testle`, `testo` |
| COBR `30`, `37` | `bbc`, `bbs` |
| COBR `31..36` | ordinal compare-and-branch `cmpob*` |
| COBR `38..3f` | signed compare-and-branch `cmpib*` |
| MEM `80`, `88`, `90`, `c0`, `c8` | `ldob`, `ldos`, `ld`, `ldib`, `ldis` |
| MEM `82`, `8a`, `92`, `c2`, `ca` | `stob`, `stos`, `st`, `stib`, `stis` |
| MEM `84`, `85`, `8c` | `bx`, `balx`, `lda` |
| REG `580..58f`, excluding unassigned `585` | `notbit`, `and`, `andnot`, `setbit`, `notand`, `xor`, `or`, `nor`, `xnor`, `not`, `ornot`, `clrbit`, `notor`, `nand`, `alterbit` |
| REG `590..59e` admitted rows | `addo`, `addi`, `subo`, `subi`, `shro`, `shri`, `shlo`, `rotate`, `shli`; `shrdi` is outside V1 |
| REG `5a0..5a7`, `5ac`, `5ae` | `cmpo`, `cmpi`, `concmpo`, `concmpi`, `cmpinco`, `cmpinci`, `cmpdeco`, `cmpdeci`, `scanbyte`, `chkbit` |
| REG `5b0`, `5b2` | `addc`, `subc` |
| REG `5cc` | `mov` |
| REG `640`, `641`, `645`, `650`, `651` | `spanbit`, `scanbit`, `modac`, `modify`, `extract` |
| REG `701`, `708`, `70b` | `mulo`, `remo`, `divo` |
| REG `741`, `748`, `749`, `74b` | `muli`, `remi`, `modi`, `divi` |

`stib` and `stis` are architecturally distinct encodings but store the same
low byte/short data as `stob` and `stos`; their traces retain the encoded
mnemonic. Loads extend according to `o` (zero) or `i` (sign). `balx` writes
the address following its complete four- or eight-byte instruction to its
destination before transferring to the effective address. All source values,
including address registers and a destination used as a source, come from the
pre-state.

The KA instruction pointer has no storage for bits 1:0. Every control-transfer
target is therefore canonicalized as `target & !0x3` before it becomes the
post-state IP. CTRL and COBR displacements already produce that result because
their low two encoded bits are fixed zero; `bx` and `balx` explicitly discard
the low two bits of the complete MEM effective address. This is not a data-
alignment fault and does not change the `balx` link value. All target operands
and any link destination are still sampled from the pre-state before the
canonical IP is committed. Tests must cover low-bit values zero through three,
source/link aliasing, and four- and eight-byte `balx` forms, and must prove that
no successful step can create an `I960State` that `restore` rejects solely for
IP alignment.

Ordinal arithmetic wraps modulo 2^32. Integer arithmetic uses two's-
complement values. Shifts use the low five bits of the count; right integer is
arithmetic and right ordinal is logical. Rotate is a 32-bit rotate. Signed
division truncates toward zero, `remi` has the dividend's sign, and `modi`
adjusts a nonzero remainder to the divisor's sign exactly as Chapter 11
specifies. Division by zero is a typed atomic `DivisionByZero` boundary.

`addi`, `subi`, `muli`, and the signed division overflow case obey AC.IOM. If
overflow is masked, the wrapped result commits and sticky AC.IOF is set. If
unmasked, V1 reports typed `IntegerOverflow` with complete state unchanged
instead of fabricating the excluded fault-table transition. `addc` and `subc`
always update the documented carry and signed-overflow CC bits and never
raise integer overflow.

Comparisons set exact manual CC encodings: less `100`, equal `010`, greater
`001`; conditional comparisons retain or replace CC per their documented
predicate. Conditional branches and tests use the opcode's three-bit mask;
the `no` form uniquely selects CC `000`, while the other seven forms test
whether `mask & CC` is nonzero. `bbc`/`bbs`, bit operations, `extract`, and
scan operations mask or bound positions exactly as their manual actions say.
Every `cmpib*`/`cmpob*` first installs the comparison CC and branches from
that value; `bbc`/`bbs` install CC=`010` when their selected condition is true
and CC=`000` otherwise. Length-zero, bit-31, all-zero, and all-one edges
require explicit witnesses.

## 7. Deliberate exclusions and classification

Valid KA/KB encodings outside V1 return typed `UnsupportedInstruction` after
complete transport but before architectural mutation. This set includes:

- local `call`, `callx`, and `ret`, register-frame caching/spilling, and
  `flushreg`;
- `calls`, process/trace controls, interrupts, initialization, system tables,
  fault-table entry, `fault*`, `mark`, `fmark`, and `syncf`;
- multi-register `movl/movt/movq`, `ldl/ldt/ldq`, `stl/stt/stq`, `emul`,
  `ediv`, and `shrdi`;
- atomic `atadd`/`atmod` and synchronous `synld`/`synmov*` bus operations;
- decimal operations and all real/floating-point instructions, registers,
  conversions, masks, and status; and
- every later Cx/Jx/Hx/Rx extension.

Unassigned primary/REG-secondary opcodes are `UnknownInstruction`; malformed
fixed bits, reserved MEM mode/scale, or literal destination are
`InvalidEncoding`. A known excluded opcode is never mislabeled unknown.
Transport precedence is retained: a recognized eight-byte excluded MEMB form
whose displacement word is absent is `TruncatedInstruction`, not
`UnsupportedInstruction`.

The omitted call mechanism is a separately reviewable extension because it
adds cached local-register frames, stack-frame alignment, spill/fill memory
side effects, and return/fault precedence. External interrupts and fault-
handler transitions likewise require system tables and process controls.
They must not be approximated inside this scalar cell.

## 8. Trace and checked lifecycle

Every successful `step` returns `I960StepTrace` containing:

- IP before/after, exact four or eight fetched bytes, first raw word and
  optional displacement word;
- stable lowercase architectural mnemonic and decoded format;
- every source register/literal index and sampled value, optional destination
  index/value, AC before/after, branch decision, and target;
- ordered instruction and data transactions; and
- the complete post-state.

There is no public unchecked setter. The checked API provides `new`, `reset`,
`load`, `snapshot`, `restore`, byte-addressed memory helpers, `step`,
`run(max_steps)`, and `execute(program, origin, max_steps)`. `max_steps` must
be in `1..=64`. The fixed retained-trace cap is therefore 64, and each retained
trace may own exactly one complete 64-KiB post-state. A zero or larger budget
is rejected as typed `StepLimitOutOfRange` before load, execution, or result
allocation. Because KA has no architectural halt instruction in this boundary,
a successful run executes exactly its accepted budget and returns
`StopReason::StepBudgetReached`; the budget is not an error and no halt state
is invented. There is no unchecked or streaming path that can retain more
than 64 complete-state traces in one result.

Any step error leaves complete state unchanged. `run` is transactional across
all steps: an instruction, memory, or allocation error after progress restores
the complete call-entry state and returns no partial authoritative traces.
`execute` is atomic across load and run. Snapshot/trace/result allocations use
`try_reserve`/`try_reserve_exact` and a deterministic test-only failure seam.
`new`/`Default` may perform the one fixed 64-KiB allocation outside the typed
allocation contract; after construction, every checked-operation allocation
is fallible and rollback-safe.

The error enum is closed over `AllocationFailed`, `NoProgramLoaded`,
`EmptyProgram`, `MisalignedProgram`, `ProgramOutOfRange`, `MisalignedFetch`,
`FetchOutsideProgram`, `TruncatedInstruction`, `UnknownInstruction`,
`InvalidEncoding`, `UnsupportedInstruction`, `MemoryOutOfRange`,
`DivisionByZero`, `IntegerOverflow`, `InvalidState`, and
`StepLimitOutOfRange`. Errors carry stable context but never retain partial
mutable state.

## 9. Completion evidence

`RCPU-091` is complete only when all of the following pass:

1. A frozen independent decoder table covers all 256 primary bytes; all 16
   REG secondary values for each REG primary; every MEM mode and scale; both
   forms of every fixed bit; register/literal selectors; destination modes;
   and known unsupported versus reserved rows. Production and expected tables
   cannot share constructors or constants.
2. All admitted instructions have manual-derived transition witnesses for
   every operand role, register/literal selector, local/global boundary,
   source/destination alias, arithmetic boundary, CC input, taken/not-taken
   control path, and relevant four/eight-byte transport.
3. Exhaustive truth tables cover all eight CC values for each branch/test
   mask, all 32 bit positions, all shift counts, byte/short sign extension,
   MEM modes/scales, unaligned offsets, end-of-memory spans, integer overflow
   mask behavior, and divide/remainder/modulo signs and exceptional edges.
4. A deterministic manifest regenerates a checksum-pinned corpus by executing
   the exact MAME 0.289 `i80960ka_device` for the common supported subset.
   The harness must build from the pinned sources using MAME's documented
   build system; copied Rust expectations or a reimplementation labeled MAME
   are forbidden. If the real oracle cannot run, the cell remains incomplete.
   Normal package builds and tests are network-free: the explicit regeneration
   command accepts only an exact clean pinned source tree (or individually
   hash-verified staged files), bounds rows and output bytes, and never
   downloads or executes moving remote content implicitly.
5. Rust and an independent Python consumer replay every corpus row and compare
   complete projected state plus ordered transactions. Manual-derived rows
   independently cover MAME gaps and every documented disagreement.
6. Lifecycle matrices cover reset/load/restore, self-modifying code,
   four/eight-byte truncation, every typed fault, and deterministic allocation
   failure at first allocation, after at least one successful instruction,
   while materializing trace post-state, and while restoring rollback state.
   They also cover accepted budgets 1 and 64, rejected budgets 0 and 65,
   `execute` limit precedence before load, and prove that rejected limits leave
   complete state unchanged without allocating a result.
7. Package README and changelog state the scalar boundary and exclusions;
   workspace and `BUILD` integration are complete; formatting, release tests,
   Clippy with warnings denied, rustdoc with warnings denied, lessons
   validation, affected builds, and measured package-source line coverage
   above 80% pass.
8. A different agent reviews the exact clean implementation head for source
   fidelity, security/resource bounds, decode completeness, and atomicity
   before publication.

## 10. Dependent gate and systems work

`RCPU-092` may start only after `RCPU-091` merges and after its own spec passes
independent review. Public Intel material used here exposes architectural and
external behavior, not the KA's internal gate netlist. Therefore the gate
cell must call itself an **educational repository-synthesized gate model**,
derive storage and datapath topology from its own published design, use the
functional package solely as a test oracle, and never claim exact 80960KA
gate count, placement, timing, or transistor fidelity.

Later independent cells may add local call/return and frame spill/fill,
architectural faults and interrupts, atomics and synchronous bus operations,
multi-register transfers, KB floating point, and source-backed board/SoC
composition. Caches, pipelines, bus cycles, peripherals, DMA, arbitration,
electrical behavior, physical implementation, and SPICE/transistor descent
remain outside this CPU profile.
