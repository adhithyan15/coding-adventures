# ISA Simulators -- Instruction Set Architectures from ARM to WASM

## What is an ISA?

An **Instruction Set Architecture** (ISA) is the contract between software
and hardware. It defines:

- What instructions the processor understands
- How those instructions are encoded in binary
- What registers are available
- How memory is addressed
- How the processor behaves for each instruction

Think of the ISA as a "programming language for hardware." Just as Python
defines what `print("hello")` means, the ARM ISA defines what the binary
pattern `0xE2800001` means (it's `ADD R0, R0, #1` -- add 1 to register R0).

```
                 ISA
                 ===
                  |
    +-------------+-------------+
    |                           |
Software side:              Hardware side:
  Compilers produce           Silicon implements
  instructions that           circuits that execute
  conform to the ISA          those instructions
```

The ISA is what lets you swap out the hardware (upgrade your CPU) without
recompiling your software, or swap out the compiler without changing your CPU.

### The Repo's ISA Simulators

This repo simulates six different instruction set architectures:

```
code/packages/python/
    arm-simulator/         -- ARM (1985, phones and tablets)
    riscv-simulator/       -- RISC-V (2010, open-source)
    jvm-simulator/         -- JVM (1995, Java bytecode)
    clr-simulator/         -- CLR/IL (2002, .NET bytecode)
    wasm-simulator/        -- WebAssembly (2017, browser)
    intel4004-simulator/   -- Intel 4004 (1971, first microprocessor)
```

Each simulator reads binary-encoded instructions and executes them step by
step, maintaining registers (or a stack), memory, and a program counter --
just like real hardware would.

---

## RISC vs CISC Design Philosophy

ISAs fall into two major camps:

### RISC (Reduced Instruction Set Computer)

Philosophy: **Simple instructions, executed fast.**

- Each instruction does one small thing
- All instructions are the same size (fixed-width encoding)
- Only LOAD and STORE touch memory; arithmetic uses registers only
- Many general-purpose registers

Examples: ARM, RISC-V, MIPS, PowerPC

```
RISC approach to "increment memory[addr]":

    LOAD  R1, [addr]     # 1. Load value from memory into register
    ADD   R1, R1, #1     # 2. Add 1 to the register
    STORE R1, [addr]     # 3. Store back to memory

    Three simple instructions, each one cycle.
```

### CISC (Complex Instruction Set Computer)

Philosophy: **Powerful instructions, closer to high-level languages.**

- Some instructions do multiple things (load + operate + store)
- Variable-width encoding (1 to 15+ bytes per instruction)
- Instructions can directly operate on memory
- Fewer registers (historically)

Examples: x86, x86-64, VAX

```
CISC approach to "increment memory[addr]":

    INC [addr]           # One instruction does load + add + store

    One complex instruction, multiple cycles.
```

### Where the Repo's Architectures Fall

```
Pure RISC:         ARM, RISC-V
Stack machines:    JVM, CLR, WASM, Our VM
Accumulator:       Intel 4004
```

Stack machines are a special case -- they don't fit neatly into RISC/CISC
because they use an implicit stack instead of named registers. They are
conceptually simple (all operands come from the stack) but their instructions
are variable-width (like CISC).

---

## ARM -- Register-Register Architecture

**Location:** `code/packages/python/arm-simulator/`

### History

ARM (originally Acorn RISC Machine) was designed in 1985 by Sophie Wilson
and Steve Furber at Acorn Computers in Cambridge, England. ARM's big insight
was **power efficiency** -- while Intel focused on raw speed, ARM optimized
for low power consumption.

This bet paid off spectacularly. Today ARM processors are in virtually every
smartphone, tablet, and embedded device. Apple's M-series chips are ARM.

### Key Features

**16 registers** (R0-R15), each 32 bits wide:

```
R0-R3    Function arguments and return values
R4-R11   General purpose (callee-saved)
R12      IP (intra-procedure scratch register)
R13      SP (stack pointer)
R14      LR (link register -- return address)
R15      PC (program counter -- visible as a register!)
```

**Condition codes on every instruction** -- ARM's most distinctive feature.
Every instruction has a 4-bit condition field, so you can write:

```
CMP  R0, R1           ; compare R0 and R1, set flags
ADDGT R2, R0, R1      ; add ONLY IF R0 > R1 (Greater Than)
SUBLE R2, R0, R1      ; sub ONLY IF R0 <= R1 (Less or Equal)
```

This reduces branches, which is good for pipeline performance.

### Instruction Encoding

Every ARM instruction is exactly 32 bits:

```
Data processing format:
+------+----+---+--------+---+------+------+--------------+
| cond | 00 | I | opcode | S |  Rn  |  Rd  |   operand2   |
| 4bit |    |1b |  4bit  |1b | 4bit | 4bit |    12bit     |
+------+----+---+--------+---+------+------+--------------+
 31  28  27 26  25 24   21 20  19 16  15 12  11           0

cond:     Condition (0b1110 = AL = always execute)
I:        Immediate flag (1 = operand2 is immediate value)
opcode:   Operation (MOV=0b1101, ADD=0b0100, SUB=0b0010)
S:        Set condition flags
Rn:       First source register
Rd:       Destination register
operand2: Register or rotated 8-bit immediate
```

### Example: `x = 1 + 2` on ARM

```
MOV R0, #1         ; R0 = 1          (load immediate)
MOV R1, #2         ; R1 = 2          (load immediate)
ADD R2, R0, R1     ; R2 = R0 + R1    (register-register add)
HLT                ; halt

Execution trace:
    Step 1: R0 = 1
    Step 2: R1 = 2
    Step 3: R2 = 1 + 2 = 3
    Step 4: halt, result in R2
```

---

## RISC-V -- Clean, Modern, Open-Source ISA

**Location:** `code/packages/python/riscv-simulator/`

### History

RISC-V (pronounced "risk-five") was designed at UC Berkeley in 2010 by
Patterson and Hennessy -- the same people who wrote the definitive computer
architecture textbooks. It was designed from scratch with no historical
baggage, making it the cleanest ISA to learn.

Unlike ARM (which is commercially licensed), RISC-V is **open-source**. Anyone
can build a RISC-V processor without paying royalties. This has made it
hugely popular in education and increasingly in industry.

### Key Features

**32 registers** (x0-x31), each 32 bits wide:

```
x0       Always 0 (hardwired -- writes are ignored)
x1       ra (return address)
x2       sp (stack pointer)
x3       gp (global pointer)
x4       tp (thread pointer)
x5-x7    t0-t2 (temporaries)
x8-x9    s0-s1 (saved registers)
x10-x17  a0-a7 (function arguments and return values)
x18-x27  s2-s11 (more saved registers)
x28-x31  t3-t6 (more temporaries)
```

**The x0 register is special and brilliant.** Because it always reads as 0,
many operations become simpler:

```
Load an immediate:   addi x1, x0, 42    -->  x1 = 0 + 42 = 42
Move a register:     addi x2, x1, 0     -->  x2 = x1 + 0 = x1
Compare to zero:     beq  x1, x0, label -->  branch if x1 == 0
No-op:               addi x0, x0, 0     -->  does nothing
```

### Instruction Encoding

Every RISC-V instruction is exactly 32 bits, with highly regular field
positions:

```
R-type (register-register):
+----------+------+------+--------+------+----------+
|  funct7  |  rs2 |  rs1 | funct3 |  rd  |  opcode  |
|  7 bits  | 5bit | 5bit |  3bit  | 5bit |  7 bits  |
+----------+------+------+--------+------+----------+
  31    25  24  20 19  15  14   12  11   7  6       0

I-type (immediate):
+---------------+------+--------+------+----------+
|   imm[11:0]   |  rs1 | funct3 |  rd  |  opcode  |
|    12 bits    | 5bit |  3bit  | 5bit |  7 bits  |
+---------------+------+--------+------+----------+
  31          20 19  15  14   12  11   7  6       0
```

The regularity is deliberate -- register fields are always in the same
positions, which makes the hardware decoder simpler than ARM's.

### Example: `x = 1 + 2` on RISC-V

```
addi x1, x0, 1    ; x1 = 0 + 1 = 1     (I-type)
addi x2, x0, 2    ; x2 = 0 + 2 = 2     (I-type)
add  x3, x1, x2   ; x3 = 1 + 2 = 3     (R-type)
ecall              ; halt                 (system call)

Execution trace:
    Step 1: x1 = 0 + 1 = 1  (x0 is always 0)
    Step 2: x2 = 0 + 2 = 2
    Step 3: x3 = 1 + 2 = 3
    Step 4: halt, result in x3
```

### ARM vs RISC-V

```
                    ARM                     RISC-V
                    ===                     ======
Registers:          16 (R0-R15)            32 (x0-x31)
Zero register:      None                    x0 (hardwired 0)
Condition codes:    On every instruction    Separate branch instructions
Encoding:           Complex (many formats)  Regular (few formats)
License:            Commercial              Open-source
Designed:           1985                    2010
```

---

## JVM -- Stack-Based, Typed Opcodes

**Location:** `code/packages/python/jvm-simulator/`

### History

The Java Virtual Machine was introduced by Sun Microsystems in 1995. Its
revolutionary promise: "write once, run anywhere." Compile to JVM bytecode,
and any machine with a JVM can run it.

Today the JVM is the most widely deployed virtual machine in history, running
not just Java but also Kotlin, Scala, Clojure, Groovy, and JRuby.

### Key Features

**Stack-based with typed opcodes.** Unlike our VM (which has one untyped ADD),
the JVM has separate opcodes for each type:

```
Our VM:    ADD              (works on whatever is on the stack)
JVM:       iadd             (integer add)
           ladd             (long add)
           fadd             (float add)
           dadd             (double add)
```

This enables the JVM's **bytecode verifier** to check type safety before any
code executes.

**Variable-width encoding.** Instructions range from 1 to 5+ bytes:

```
iconst_1       0x04                    1 byte  (push 1)
bipush 42      0x10 0x2A              2 bytes (push 42)
ldc #3         0x12 0x03              2 bytes (load from constant pool)
iadd           0x60                    1 byte
istore_0       0x3B                    1 byte  (store to local 0)
goto +5        0xA7 0x00 0x05         3 bytes (jump forward 5 bytes)
```

**Short-form instructions** for common values. `iconst_0` through `iconst_5`
are single-byte opcodes for pushing small constants. `istore_0` through
`istore_3` are single-byte opcodes for the first four local variables.

### The Constant Pool

The JVM's constant pool is richer than ours. It stores not just numbers and
strings, but also class names, method signatures, field references, and more.
Each entry has a tag byte identifying its type. Instructions like `ldc`
reference entries by index.

### Example: `x = 1 + 2` on the JVM

```
iconst_1         ; push 1          stack: [1]
iconst_2         ; push 2          stack: [1, 2]
iadd             ; int add         stack: [3]
istore_0         ; store local 0   stack: [], x=3
return           ; end method
```

---

## CLR/IL -- Similar to JVM but for .NET

**Location:** `code/packages/python/clr-simulator/`

### History

The Common Language Runtime (CLR) is Microsoft's answer to the JVM, released
in 2002 with .NET 1.0. C#, F#, VB.NET, and PowerShell all compile to Common
Intermediate Language (CIL), which the CLR executes.

### Key Difference from JVM: Type Inference

The CLR uses **type-inferred** opcodes where the JVM uses typed opcodes:

```
JVM:    iconst_1        "i" means this pushes an int32
        iconst_2
        iadd            "i" means int32 addition

CLR:    ldc.i4.1        push int32 constant 1
        ldc.i4.2        push int32 constant 2
        add             type inferred from the stack!
```

The CLR's `add` instruction works for int32, int64, float, or double -- it
figures out the type from what's on the evaluation stack. This means fewer
opcodes but the runtime must track stack types.

### More Short Forms

The CLR has short forms for constants 0 through 8 (JVM only goes to 5):

```
JVM:    iconst_0 ... iconst_5     (6 shortcuts)
CLR:    ldc.i4.0 ... ldc.i4.8     (9 shortcuts)
```

### Two-Byte Opcodes (0xFE Prefix)

The CLR has more than 256 instructions. It uses a prefix byte (0xFE) to
create a second "page" of opcodes:

```
ceq  = 0xFE 0x01    compare equal
cgt  = 0xFE 0x02    compare greater than
clt  = 0xFE 0x04    compare less than
```

### Example: `x = 1 + 2` on the CLR

```
ldc.i4.1         ; push 1          stack: [1]
ldc.i4.2         ; push 2          stack: [1, 2]
add              ; add             stack: [3]
stloc.0          ; store local 0   stack: [], x=3
ret              ; return
```

---

## WebAssembly -- Portable Stack Machine

**Location:** `code/packages/python/wasm-simulator/`

### History

WebAssembly (WASM) was standardized by the W3C in 2017. Unlike the JVM and
CLR, which target general-purpose computing, WASM was designed specifically
for the web -- a compact, fast, safe bytecode that runs in browsers alongside
JavaScript.

Languages like Rust, C++, Go, and AssemblyScript compile to WASM, letting
you run near-native-speed code in a browser sandbox.

### Key Design Choices

**Structured control flow.** WASM doesn't have `goto`. Instead, it uses
structured blocks, loops, and if/else constructs. This makes bytecode easier
to validate and prevents entire classes of security exploits.

**Module-based sandboxing.** WASM code lives in modules with explicit imports
and exports. There is no global mutable state accessible from outside. This
is crucial for running untrusted code in browsers.

**Uniform encoding.** Where the JVM has `iconst_0` through `iconst_5` (saving
bytes for small values), WASM always uses `i32.const` followed by a full
4-byte value. Simpler to encode/decode, but larger bytecode.

### Variable-Width Instructions

```
i32.const 1      0x41 0x01 0x00 0x00 0x00    5 bytes
i32.add          0x6A                         1 byte
local.get 0      0x20 0x00                    2 bytes
local.set 0      0x21 0x00                    2 bytes
end              0x0B                          1 byte
```

### Example: `x = 1 + 2` on WASM

```
i32.const 1      ; push 1          stack: [1]
i32.const 2      ; push 2          stack: [1, 2]
i32.add          ; add             stack: [3]
local.set 0      ; store local 0   stack: [], x=3
end              ; halt
```

---

## Intel 4004 -- The World's First Microprocessor (1971)

**Location:** `code/packages/python/intel4004-simulator/`

### History

The Intel 4004 was the world's first commercial single-chip microprocessor,
released in 1971. Designed by Federico Faggin, Ted Hoff, and Stanley Mazor
for the Busicom 141-PF calculator, it contained just **2,300 transistors**
(a modern CPU has billions) and ran at **740 kHz** (about a million times
slower than today).

### Why 4-Bit?

The 4004 is a **4-bit** processor. Every data value is 4 bits wide (0-15).
This seems tiny, but it was perfect for calculators. A single decimal digit
(0-9) fits in 4 bits, which is exactly what BCD (Binary-Coded Decimal)
arithmetic needs.

All values in the simulator are masked to 4 bits (`& 0xF`).

### Accumulator Architecture

The 4004 uses an **accumulator architecture**. Almost every arithmetic
operation works through a single special register called the **Accumulator (A)**:

```
Modern (RISC-V):    ADD x3, x1, x2     Any register to any register
Stack (WASM):       i32.add              Pops two from stack
Accumulator (4004): ADD R0               A = A + R0. Always uses A.
```

The accumulator pattern means more instructions for the same work, but
simpler hardware -- which mattered enormously when every transistor was
precious.

### Registers

```
Accumulator (A):   4 bits. The center of all computation.
R0-R15:            16 general registers, each 4 bits.
Carry flag:        1 bit. Set on arithmetic overflow/borrow.
PC:                Program counter.
```

### Instruction Encoding

Instructions are 8 bits (1 byte). The upper nibble is the opcode, the lower
nibble is the operand:

```
+----------+----------+
|  opcode  | operand  |
| bits 7-4 | bits 3-0 |
+----------+----------+

LDM N    (0xDN):  Load immediate N into accumulator. A = N.
XCH Rn   (0xBN):  Exchange A with register N. Swap A and Rn.
ADD Rn   (0x8N):  Add register N to A. A = A + Rn.
SUB Rn   (0x9N):  Subtract register N from A. A = A - Rn.
HLT      (0x01):  Halt execution (simulator-only opcode).
```

### Example: `x = 1 + 2` on the Intel 4004

Because the 4004 uses an accumulator, you need more steps:

```
LDM 1      ; A = 1                      (load immediate)
XCH R0     ; R0 = 1, A = 0              (save to register)
LDM 2      ; A = 2                      (load immediate)
ADD R0     ; A = 2 + 1 = 3              (add register to accumulator)
XCH R1     ; R1 = 3, A = 0              (store result)
HLT        ; halt

Binary encoding:
    0xD1  0xB0  0xD2  0x80  0xB1  0x01

Execution trace:
    Step 1: A=1
    Step 2: R0=1, A=0
    Step 3: A=2
    Step 4: A=2+1=3
    Step 5: R1=3, A=0
    Step 6: halt, result in R1
```

Compare this to RISC-V's 3 instructions or WASM's 4 instructions. The
accumulator architecture requires more data movement because only one
register (A) can participate in arithmetic.

---

## Why Simulating Old Architectures Teaches Modern Concepts

### 1. The Same Principles Appear Everywhere

Every architecture, from the 1971 Intel 4004 to modern ARM chips, uses
the same fundamental cycle: fetch, decode, execute.

```
                    Intel 4004        RISC-V          JVM
                    (1971)            (2010)          (1995)
                    ==========        ======          ===
Fetch:              Read 1 byte       Read 4 bytes    Read 1+ bytes
Decode:             Upper nibble      opcode field    Opcode byte
Execute:            4-bit ALU         32-bit ALU      Stack ops
Advance:            PC += 1           PC += 4         PC += width
```

### 2. Trade-offs are Timeless

The design trade-offs from 1971 still apply today:

```
More registers      vs   Simpler encoding
Fixed-width insns   vs   Compact variable-width
Stack machine       vs   Register machine
Typed opcodes       vs   Dynamic typing
```

Understanding the 4004's accumulator architecture helps you appreciate *why*
RISC-V has 32 general-purpose registers. Understanding the JVM's typed
opcodes helps you appreciate *why* WebAssembly made the same choice 22 years
later.

### 3. Abstraction Layers Repeat

The pattern of "define an abstract machine, compile to it, interpret it"
appears at every level:

```
High-level:    Python source  --> CPython bytecode  --> CPython VM
Medium-level:  Java source    --> JVM bytecode      --> JVM
Low-level:     C source       --> x86 machine code  --> x86 CPU
Our stack:     Our source     --> Our bytecode       --> Our VM
                               --> ARM binary        --> ARM simulator
                               --> RISC-V binary     --> RISC-V simulator
                               --> JVM bytecode      --> JVM simulator
                               --> WASM bytecode     --> WASM simulator
```

---

## Comparison Table: All Six Architectures

```
                4004       ARM        RISC-V     JVM        CLR        WASM
                ====       ===        ======     ===        ===        ====
Year:           1971       1985       2010       1995       2002       2017
Type:           Accum.     Register   Register   Stack      Stack      Stack
Data width:     4-bit      32-bit     32-bit     32/64-bit  32/64-bit  32/64-bit
Registers:      A + 16     16         32         (stack)    (stack)    (stack)
Zero reg:       No         No         x0         N/A        N/A        N/A
Insn width:     8-bit      32-bit     32-bit     Variable   Variable   Variable
Encoding:       Fixed      Fixed      Fixed      Variable   Variable   Variable
Cond. exec:     No         Yes        No         No         No         No
Open source:    N/A        No         Yes        Spec only  Spec only  Yes
Typing:         Untyped    Untyped    Untyped    Typed      Inferred   Typed
```

### How Each Architecture Computes `1 + 2`

```
Intel 4004 (accumulator, 6 instructions):
    LDM 1 / XCH R0 / LDM 2 / ADD R0 / XCH R1 / HLT

ARM (register-register, 4 instructions):
    MOV R0, #1 / MOV R1, #2 / ADD R2, R0, R1 / HLT

RISC-V (register-register, 4 instructions):
    addi x1, x0, 1 / addi x2, x0, 2 / add x3, x1, x2 / ecall

JVM (stack, 5 instructions):
    iconst_1 / iconst_2 / iadd / istore_0 / return

CLR (stack, 5 instructions):
    ldc.i4.1 / ldc.i4.2 / add / stloc.0 / ret

WASM (stack, 5 instructions):
    i32.const 1 / i32.const 2 / i32.add / local.set 0 / end
```

Notice how register machines need fewer total instructions but wider
instructions (register numbers must be encoded). Stack machines need more
instructions but each one is simpler and narrower.

The accumulator architecture (4004) needs the most instructions because
every value must pass through the single accumulator register, requiring
explicit moves.

---

## References

| File | Description |
|------|-------------|
| `code/packages/python/arm-simulator/src/arm_simulator/simulator.py` | ARM simulator with condition codes |
| `code/packages/python/riscv-simulator/src/riscv_simulator/simulator.py` | RISC-V RV32I simulator |
| `code/packages/python/jvm-simulator/src/jvm_simulator/simulator.py` | JVM bytecode simulator |
| `code/packages/python/clr-simulator/src/clr_simulator/simulator.py` | CLR IL simulator |
| `code/packages/python/wasm-simulator/src/wasm_simulator/simulator.py` | WebAssembly simulator |
| `code/packages/python/intel4004-simulator/src/intel4004_simulator/simulator.py` | Intel 4004 simulator |
| `code/packages/python/cpu-simulator/` | Generic CPU framework used by ARM and RISC-V |
