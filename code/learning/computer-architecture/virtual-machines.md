# Virtual Machines -- Bytecode Interpreters

## What is a Virtual Machine?

A virtual machine (VM) is a software program that simulates a computer. It
takes bytecode (a sequence of simple instructions) and executes it, step by
step, on whatever real hardware you happen to have.

```
                    +----------------------------+
                    |     Virtual Machine         |
                    |                            |
  Bytecode  ------->  Fetch -> Decode -> Execute |-------> Results
  (portable)        |                            |
                    |  Stack, Variables, PC       |
                    +----------------------------+
                    Runs on any real CPU (x86, ARM, RISC-V, ...)
```

This is exactly how Java works: you compile Java source to bytecode once,
and the JVM (Java Virtual Machine) runs it on Windows, Linux, macOS, or
any other platform. The bytecode is *portable* -- the VM handles the
platform-specific details.

Real-world VMs:
- **JVM** -- runs Java, Kotlin, Scala, Clojure
- **CLR** -- runs C#, F#, VB.NET
- **CPython** -- runs Python
- **V8** -- runs JavaScript (with JIT compilation)
- **WASM** -- runs WebAssembly in browsers
- **Our VM** -- runs our custom bytecode

---

## Stack-Based Execution Model

Our VM (and the JVM, CLR, CPython, and WASM) is a **stack-based** machine.
All computation happens through an operand stack:

```
Think of a stack of plates in a cafeteria:

    +-------+
    |   3   |  <-- top (most recently pushed)
    +-------+
    |   7   |
    +-------+
    |  42   |  <-- bottom (first pushed)
    +-------+

    PUSH:  Put a new plate on top
    POP:   Take the top plate off
    PEEK:  Look at the top plate without removing it
```

### The Three Fundamental Operations

**1. Push a value onto the stack:**
```
Before:  [1, 2]          After:  [1, 2, 3]
                PUSH 3
                ------>
```

**2. Pop a value off the stack:**
```
Before:  [1, 2, 3]       After:  [1, 2]     (3 is removed)
                POP
                ------>
```

**3. Operate on the top values:**
```
Before:  [1, 2, 3]       After:  [1, 5]     (2+3=5)
                ADD
                ------>
         pop 3 and 2, push 2+3=5
```

### Why Stack-Based?

Stack-based VMs are simpler to compile to. The compiler doesn't need to
worry about which registers to use (register allocation is one of the
hardest problems in compiler design). It just emits instructions that push
values and pop results.

The trade-off: stack-based code has more instructions than register-based
code (you need explicit pushes for every operand), but each instruction is
simpler and smaller.

---

## The Fetch-Decode-Execute Loop

Like every processor (real or virtual), our VM runs in a continuous loop:

```
                    +--------+
                    | FETCH  |  Read the instruction at PC
                    +---+----+
                        |
                        v
                    +--------+
                    | DECODE |  Look at the opcode to decide what to do
                    +---+----+
                        |
                        v
                    +---------+
                    | EXECUTE |  Perform the operation (push, pop, add, ...)
                    +---+-----+
                        |
                        v
                    +---------+
                    | ADVANCE |  Move PC to the next instruction
                    +---+-----+  (unless we jumped)
                        |
                        +-----> back to FETCH
```

This is the *exact same cycle* that real CPUs use (see
`code/packages/python/cpu-simulator/`), just implemented in software instead
of silicon.

### Fetch

Read the instruction at the current program counter (PC):

```
    instructions = [LOAD_CONST 0, LOAD_CONST 1, ADD, HALT]
                    ^
                    PC = 0

    Fetch: instruction = instructions[0] = LOAD_CONST 0
```

### Decode

Examine the opcode to determine what to do:

```
    instruction.opcode = LOAD_CONST (0x01)
    instruction.operand = 0

    This means: push constants[0] onto the stack
```

### Execute

Perform the operation:

```
    constants[0] = 1
    Push 1 onto the stack
    Stack: [] --> [1]
```

### Advance

Move PC to the next instruction:

```
    PC = 0 --> PC = 1
    (unless this was a JUMP instruction, which sets PC directly)
```

### Comparison: Our VM vs a Real CPU

```
                Our VM (software)              Real CPU (hardware)
                =================              ===================
Fetch:          Read from Python list          Read from memory bus
Decode:         Python if/elif chain           Transistor logic gates
Execute:        Python operations              ALU, register writes
Advance:        Increment integer PC           Increment hardware PC
Clock:          Python while loop              Crystal oscillator
Stack:          Python list                    Hardware stack pointer
```

The concepts are identical -- only the implementation medium differs.

---

## Call Frames -- How Function Calls Work

When the VM encounters a CALL instruction, it needs to save its current
state so it can resume later when the function RETURNs.

### The Call Stack

Each function call creates a **CallFrame** that saves:
- The return address (which instruction to resume at)
- The caller's local variables
- The caller's stack state

```
Call stack during nested calls:

    main() calls foo() calls bar():

    +-------------------+
    | CallFrame: bar()  |  <-- current
    | return_addr = 15  |
    | locals = [...]    |
    +-------------------+
    | CallFrame: foo()  |
    | return_addr = 8   |
    | locals = [...]    |
    +-------------------+
    | CallFrame: main() |
    | return_addr = 0   |
    | locals = [...]    |
    +-------------------+
```

### CALL Instruction

```
1. Save current state:
   - Push a CallFrame with the current PC+1 (return address),
     current variables, and current locals

2. Look up the function:
   - Find the function's CodeObject in the variables dictionary

3. Switch context:
   - Set PC to 0 (start of the function's code)
   - Set up fresh locals for the function
```

### RETURN Instruction

```
1. Pop the top CallFrame from the call stack

2. Restore state:
   - Set PC to the saved return address
   - Restore the caller's variables and locals

3. If there's a value on the current stack, push it onto the
   caller's stack (this is the return value)
```

This is exactly how real CPUs handle function calls with their hardware call
stack -- the `CALL` instruction pushes a return address, and `RET` pops it.
Our CallFrame is a richer version that also saves variable state.

---

## Local Variables vs Stack Slots

The VM supports two kinds of variable storage:

### Named Variables (Global Scope)

Stored in a dictionary, accessed by name. Used for global/module-level
variables.

```
STORE_NAME 0     # names[0] = "x"  -->  variables["x"] = top_of_stack
LOAD_NAME 0      # names[0] = "x"  -->  push variables["x"]
```

Dictionary lookup by string key. Flexible but slower due to hashing.

### Local Variable Slots (Function Scope)

Stored in a flat array, accessed by index. Used for function-local variables.

```
STORE_LOCAL 0    # locals[0] = top_of_stack
LOAD_LOCAL 0     # push locals[0]
```

Array indexing by integer. Fast -- O(1) with minimal overhead. This is why
real VMs (JVM, CPython) use numbered local slots inside functions.

### Why Two Systems?

```
Global variables:                Local variables:
    variables["x"] = 42             locals[0] = 42
    variables["y"] = 7              locals[1] = 7

    - Flexible (any name)           - Fast (array index)
    - Slower (dict lookup)          - Fixed size
    - Good for global scope         - Good for function scope
```

The JVM does exactly this: global/static fields are accessed by name (via
the constant pool), while local variables use numbered slots (iload_0,
istore_1, etc.).

---

## Walk Through: Executing `1 + 2 * 3`

Let's trace the complete execution of `x = 1 + 2 * 3` from compiled
bytecode.

### The CodeObject

```
instructions = [
    0: LOAD_CONST 0      # Push constants[0] = 1
    1: LOAD_CONST 1      # Push constants[1] = 2
    2: LOAD_CONST 2      # Push constants[2] = 3
    3: MUL               # 2 * 3 = 6
    4: ADD               # 1 + 6 = 7
    5: STORE_NAME 0      # x = 7
    6: HALT
]
constants = [1, 2, 3]
names = ["x"]
```

### Execution Trace

```
Step  PC  Instruction    Stack Before    Stack After    Variables
====  ==  =============  ==============  =============  =========
  1    0  LOAD_CONST 0   []              [1]            {}
  2    1  LOAD_CONST 1   [1]             [1, 2]         {}
  3    2  LOAD_CONST 2   [1, 2]          [1, 2, 3]      {}
  4    3  MUL            [1, 2, 3]       [1, 6]         {}
  5    4  ADD            [1, 6]          [7]            {}
  6    5  STORE_NAME 0   [7]             []             {"x": 7}
  7    6  HALT           []              []             {"x": 7}
```

### Step-by-Step Stack Visualization

```
Step 1: LOAD_CONST 0 (push constants[0] = 1)

    Stack:
    +---+
    | 1 |
    +---+

Step 2: LOAD_CONST 1 (push constants[1] = 2)

    Stack:
    +---+
    | 2 |  <-- top
    +---+
    | 1 |
    +---+

Step 3: LOAD_CONST 2 (push constants[2] = 3)

    Stack:
    +---+
    | 3 |  <-- top
    +---+
    | 2 |
    +---+
    | 1 |
    +---+

Step 4: MUL (pop 3 and 2, push 2*3 = 6)

    Pop b=3, pop a=2, push a*b=6

    Stack:
    +---+
    | 6 |  <-- top (2*3)
    +---+
    | 1 |
    +---+

Step 5: ADD (pop 6 and 1, push 1+6 = 7)

    Pop b=6, pop a=1, push a+b=7

    Stack:
    +---+
    | 7 |  <-- top (1+6)
    +---+

Step 6: STORE_NAME 0 (pop 7, store in names[0] = "x")

    Pop 7, store variables["x"] = 7

    Stack: (empty)
    Variables: {"x": 7}

Step 7: HALT

    Execution stops.
    Final state: x = 7
```

Notice how the stack naturally handles the order of operations. The
multiplication result (6) is computed first and sits on the stack waiting
for the addition. The compiler arranged the instructions so that the stack
does the right thing automatically.

---

## Comparison: Our VM vs JVM vs CLR vs WASM

### Architecture Overview

```
              Our VM         JVM            CLR            WASM
              ======         ===            ===            ====
Type:         Stack          Stack          Stack          Stack
Typing:       Dynamic        Static typed   Type inferred  Static typed
Opcodes:      ~20            ~200           ~230           ~450
Encoding:     High-level     Byte-level     Byte-level     Byte-level
Variables:    Dict + slots   Slots only     Slots only     Slots only
Const pool:   List           Rich table     Embedded       Embedded
Control:      JUMP/BRANCH    goto/if_*      br/brtrue      block/loop
Functions:    CALL/RETURN    invoke*/return call/ret       call/return
Halting:      HALT           (method ends)  ret            end
```

### How Each VM Pushes the Number 42

```
Our VM:    LOAD_CONST 0     (operand is pool index)
JVM:       bipush 42        (operand is the value itself, signed byte)
CLR:       ldc.i4.s 42      (operand is the value, short form)
WASM:      i32.const 42     (operand is 4-byte little-endian int32)
```

### How Each VM Adds Two Numbers

```
Our VM:    ADD              (untyped -- works on whatever is on stack)
JVM:       iadd             (typed -- "i" means int32)
CLR:       add              (type inferred from what's on the stack)
WASM:      i32.add          (typed -- "i32" means int32)
```

### How Each VM Stores to a Variable

```
Our VM:    STORE_NAME 0     (dictionary-based, by name index)
JVM:       istore_0         (numbered local slot)
CLR:       stloc.0          (numbered local slot)
WASM:      local.set 0      (numbered local slot)
```

### Key Differences

**Typing:** Our VM is dynamically typed -- ADD works on whatever values are
on the stack. The JVM and WASM are statically typed at the bytecode level --
they have separate opcodes for integer add, float add, etc. The CLR is in
between -- it has one `add` opcode but tracks types on the evaluation stack.

**Encoding:** Our VM uses high-level Python objects for instructions. Real
VMs encode instructions as raw bytes, with variable-width encoding (some
instructions are 1 byte, others are 2-5 bytes).

**Control flow:** WASM uses *structured* control flow (block/loop/if) instead
of arbitrary jumps. This makes it easier to validate and prevents certain
security exploits.

---

## The VM's Complete Instruction Set

Here is every opcode our VM supports, grouped by category:

### Stack Operations (0x0_)

```
LOAD_CONST operand   Push constants[operand] onto the stack
POP                  Discard the top value
DUP                  Duplicate the top value
```

### Variable Operations (0x1_)

```
STORE_NAME operand   Pop top, store in variables[names[operand]]
LOAD_NAME operand    Push variables[names[operand]]
STORE_LOCAL operand  Pop top, store in locals[operand]
LOAD_LOCAL operand   Push locals[operand]
```

### Arithmetic (0x2_)

```
ADD    Pop b, pop a, push a + b  (also concatenates strings)
SUB    Pop b, pop a, push a - b
MUL    Pop b, pop a, push a * b
DIV    Pop b, pop a, push a // b (integer division)
```

### Comparison (0x3_)

```
CMP_EQ   Pop b, pop a, push 1 if a == b, else 0
CMP_LT   Pop b, pop a, push 1 if a < b, else 0
CMP_GT   Pop b, pop a, push 1 if a > b, else 0
```

### Control Flow (0x4_)

```
JUMP operand            Set PC to operand (unconditional)
JUMP_IF_FALSE operand   Pop top; if falsy, set PC to operand
JUMP_IF_TRUE operand    Pop top; if truthy, set PC to operand
```

### Function Operations (0x5_)

```
CALL operand    Save state, jump to function named by names[operand]
RETURN          Restore state from call stack, resume caller
```

### I/O (0x6_)

```
PRINT    Pop top, add its string representation to the output list
```

### VM Control (0xF_)

```
HALT     Stop execution immediately
```

---

## Runtime Errors

The VM detects several error conditions:

```
StackUnderflowError    Tried to pop from an empty stack
                       (usually means buggy bytecode, not user error)

UndefinedNameError     Tried to LOAD_NAME for a variable that doesn't exist
                       (like Python's NameError)

DivisionByZeroError    Tried to DIV when the divisor is 0
                       (like Python's ZeroDivisionError)

InvalidOpcodeError     Encountered an opcode the VM doesn't recognize
                       (corrupted or incompatible bytecode)

InvalidOperandError    Operand is out of bounds
                       (e.g., LOAD_CONST 99 when constants has 3 entries)
```

---

## References

| File | Description |
|------|-------------|
| `code/packages/python/virtual-machine/src/virtual_machine/vm.py` | The complete VM implementation |
| `code/packages/python/bytecode-compiler/src/bytecode_compiler/compiler.py` | Compiler that produces CodeObjects |
| `code/packages/python/bytecode-compiler/src/bytecode_compiler/jvm_compiler.py` | JVM bytecode compiler |
| `code/packages/python/bytecode-compiler/src/bytecode_compiler/clr_compiler.py` | CLR IL compiler |
| `code/packages/python/bytecode-compiler/src/bytecode_compiler/wasm_compiler.py` | WASM bytecode compiler |
