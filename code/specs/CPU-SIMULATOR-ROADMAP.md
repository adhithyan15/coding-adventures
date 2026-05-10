# CPU Simulator Roadmap

**Goal**: Build a complete suite of behavioral simulators covering every major instruction
set architecture from 1948 mainframes through modern mobile processors — so that any ISA
back-end can be developed, tested, and debugged without owning the physical hardware.

Each simulator implements the **SIM00 `Simulator[State]` protocol**:
`reset()`, `load()`, `step()`, `execute()`, `get_state()`,
`set_input_port()`, `get_output_port()`, `interrupt()`, `nmi()` (where applicable).

---

## Why this matters

Compiler back-ends, binary translators, and low-level debuggers all need a faithful
execution environment.  Owning every physical board is impossible; owning a suite of
simulators is not.  This roadmap builds toward the ability to:

- Write and test code for any ISA in the collection without the physical CPU
- Cross-compile from any source language to any target ISA
- Replay and bisect hardware-specific bugs in a deterministic simulator
- Teach computer architecture bottom-up, from relay logic to AArch64

---

## Numbering convention

Simulators live in `code/packages/python/` and are numbered within the `07*` family.
The "tik-tok" alternation below explains the odd-lettered (IC) vs even-lettered
(mainframe) split.

- **Odd letters** (k, m, o, q, s, u, v, w, x, y) — post-4004 integrated circuits
- **Even letters** (l, n, p, r, t) — pre-4004 mainframes / minicomputers

---

## Completed simulators

| Layer | Package | Processor | Year | Notes |
|-------|---------|-----------|------|-------|
| 07i   | `intel8080-simulator` | Intel 8080  | 1974 | Gate-level + behavioral |
| 07j   | `mos6502-simulator`   | MOS 6502    | 1975 | Full NMOS; BCD mode; JMP indirect bug |
| **07k** | **`z80-simulator`** | **Zilog Z80** | **1976** | **← current PR** |

---

## Upcoming: "Tik-Tok" alternating plan

The plan alternates between post-4004 ICs and pre-4004 mainframes/minicomputers so
each session adds variety while building a complete historical record.

### Round 1 — just completed
| Step | Layer | Processor | Year | Why it matters |
|------|-------|-----------|------|----------------|
| ✅ | 07k | Zilog Z80 | 1976 | Direct 8080 superset; powered TRS-80, ZX Spectrum, CP/M |

### Round 2 — next up
| Step | Layer | Processor | Year | Why it matters |
|------|-------|-----------|------|----------------|
| ⬜ | 07l | Manchester Baby (SSEM) | 1948 | First stored-program computer ever run |
| ⬜ | 07m | Intel 8086 | 1978 | Birth of x86; segmented memory; IBM PC |

### Round 3
| Step | Layer | Processor | Year | Why it matters |
|------|-------|-----------|------|----------------|
| ⬜ | 07n | EDSAC | 1949 | First practical stored-program computer; Wheeler jump |
| ⬜ | 07o | Motorola 68000 | 1979 | Mac, Amiga, Atari ST, early Sun workstations |

### Round 4
| Step | Layer | Processor | Year | Why it matters |
|------|-------|-----------|------|----------------|
| ⬜ | 07p | PDP-8 | 1965 | First mass-market minicomputer; 12-bit words; paper tape |
| ⬜ | 07q | Intel 80386 | 1985 | 32-bit x86; protected mode; paging; Linux foundation |

### Round 5
| Step | Layer | Processor | Year | Why it matters |
|------|-------|-----------|------|----------------|
| ⬜ | 07r | IBM System/360 | 1964 | First family architecture; EBCDIC; fixed-length ISA |
| ⬜ | 07s | MIPS R3000 | 1988 | RISC pioneer; PlayStation; SGI; MIPS32 still in routers |

### Round 6
| Step | Layer | Processor | Year | Why it matters |
|------|-------|-----------|------|----------------|
| ⬜ | 07t | CDC 6600 | 1964 | First supercomputer; scoreboarding; Seymour Cray |
| ⬜ | 07u | PowerPC 601 | 1992 | Apple/IBM/Motorola AIM alliance; BeOS, classic Mac |

### Round 7 — 64-bit era
| Step | Layer | Processor | Year | Why it matters |
|------|-------|-----------|------|----------------|
| ⬜ | 07v | x86-64 (AMD64) | 2003 | Ubiquitous server/desktop ISA; 64-bit x86 |
| ⬜ | 07w | ARMv7-A (Cortex-A8) | 2004 | First iPhone-era ARM; Thumb-2; VFP/NEON |

### Round 8 — modern mobile
| Step | Layer | Processor | Year | Why it matters |
|------|-------|-----------|------|----------------|
| ⬜ | 07x | AArch64 (ARMv8-A) | 2011 | Apple A7+, all modern Android; clean 64-bit ARM |
| ⬜ | 07y | RISC-V (RV32I/RV64I) | 2010 | Open standard; the future of embedded and HPC |

### Round 9 — contemporary targets
| Step | Layer | Processor | Year | Why it matters |
|------|-------|-----------|------|----------------|
| ⬜ | 07z | Apple M1 (AArch64 ext.) | 2020 | Apple Silicon; unified memory; performance islands |

---

## Architecture notes per upcoming simulator

### 07l — Manchester Baby (SSEM, 1948)
- 32-bit words, 32-word store (Williams tube)
- 7 instructions: JMP, JRP, LDN, STO, SUB, CMP (skip), STP
- Accumulator-only; no I/O ports; halts on STP
- Fun fact: first program computed the highest factor of 2^18

### 07m — Intel 8086 (1978)
- 16-bit registers (AX/BX/CX/DX + SI/DI/SP/BP)
- Segmented memory: CS:IP, SS:SP, DS/ES + effective address
- 256 instructions; string ops (MOVS/CMPS/SCAS/LODS/STOS + REP)
- Hardware INT/IRET; software INT n
- 1 MB address space via segment*16+offset

### 07n — EDSAC (1949)
- 17-bit words (plus sign bit = 18 bits), serial bit-serial arithmetic
- ~20 order types (add, subtract, multiply, shift, branch, I/O)
- Subroutine library via Wheeler jump (BSR/RET predecessor)
- Mercury delay-line memory; 512–1024 locations

### 07o — Motorola 68000 (1979)
- 8 data registers D0–D7 + 8 address registers A0–A7 (A7 = SP)
- 24-bit address bus (16 MB), 32-bit internal
- Rich addressing modes: indirect, displacement, index, PC-relative
- MOVEM, BTST/BCHG/BCLR/BSET; DIVS/MULS; LINK/UNLK
- Supervisor/user mode; exception vectors at 0x000

### 07p — PDP-8 (1965)
- 12-bit words, 4K word pages, accumulator + link bit
- 8 memory reference instructions + group 1/2/3 micro-ops
- Auto-index registers (locations 0o10–0o17)
- IOT instructions for I/O; single interrupt flag

### 07q — Intel 80386 (1985)
- 32-bit registers (EAX/EBX/ECX/EDX/ESI/EDI/ESP/EBP)
- Protected mode: segments + 4 GB flat via GDT/LDT
- Paging (4 KB pages, CR3); CPL 0–3
- FPU (80387) optional; all 8086/286 instructions extended

### 07r — IBM System/360 (1964)
- 16 general 32-bit registers (R0–R15)
- 64-bit floating point (S/360 hex FP); EBCDIC character set
- Fixed-length 16/32/48-bit instructions
- 24-bit logical address space; channel I/O (not simulated)

### 07s — MIPS R3000 (1988)
- 32 general-purpose 32-bit registers (R0 hardwired 0)
- Load-delay slots, branch-delay slots
- MIPS I ISA: R/I/J formats
- Multiply/divide in HI/LO registers
- CP0 for TLB/exception handling (minimal)

### 07t — CDC 6600 (1964)
- 60-bit words; central memory 131,072 words
- 8 functional units, 10 peripheral processors
- Scoreboarding for out-of-order execution
- 3-bit register fields; X0–X7, A0–A7, B0–B7

### 07u — PowerPC 601 (1992)
- 32 general-purpose 32-bit registers (GPR0–31)
- 32 floating-point 64-bit registers (FPR0–31)
- Link register (LR), count register (CTR)
- Branch prediction; condition register CR (8×4-bit fields)
- Big-endian; memory-coherent; POWER heritage instructions

### 07v — x86-64 / AMD64 (2003)
- 16 general-purpose 64-bit registers (RAX/RBX/… + R8–R15)
- REX prefix for register extension
- RIP-relative addressing; 64-bit immediate support
- SSE2 baseline; XMM0–XMM15
- Compatibility mode (32-bit/16-bit in 64-bit OS)

### 07w — ARMv7-A / Thumb-2 (2004)
- 16 registers R0–R15 (R13=SP, R14=LR, R15=PC)
- CPSR/SPSR condition flags; conditional execution on every instruction
- Thumb-2 16/32-bit mixed encoding
- VFPv3 floating point; NEON SIMD
- 7 processor modes; MMU; coprocessor interface

### 07x — AArch64 / ARMv8-A (2011)
- 31 general-purpose 64-bit registers (X0–X30) + XZR/SP
- Clean orthogonal ISA; no conditional execution (except B.cond)
- NEON/Advanced SIMD V0–V31; SVE optional
- EL0–EL3 exception levels; system registers via MSR/MRS
- 48-bit virtual address space

### 07y — RISC-V (RV32I / RV64I, 2010)
- 32 general-purpose registers (x0 hardwired 0)
- 47 base integer instructions (RV32I); clean, minimal encoding
- Standard extensions: M (multiply), A (atomic), F/D (float), C (compressed)
- No delay slots; no branch-delay; no architecture-mandated endian
- Open specification; no IP licensing

### 07z — Apple M1 (AArch64 + Apple extensions, 2020)
- AArch64 baseline + AMX (Apple Matrix Extension)
- Unified memory architecture; no separate VRAM
- Firestorm (P-cores) + Icestorm (E-cores) heterogeneous
- Behavioral simulation of user-space AArch64 sufficient for compiler testing

---

## Package layout template

Each simulator follows the same layout:

```
code/packages/python/<name>-simulator/
├── BUILD
├── CHANGELOG.md
├── README.md
├── pyproject.toml
└── src/
    └── <name>_simulator/
        ├── __init__.py
        ├── py.typed
        ├── flags.py       (flag computation helpers)
        ├── state.py       (immutable State dataclass)
        └── simulator.py   (Simulator[State] implementation)
tests/
    ├── test_flags.py
    ├── test_protocol.py
    ├── test_load_store.py
    ├── test_arithmetic.py
    ├── test_logical.py
    ├── test_rotate_shift.py
    ├── test_bit_ops.py     (where applicable)
    ├── test_block_ops.py   (where applicable)
    ├── test_branch.py
    ├── test_interrupts.py
    └── test_programs.py
```

Each spec lives at `code/specs/<layer>-<name>-simulator.md`.

---

## Session continuity note

This roadmap is included in every PR so future sessions can pick up exactly where
the last session left off.  When resuming:

1. Check `code/specs/CPU-SIMULATOR-ROADMAP.md` for the next ⬜ item
2. Write the spec first (`code/specs/<layer>-<name>-simulator.md`)
3. Implement the package following the template above
4. Push to a PR; run `/babysit-pr` to monitor CI
5. Once merged, immediately start the next item without waiting for the user

---

*Last updated: 2026-05-04 — Z80 PR in progress*
