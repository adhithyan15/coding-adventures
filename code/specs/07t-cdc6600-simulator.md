# Layer 07t — CDC 6600 (1964) Behavioral Simulator

## Overview

The Control Data Corporation 6600 (1964) was the world's **first supercomputer**,
designed by Seymour Cray at Control Data Corporation.  It ran at 40 MHz with a
60-bit word architecture and achieved roughly 10 MFLOPS — three times faster than
any contemporary machine.  It held the title of world's fastest computer until
1969 when the CDC 7600 (also by Cray) superseded it.

Historical significance:
- **First machine called a "supercomputer"** — the term was coined for it
- **Scoreboarding** — first use of out-of-order execution via a hardware scoreboard
- **10 Peripheral Processors (PPs)** — first use of satellite processors for I/O,
  freeing the Central Processor (CP) for pure arithmetic
- **3 register types** — X (operand), A (address), B (index) — an orthogonal design
  ahead of its time
- Seymour Cray's first major design; set the template for Cray supercomputers

Comparison with prior simulators in this series:

| Feature | Alpha AXP (07s) | CDC 6600 (07t) |
|---------|-----------------|----------------|
| Year    | 1992            | 1964           |
| Word width | 64 bits      | **60 bits**    |
| GPRs    | 32 × 64-bit     | 8 × 60-bit (X), 8 × 18-bit (A), 8 × 18-bit (B) |
| Instruction size | 32-bit fixed | **15-bit or 30-bit**, packed 4 per word |
| Condition codes | None | None (branches test register directly) |
| Endianness | Little | **Big** (MSB = bit 59 of each 60-bit word) |
| Memory words | 64-bit bytes | **60-bit words** |

---

## Architecture

### Register Set

#### X Registers (Operand Registers): X0–X7
- 8 registers, each **60 bits** wide
- Hold integer or floating-point operands
- X0 is a general-purpose register (unlike B0, it is NOT hardwired to zero)
- All arithmetic results go into an X register

#### A Registers (Address Registers): A0–A7
- 8 registers, each **18 bits** wide (addresses up to 262,143 words)
- Hold memory addresses
- In hardware, writing to A1–A5 automatically triggers a memory **read** into
  the corresponding X register; writing to A6–A7 triggers a memory **write** from
  the corresponding X register.  This simulator implements explicit load/store
  instructions that capture this semantics cleanly.
- A0 is general-purpose (no automatic memory side effect)

#### B Registers (Index/Increment Registers): B0–B7
- 8 registers, each **18 bits** wide
- Used for loop counters, array indices, and integer arithmetic
- **B0 is hardwired to 0** — reads always return 0; writes are ignored

### Program Counter (P Register)
- The CDC 6600 calls this the "P" register (for "Parcel")
- Points to a **parcel address** — word × 4 + parcel_index (0–3)
- 15-bit instruction advances P by 1; 30-bit instruction advances P by 2
- Branch target is a parcel address

### Memory
- Behavioral simulation uses **4096 sixty-bit words** (sufficient for test programs)
- Real CDC 6600: 131,072 sixty-bit words (1 M-word optional)
- Big-endian: bit 59 is the most-significant bit of each 60-bit word

### Instruction Packing

Each 60-bit memory word holds **four 15-bit parcels** (p0–p3):

```
Word bits: [59:45] = parcel 0 (p0, most significant)
           [44:30] = parcel 1 (p1)
           [29:15] = parcel 2 (p2)
           [14: 0] = parcel 3 (p3, least significant)
```

Instructions are either **15 bits** (one parcel) or **30 bits** (two parcels).

---

## Instruction Formats

### Format 1 — Short (15 bits)

```
Bits [14:9] = f   (6-bit opcode)
Bits [ 8:6] = i   (3-bit destination register index)
Bits [ 5:3] = j   (3-bit source register index, left operand)
Bits [ 2:0] = k   (3-bit source register index, right operand)
```

Used for register-to-register operations.  Examples:
- `Xi = Xj + Xk` (integer add)
- `Xi = Xj & Xk` (boolean AND)
- `Bi = Bj + Bk` (B-register add)

### Format 2 — Long (30 bits)

```
Bits [29:24] = f   (6-bit opcode)
Bits [23:21] = i   (3-bit destination register index)
Bits [20:18] = j   (3-bit source register index)
Bits [17: 0] = K   (18-bit constant, address, or branch target)
```

Used for load-immediate, memory access, and branches.

### HALT Instruction
- HALT is encoded as a **15-bit all-zeros parcel** (`0x0000`)
- When the simulator encounters parcel value 0x0000, execution stops
- An uninitialised memory region (all zeros) will halt immediately — convenient
  for test programs that fall off the end of their code

---

## Instruction Set (Subset Implemented)

Opcodes are given in decimal and octal (CDC documentation used octal).

### Format 1 — Register-to-Register (15 bits)

| f (dec) | f (oct) | Mnemonic | Operation |
|---------|---------|----------|-----------|
| 1  | 01 | TXB  | Xi = zero_extend60(Bj) |
| 2  | 02 | TBX  | Bi = Xj[17:0] |
| 3  | 03 | TAX  | Xi = zero_extend60(Aj) |
| 4  | 04 | TXA  | Ai = Xj[17:0] |
| 5  | 05 | IXPB | Xi = Xj + zero_extend60(Bk) (integer add X+B) |
| 6  | 06 | IXMB | Xi = Xj - zero_extend60(Bk) (integer subtract X-B) |
| 7  | 07 | IXXP | Xi = Xj + Xk (integer add X+X, 60-bit) |
| 8  | 10 | IXXM | Xi = Xj - Xk (integer subtract X-X, 60-bit) |
| 9  | 11 | BXND | Xi = Xj & Xk (boolean AND) |
| 10 | 12 | BXOR | Xi = Xj \| Xk (boolean OR) |
| 11 | 13 | BXXR | Xi = Xj ^ Xk (boolean XOR, "exclusive or") |
| 12 | 14 | BXMR | Xi = ~Xj (boolean complement; k ignored) |
| 13 | 15 | LSHL | Xi = Xj << (Bk & 63) (logical shift left) |
| 14 | 16 | LSHR | Xi = Xj >> (Bk & 63) (logical shift right, zero-fill) |
| 15 | 17 | IBBP | Bi = Bj + Bk (B-register integer add, 18-bit) |
| 16 | 20 | IBBM | Bi = Bj - Bk (B-register integer subtract, 18-bit) |
| 17 | 21 | IAAP | Ai = Aj + Bk (address register add, 18-bit) |
| 18 | 22 | IAAM | Ai = Aj - Bk (address register subtract, 18-bit) |
| 19 | 23 | CMPEQ | Bi = 1 if Xj == Xk else 0 (compare into B) |
| 20 | 24 | CMPLT | Bi = 1 if signed(Xj) < signed(Xk) else 0 |
| 21 | 25 | CMPGT | Bi = 1 if signed(Xj) > signed(Xk) else 0 |
| 22 | 26 | IXMUL | Xi = (Xj * Xk)[59:0] (lower 60 bits of integer multiply) |

### Format 2 — Long / Immediate (30 bits)

| f (dec) | f (oct) | Mnemonic | Operation |
|---------|---------|----------|-----------|
| 32 | 40 | LDXI | Xi = K (load 18-bit zero-extended constant into Xi) |
| 33 | 41 | LDBI | Bi = K (load 18-bit constant into Bi) |
| 34 | 42 | LDAI | Ai = K (load 18-bit constant into Ai) |
| 35 | 43 | LDX  | Xi = mem[Aj + K] (load Xi from memory at word address Aj+K) |
| 36 | 44 | STX  | mem[Ai + K] = Xj (store Xj to memory) |
| 37 | 45 | LDB  | Bi = mem[Aj + K][17:0] (load lower 18 bits of word into Bi) |
| 38 | 46 | STB  | mem[Ai + K][17:0] = Bj (store Bj into lower 18 bits of word) |
| 40 | 50 | JEQ  | if Bj == 0: P = K (conditional branch, jump if B zero) |
| 41 | 51 | JNE  | if Bj != 0: P = K |
| 42 | 52 | JXZ  | if Xj == 0: P = K (jump if X register zero) |
| 43 | 53 | JXN  | if Xj != 0: P = K |
| 44 | 54 | JMP  | P = K (unconditional branch to parcel address K) |
| 45 | 55 | JSR  | B7 = P+2; P = K (call: save return parcel addr in B7) |
| 46 | 56 | RET  | P = Bj (return: jump to parcel address in Bj) |

---

## Signed Integer Convention

The CDC 6600 uses **one's-complement** arithmetic for negative numbers (different
from the two's-complement used by all modern processors).  However, this simulator
uses **Python's arbitrary-precision integers** with explicit masking to 60 bits, and
interprets sign via bit 59 (the most-significant bit):

```
Positive: bit59 == 0, value in [0, 2^59 - 1]
Negative: bit59 == 1, value = -(~x & MASK60) in one's-complement terms
```

For simplicity, this simulator uses **two's-complement** semantics internally
(standard Python int), masked to 60 bits.  The behaviour of CMPLT/CMPGT uses
Python's `ctypes.c_int64` sign-extension truncated to 60 bits (bit 59 as sign).
Programs that don't rely on one's-complement edge cases will behave identically.

---

## SIM00 Protocol

```python
class CDC6600Simulator(Simulator[CDC6600State]):
    def reset(self) -> None: ...
    def load(self, program: bytes) -> None: ...
    def step(self) -> StepTrace: ...
    def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult: ...
    def get_state(self) -> CDC6600State: ...
```

`CDC6600State` is a `frozen=True` dataclass:

```python
@dataclass(frozen=True)
class CDC6600State:
    p:      int                 # parcel address (word*4 + parcel_index)
    x:      tuple[int, ...]     # 8 × 60-bit operand registers (X0–X7)
    a:      tuple[int, ...]     # 8 × 18-bit address registers (A0–A7)
    b:      tuple[int, ...]     # 8 × 18-bit index registers (B0–B7, B0==0)
    memory: tuple[int, ...]     # 4096 × 60-bit words
    halted: bool
```

Convenience properties on `CDC6600State`: `.x0`–`.x7`, `.a0`–`.a7`, `.b0`–`.b7`.

---

## Program Encoding Helpers

Programs are passed as `bytes`.  Each pair of bytes encodes one 15-bit parcel
(big-endian, high byte first, high-nibble padding):

```python
def parcel(f, i, j, k):
    """Encode a 15-bit short instruction."""
    return ((f & 0x3F) << 9 | (i & 7) << 6 | (j & 7) << 3 | (k & 7)).to_bytes(2, "big")

def long_instr(f, i, j, K):
    """Encode a 30-bit long instruction as 4 bytes."""
    word = (f & 0x3F) << 24 | (i & 7) << 21 | (j & 7) << 18 | (K & 0x3FFFF)
    return word.to_bytes(4, "big")

HALT = b"\x00\x00"   # 15-bit all-zeros parcel
```

The `load()` method packs parcels from the byte stream into 60-bit memory words
(4 parcels per word, big-endian).

---

## Package Layout

```
code/packages/python/cdc6600-simulator/
├── pyproject.toml
├── README.md
├── CHANGELOG.md
├── src/
│   └── cdc6600_simulator/
│       ├── __init__.py
│       ├── py.typed
│       ├── state.py        (CDC6600State, constants)
│       └── simulator.py    (CDC6600Simulator — instruction dispatch)
└── tests/
    ├── test_protocol.py
    ├── test_instructions.py
    ├── test_programs.py
    └── test_coverage.py
```

---

## Design Notes

### Why 60-bit Words?

Seymour Cray chose 60 bits as the word size to get the best balance of precision for
scientific computation (more than 32-bit IEEE float but achievable in discrete logic)
while avoiding 64 bits (which would have required more hardware).  The 60-bit word
gives 48 bits of mantissa for floating-point — far more precise than the 23-bit
single-precision float later standardized.

### Why 3 Register Types?

X registers hold large 60-bit quantities (floating-point and integer values).
A and B registers are smaller (18 bits) because address spaces in 1964 were
measured in thousands of words, not gigabytes.  Having separate narrow registers for
addresses and loop counters freed the X registers entirely for data — an early form
of architectural register classification.

### Scoreboarding (Hardware, Omitted Here)

The real CDC 6600 could execute up to 10 instructions in parallel across 8 functional
units.  The scoreboard tracked which functional units were in use and stalled only
when a true data dependency existed.  This behavioral simulator executes all
instructions sequentially — the functional units and scoreboard are invisible to
programs that do not rely on precise timing.

### Why One's-Complement? (And Why We Approximate)

One's-complement arithmetic was preferred in 1964 because it simplified hardware
design (no carry-propagation needed for the "end-around carry" trick).  It also means
there are two representations of zero (+0 and −0), which affects comparison semantics.
For this behavioral simulator, two's-complement is used throughout (standard Python
ints), and the single edge case (−0 vs +0) is irrelevant to instruction-set testing.

---

## Simplifications

1. **No floating-point** — X registers hold 60-bit integers; FP instructions omitted
2. **No peripheral processors (PPs)** — No I/O simulation
3. **Two's-complement integers** — Not true one's-complement (edge cases differ)
4. **4096-word memory** — Truncated from 131,072 words
5. **No scoreboarding** — Sequential execution only
6. **No exchange jump** — The CDC 6600's context-switch instruction is omitted
7. **B7 is the link register** — JSR saves return parcel address in B7 (convention
   used in this simulator; the hardware had no dedicated link register)

---

## Test Plan

| Test module | What it covers |
|-------------|----------------|
| test_protocol.py | SIM00 compliance: all 5 methods, return types |
| test_protocol.py | reset() zeros all state; P=0 |
| test_protocol.py | load() packs bytes into 60-bit words |
| test_protocol.py | execute() returns ExecutionResult |
| test_protocol.py | step() returns StepTrace with pc_before/pc_after |
| test_protocol.py | get_state() is frozen; step() does not mutate snapshot |
| test_instructions.py | TXB: Xi = Bj (B to X transmit) |
| test_instructions.py | TBX: Bi = Xj[17:0] (X to B transmit) |
| test_instructions.py | TAX: Xi = Aj (A to X transmit) |
| test_instructions.py | TXA: Ai = Xj[17:0] (X to A transmit) |
| test_instructions.py | IXPB: Xi = Xj + Bk (add B into X) |
| test_instructions.py | IXMB: Xi = Xj - Bk |
| test_instructions.py | IXXP: Xi = Xj + Xk (integer add X+X) |
| test_instructions.py | IXXM: Xi = Xj - Xk |
| test_instructions.py | BXND, BXOR, BXXR, BXMR (boolean ops) |
| test_instructions.py | LSHL, LSHR (shift by Bk) |
| test_instructions.py | IBBP, IBBM (B-register integer arithmetic) |
| test_instructions.py | IAAP, IAAM (A-register arithmetic) |
| test_instructions.py | CMPEQ, CMPLT, CMPGT (compare into B) |
| test_instructions.py | IXMUL: lower 60 bits of multiply |
| test_instructions.py | LDXI, LDBI, LDAI (load immediate) |
| test_instructions.py | LDX, STX (load/store X registers) |
| test_instructions.py | LDB, STB (load/store B registers via memory) |
| test_instructions.py | JEQ, JNE (branch on B register) |
| test_instructions.py | JXZ, JXN (branch on X register) |
| test_instructions.py | JMP (unconditional branch) |
| test_instructions.py | JSR/RET (subroutine call/return via B7) |
| test_coverage.py | B0 hardwired zero (writes ignored) |
| test_coverage.py | HALT on all-zeros parcel |
| test_coverage.py | Unknown opcode raises ValueError |
| test_coverage.py | max_steps guard terminates infinite loop |
| test_coverage.py | 60-bit mask: arithmetic stays within 60 bits |
| test_coverage.py | A/B register 18-bit mask |
| test_coverage.py | Memory bounds checking |
| test_programs.py | Sum 1–10 using IXXP + JNE loop |
| test_programs.py | Factorial 5! using IXMUL + JNE loop |
| test_programs.py | Fibonacci using LDX/STX + loop |
| test_programs.py | Subroutine call/return using JSR/RET |
| test_programs.py | Array sum using LDX + IAAP loop |
| test_programs.py | Boolean operations: XOR-based swap |
