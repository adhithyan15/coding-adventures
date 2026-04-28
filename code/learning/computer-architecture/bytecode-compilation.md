# Bytecode Compilation -- From Trees to Instructions

## What is Bytecode?

Bytecode is a compact, portable representation of a program as a sequence of
simple instructions. It sits between source code (human-readable) and machine
code (CPU-specific):

```
Source code       "x = 1 + 2"              (human-readable)
     |
     v
AST               Assignment(Name("x"),    (tree structure)
                     BinaryOp(1, "+", 2))
     |
     v
Bytecode          LOAD_CONST 0             (flat instruction sequence)
                  LOAD_CONST 1
                  ADD
                  STORE_NAME 0
                  HALT
     |
     v
Machine code      mov eax, 1               (CPU-specific)
                  add eax, 2
                  mov [x], eax
```

Real-world examples of bytecode:
- **javac** compiles Java source into JVM bytecode (`.class` files)
- **csc** compiles C# source into CLR IL (`.dll` files)
- **CPython** compiles Python source into Python bytecode (`.pyc` files)
- **Our compiler** compiles ASTs into `CodeObject` (for our VM)

### Why Compile to Bytecode Instead of Interpreting ASTs Directly?

You *could* walk the AST and evaluate it directly (a tree-walk interpreter).
Many scripting languages started this way. But bytecode has advantages:

**1. Performance**

Walking a tree requires following pointers between heap-allocated nodes.
Bytecode is a flat array of instructions -- sequential memory access, which
is much faster on modern CPUs due to cache locality.

```
AST interpretation:                 Bytecode execution:
    Follow pointer to BinaryOp      Read instruction at index 0
    Follow pointer to left child     Read instruction at index 1
    Follow pointer to NumberLiteral  Read instruction at index 2
    Read value                       Read instruction at index 3
    Back up to BinaryOp              ...
    Follow pointer to right child
    ...

    (lots of pointer chasing)        (sequential memory access)
```

**2. Portability**

Bytecode can be serialized to disk and loaded later. Java's `.class` files
work this way -- compile once, run anywhere that has a JVM.

**3. Optimization opportunities**

A flat instruction stream is easier to optimize than a tree. Peephole
optimizations (replacing patterns of instructions with faster alternatives)
are straightforward on bytecode.

**4. Separation of concerns**

The compiler focuses on *what* instructions to generate. The VM focuses on
*how* to execute them efficiently. Neither needs to know the other's details.

---

## Stack-Based vs Register-Based Bytecode

There are two main approaches to bytecode design.

### Stack-Based (what we implement)

Instructions operate on an implicit operand stack:

```
LOAD_CONST 1     push 1         stack: [1]
LOAD_CONST 2     push 2         stack: [1, 2]
ADD              pop 2, pop 1   stack: [3]
                 push 1+2
```

Instructions don't name their operands -- they always use the top of the
stack. This means instructions are compact (just an opcode and maybe one
operand) and the compiler doesn't need to allocate registers.

**Used by:** JVM, CLR, CPython, WebAssembly, our VM

### Register-Based

Instructions name their operands explicitly using register numbers:

```
LOAD R0, 1       R0 = 1
LOAD R1, 2       R1 = 2
ADD  R2, R0, R1  R2 = R0 + R1
```

Instructions are wider (they encode register numbers) but there are fewer
of them -- one ADD instead of three stack operations.

**Used by:** Lua's VM, Dalvik (old Android VM), real CPUs

### Comparison

```
                    Stack-Based             Register-Based
                    ===========             ==============
Instruction size:   Small (1-3 bytes)       Larger (encode reg #s)
Instruction count:  More instructions       Fewer instructions
Compiler work:      Simple (just push/pop)  Complex (register alloc)
Optimization:       Harder (implicit data)  Easier (explicit data)
Examples:           JVM, CLR, WASM          Lua VM, Dalvik
```

We use stack-based bytecode because it's simpler to compile to and simpler
to execute, making it ideal for learning.

---

## Walking the AST to Emit Bytecode -- The Visitor Pattern

The compiler traverses the AST using a technique called **post-order
traversal**: visit the children of a node before visiting the node itself.
This naturally produces stack-machine instructions.

### Why Post-Order?

Consider the expression `1 + 2`:

```
AST:       BinaryOp("+")
           /            \
    NumberLiteral(1)  NumberLiteral(2)
```

To add two numbers on a stack machine, both values must be on the stack
*before* the ADD instruction executes. So we must emit the operands first
(children), then the operation (parent):

```
1. Visit left child:   emit LOAD_CONST 0   (pushes 1)
2. Visit right child:  emit LOAD_CONST 1   (pushes 2)
3. Visit parent (+):   emit ADD             (pops both, pushes 3)
```

This is **post-order traversal** -- children before parent.

### Deeper Example: `1 + 2 * 3`

```
AST:           BinaryOp("+")
               /            \
        NumberLiteral(1)   BinaryOp("*")
                           /            \
                    NumberLiteral(2)  NumberLiteral(3)
```

Post-order traversal visits nodes in this order:

```
1. NumberLiteral(1)     --> LOAD_CONST 0    stack: [1]
2. NumberLiteral(2)     --> LOAD_CONST 1    stack: [1, 2]
3. NumberLiteral(3)     --> LOAD_CONST 2    stack: [1, 2, 3]
4. BinaryOp("*")        --> MUL             stack: [1, 6]
5. BinaryOp("+")        --> ADD             stack: [7]
```

The result: `LOAD_CONST 0, LOAD_CONST 1, LOAD_CONST 2, MUL, ADD`

This is **Reverse Polish Notation** (RPN). The stack does all the bookkeeping
that parentheses and precedence rules handle in the source code.

### The Compiler's _compile_expression Method

The compiler dispatches on the AST node type:

```
def _compile_expression(node):
    if node is NumberLiteral:
        index = add_to_constant_pool(node.value)
        emit(LOAD_CONST, index)

    elif node is StringLiteral:
        index = add_to_constant_pool(node.value)
        emit(LOAD_CONST, index)

    elif node is Name:
        index = add_to_name_pool(node.name)
        emit(LOAD_NAME, index)

    elif node is BinaryOp:
        _compile_expression(node.left)     # Push left value
        _compile_expression(node.right)    # Push right value
        emit(operator_to_opcode[node.op])  # Pop both, push result
```

The recursive structure mirrors the AST perfectly. Each call to
`_compile_expression` produces instructions that leave exactly one value
on the stack.

---

## Instruction Encoding

### Opcodes and Operands

Each instruction consists of an **opcode** (what to do) and an optional
**operand** (additional data):

```
Instruction         Opcode      Operand     Description
===============     =========   =========   ==========================
LOAD_CONST 0        0x01        0           Push constants[0]
LOAD_CONST 1        0x01        1           Push constants[1]
ADD                 0x20        (none)      Pop two, push sum
STORE_NAME 0        0x10        0           Pop, store in names[0]
HALT                0xFF        (none)      Stop execution
```

Our opcodes are grouped by category using the high nibble:

```
0x0_ = stack operations     (LOAD_CONST, POP, DUP)
0x1_ = variable operations  (STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL)
0x2_ = arithmetic           (ADD, SUB, MUL, DIV)
0x3_ = comparison           (CMP_EQ, CMP_LT, CMP_GT)
0x4_ = control flow         (JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE)
0x5_ = function operations  (CALL, RETURN)
0x6_ = I/O                  (PRINT)
0xF_ = VM control           (HALT)
```

### The Operator-to-Opcode Map

The compiler uses a dictionary to translate source-level operators to VM
opcodes:

```
    "+"  -->  OpCode.ADD   (0x20)
    "-"  -->  OpCode.SUB   (0x21)
    "*"  -->  OpCode.MUL   (0x22)
    "/"  -->  OpCode.DIV   (0x23)
```

---

## Multiple Backends: JVM, CLR, WASM

One of the most powerful aspects of our compiler architecture is that the
same AST can be compiled to *different* bytecode formats. The repo includes
four compilation backends:

```
                             +-- Our VM bytecode (compiler.py)
                             |
AST  -->  Compiler switch  --+-- JVM bytecode   (jvm_compiler.py)
                             |
                             +-- CLR IL          (clr_compiler.py)
                             |
                             +-- WASM bytecode   (wasm_compiler.py)
```

### How Each Backend Compiles `1 + 2`

**Our VM** (stack machine, high-level opcodes):
```
LOAD_CONST 0     # Push constants[0] = 1
LOAD_CONST 1     # Push constants[1] = 2
ADD              # Pop both, push 3
HALT             # Stop
```

**JVM** (stack machine, typed opcodes, compact encoding):
```
iconst_1         # 0x04  (single byte -- dedicated opcode for 1)
iconst_2         # 0x05  (single byte -- dedicated opcode for 2)
iadd             # 0x60  (integer add)
return           # 0xB1  (void return)
```

The JVM has dedicated single-byte opcodes (`iconst_0` through `iconst_5`) for
pushing the most common small constants. For larger values, it uses `bipush`
(2 bytes, values -128 to 127) or `ldc` (load from constant pool).

**CLR IL** (stack machine, type-inferred opcodes):
```
ldc.i4.1         # 0x17  (push int32 constant 1)
ldc.i4.2         # 0x18  (push int32 constant 2)
add              # 0x58  (type inferred from stack)
ret              # 0x2A  (return)
```

The CLR has short forms for constants 0-8 (more than JVM's 0-5). Its `add`
opcode works for any numeric type -- the CLR infers the type from what's
on the evaluation stack.

**WASM** (stack machine, uniform encoding):
```
i32.const 1      # 0x41 0x01000000  (5 bytes: opcode + LE int32)
i32.const 2      # 0x41 0x02000000  (5 bytes: opcode + LE int32)
i32.add          # 0x6A             (1 byte)
end              # 0x0B             (1 byte)
```

WASM uses a uniform encoding -- `i32.const` is always 5 bytes, regardless
of the value. This is simpler than JVM/CLR's tiered encoding but uses more
space for small constants.

### Comparison of Encoding Choices

```
                Our VM        JVM          CLR          WASM
                ======        ===          ===          ====
"Push 1":       2 bytes       1 byte       1 byte       5 bytes
                LOAD_CONST 0  iconst_1     ldc.i4.1     i32.const 1

"Push 42":      2 bytes       2 bytes      2 bytes      5 bytes
                LOAD_CONST 0  bipush 42    ldc.i4.s 42  i32.const 42

"Push 1000":    2 bytes       2 bytes      5 bytes      5 bytes
                LOAD_CONST 0  ldc #idx     ldc.i4 1000  i32.const 1000

"Add":          1 byte        1 byte       1 byte       1 byte
                ADD           iadd         add          i32.add
```

---

## Constant Pools

### What They Are

A constant pool is a list of literal values (numbers, strings) that
instructions reference by index rather than embedding directly.

```
Without constant pool:          With constant pool:

    LOAD_VALUE 42               constants = [42, "hello", 1000]
    LOAD_VALUE "hello"          LOAD_CONST 0    # constants[0] = 42
    LOAD_VALUE 1000             LOAD_CONST 1    # constants[1] = "hello"
                                LOAD_CONST 2    # constants[2] = 1000
```

### Why They're Needed

**1. Space efficiency**: If `42` appears ten times in the code, it's stored
once in the pool and referenced by index ten times.

**2. Uniform instruction format**: Every `LOAD_CONST` instruction has the same
shape: opcode + integer index. The instruction doesn't need to know whether
the constant is a small number, a big number, or a string.

**3. How real VMs do it**: The JVM's constant pool stores strings, class names,
method signatures, field references, and numeric literals. Our two pools
(constants + names) are a simplified version of the same idea.

### Deduplication

The compiler checks whether a value already exists in the pool before adding
it:

```
Compiling "x = 1 + 1":

    _compile_expression(NumberLiteral(1)):
        1 not in constants --> add it at index 0
        constants = [1]
        Emit LOAD_CONST 0

    _compile_expression(NumberLiteral(1)):
        1 already in constants at index 0 --> reuse it
        Emit LOAD_CONST 0

    Result:
        constants = [1]     (stored once, not twice)
        instructions = [LOAD_CONST 0, LOAD_CONST 0, ADD, STORE_NAME 0, HALT]
```

### The Name Pool

The name pool works exactly like the constant pool but for variable names.
When the compiler sees `x = 42`:

```
1. Compile the value 42:
   constants = [42],  emit LOAD_CONST 0

2. Store into variable "x":
   names = ["x"],  emit STORE_NAME 0

Later, when the code references "x" in an expression:
   "x" already in names at index 0
   emit LOAD_NAME 0
```

---

## Complete Compilation Walkthrough: `x = 1 + 2`

Here is the full compilation trace for a simple program:

```
Source:  x = 1 + 2

AST:    Assignment(
            target = Name("x"),
            value  = BinaryOp(
                left  = NumberLiteral(1),
                op    = "+",
                right = NumberLiteral(2)
            )
        )
```

**Step 1:** The compiler calls `_compile_statement(Assignment(...))`.

**Step 2:** `_compile_assignment` is called. It first compiles the right-hand
side expression:

**Step 3:** `_compile_expression(BinaryOp(1, "+", 2))` is called. Since it's
a BinaryOp, it recursively compiles left and right:

**Step 4:** `_compile_expression(NumberLiteral(1))`:
```
    1 is not in constants --> add at index 0
    constants = [1]
    Emit: LOAD_CONST 0
```

**Step 5:** `_compile_expression(NumberLiteral(2))`:
```
    2 is not in constants --> add at index 1
    constants = [1, 2]
    Emit: LOAD_CONST 1
```

**Step 6:** Back in the BinaryOp handler, emit the operator:
```
    op = "+" --> OpCode.ADD
    Emit: ADD
```

**Step 7:** Back in `_compile_assignment`, store the result:
```
    "x" is not in names --> add at index 0
    names = ["x"]
    Emit: STORE_NAME 0
```

**Step 8:** `compile()` appends the final HALT:
```
    Emit: HALT
```

### The Final CodeObject

```
CodeObject:
    instructions = [
        Instruction(LOAD_CONST, 0),    # Push 1
        Instruction(LOAD_CONST, 1),    # Push 2
        Instruction(ADD),              # 1 + 2 = 3
        Instruction(STORE_NAME, 0),    # x = 3
        Instruction(HALT),             # Stop
    ]
    constants = [1, 2]
    names = ["x"]
```

### Stack Trace During Execution

```
Instruction      Stack (after)    Variables
===========      =============    =========
LOAD_CONST 0     [1]              {}
LOAD_CONST 1     [1, 2]           {}
ADD              [3]              {}
STORE_NAME 0     []               {"x": 3}
HALT             []               {"x": 3}
```

---

## References

| File | Description |
|------|-------------|
| `code/packages/python/bytecode-compiler/src/bytecode_compiler/compiler.py` | Main compiler (AST to our VM bytecode) |
| `code/packages/python/bytecode-compiler/src/bytecode_compiler/jvm_compiler.py` | JVM bytecode backend |
| `code/packages/python/bytecode-compiler/src/bytecode_compiler/clr_compiler.py` | CLR IL bytecode backend |
| `code/packages/python/bytecode-compiler/src/bytecode_compiler/wasm_compiler.py` | WASM bytecode backend |
| `code/packages/python/virtual-machine/src/virtual_machine/vm.py` | VM that executes the bytecode |
