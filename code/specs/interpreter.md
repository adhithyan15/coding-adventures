# Pluggable Bytecode Compiler & Virtual Machine for Starlark

## Overview

This spec defines a **pluggable bytecode compiler and virtual machine** that can
execute Starlark programs — and, eventually, Python programs too.

The key idea: every virtual machine needs the same universal primitives (a stack,
memory, a program counter, a call stack, an eval loop), but the specific opcodes
and their semantics vary by language. We build a **generic chassis** that any
language can plug into, then Starlark plugs in its engine as the first client.

```
Source Code                        "x = 1 + 2"
    |
    v
Lexer (starlark-lexer)             NAME('x') EQUALS INT(1) PLUS INT(2)
    |
    v
Parser (starlark-parser)           ASTNode(rule_name="file", children=[...])
    |
    v
Bytecode Compiler (starlark-ast-to-bytecode-compiler)
    |                              LOAD_CONST 0    # push 1
    |                              LOAD_CONST 1    # push 2
    |                              ADD             # pop both, push 3
    |                              STORE_NAME 0    # pop 3, store in "x"
    v                              HALT
Virtual Machine (starlark-vm)      x = 3
```

This is **exactly** how real language runtimes work:

- **CPython**: Python source → `compile()` → bytecode → `ceval.c` eval loop
- **JVM**: Java source → `javac` → `.class` bytecode → JVM interpreter
- **YARV**: Ruby source → compiler → YARV instructions → VM

Our architecture mirrors these, but with a twist: the compiler and VM are
**pluggable**. Starlark registers its opcodes and rule handlers; later, Python
registers a larger set without touching the framework.


## Part 1: Why Pluggable?

### The Car Analogy

Think of a car:

- The **chassis** is universal: every car needs a frame, wheels, a steering
  column, brakes, a transmission. These are the same whether the car runs on
  gasoline, electricity, or hydrogen.

- The **engine** is specific: a gas engine has pistons and fuel injectors, an
  electric motor has coils and magnets. You swap the engine without redesigning
  the chassis.

Our VM is the chassis. Starlark's opcodes are the engine.

```
+-------------------------------------------------------+
|                   Generic VM (chassis)                 |
|                                                       |
|  +--------+  +----------+  +------+  +------------+  |
|  | Stack  |  | Variable |  | Call |  | Eval Loop  |  |
|  | (push/ |  | Storage  |  | Stack|  | (fetch/    |  |
|  |  pop)  |  | (global/ |  |      |  |  decode/   |  |
|  |        |  |  local)  |  |      |  |  execute)  |  |
|  +--------+  +----------+  +------+  +-----+------+  |
|                                            |          |
|                                    +-------v-------+  |
|                                    | Opcode        |  |
|                                    | Registry      |  |
|                                    | (PLUGIN POINT)|  |
|                                    +-------+-------+  |
+--------------------------------------------+---------+
                                             |
                        +--------------------+-------------------+
                        |                    |                   |
                  +-----v------+     +-------v------+    +------v-------+
                  | Starlark   |     | Python       |    | Your         |
                  | Opcodes    |     | Opcodes      |    | Language     |
                  | (~50 ops)  |     | (~100 ops)   |    | Opcodes      |
                  +------------+     +--------------+    +--------------+
```

### What Every VM Needs (Universal Primitives)

No matter what language you're running, your VM needs these:

1. **An operand stack** — where computation happens. Push values, operate on
   them, pop results. This is the same from the JVM to CPython to our VM.

2. **Variable storage** — somewhere to keep named values (`x = 42`). We provide
   two mechanisms:
   - A **name dictionary** for global/module scope (string → value)
   - **Local slots** for function scope (integer index → value, faster)

3. **A program counter (PC)** — which instruction are we on? Advances by one
   after each instruction unless a jump changes it.

4. **A call stack** — when you call a function, the VM saves where it was (the
   return address, local variables, stack state) so it can come back. This is
   identical to how hardware CPUs use the stack register.

5. **A constant pool** — literal values (numbers, strings) stored once and
   referenced by index. Saves space when the same constant appears many times.

6. **A name pool** — variable name strings stored once and referenced by index.
   `STORE_NAME 0` means "store in the variable whose name is `names[0]`."

7. **An eval loop** — the fetch-decode-execute cycle. Fetch the next
   instruction, look up its handler, execute it, repeat. This is the same loop
   at every level of the computing stack — from our CPU simulator (Layer 8) to
   the ARM simulator (Layer 7) to this VM.

8. **Execution tracing** — a record of every instruction executed, with
   before/after stack snapshots. Essential for debugging and visualization.

9. **Error infrastructure** — stack underflow, division by zero, undefined
   variables, type errors. Every VM needs these.

### What's Language-Specific (The Plugin)

These vary by language and are registered as plugins:

1. **Opcodes** — which operations exist (ADD, BUILD_LIST, FOR_ITER, ...)
2. **Opcode handlers** — what each opcode *does* (ADD might concatenate strings
   in one language, raise a TypeError in another)
3. **Value types** — what kinds of values the stack can hold (int, list, dict,
   class instance, generator, ...)
4. **Built-in functions** — len, range, print, sorted, ...
5. **Language restrictions** — Starlark forbids recursion; Python allows it


## Part 2: The Generic VM Framework

### Data Structures

These are the building blocks every language shares.

#### Instruction

A single VM instruction: an opcode plus an optional operand.

```
Instruction:
    opcode: int       # Which operation (e.g., 0x20 = ADD)
    operand: int | None   # Optional argument (e.g., constant pool index)
```

In a real bytecode format, this would be raw bytes: `[opcode_byte]
[operand_bytes...]`. We use a structured type for clarity, but the concept is
identical.

Some instructions need no operand (ADD just pops two values and pushes the
result). Others need an operand to know *which* constant to load (LOAD_CONST 3)
or *where* to jump (JUMP 17).

#### CodeObject

A compiled unit of code — the bytecode equivalent of a source file or function.

```
CodeObject:
    instructions: list[Instruction]    # The instruction sequence
    constants: list[value]             # Literal values pool
    names: list[str]                   # Variable name pool
    free_vars: list[str]               # Closure variable names (for nested functions)
    num_locals: int                    # Number of local variable slots needed
```

This is our version of Java's `.class` file, Python's `code` object, or .NET's
method body. It bundles everything the VM needs to execute a piece of code.

**Why pools?** Instead of embedding `42` directly in every instruction that uses
it, we store it once in `constants[0]` and emit `LOAD_CONST 0`. This saves space
(the constant is stored once even if used 100 times) and keeps instructions a
uniform size.

#### CallFrame

A saved execution context for function calls.

```
CallFrame:
    return_address: int                # PC to restore when function returns
    saved_locals: list[value]          # Caller's local variable slots
    saved_stack_depth: int             # Stack depth to restore on return
    closure_cells: list[Cell]          # Captured closure variables (optional)
```

When you call a function, the VM pushes a CallFrame. When the function returns,
the VM pops it and restores the saved state. This is exactly what real CPUs do
with their hardware stack — `call` pushes a return address, `ret` pops it.

#### Cell (for closures)

A mutable reference cell used to share variables between a function and its
enclosing scope. This is how closures work:

```
Cell:
    value: any    # The captured variable's current value
```

When a function captures a variable from its enclosing scope, both the outer
function and the inner function point to the same Cell. Mutations through either
reference are visible to both. CPython uses the exact same mechanism (`PyCell`
objects).

### The Generic VM Class

```python
class GenericVM:
    """A pluggable stack-based bytecode virtual machine.

    Languages register their opcodes and handlers at construction time.
    The eval loop is universal — it dispatches to registered handlers.
    """

    def __init__(self):
        # Universal state — every VM has these
        self.stack: list = []               # Operand stack
        self.variables: dict = {}           # Named variable storage (global scope)
        self.locals: list = []              # Local variable slots (function scope)
        self.pc: int = 0                    # Program counter
        self.halted: bool = False           # Has execution stopped?
        self.call_stack: list = []          # Saved contexts for function calls
        self.output: list[str] = []         # Captured print output

        # The plugin point — languages register handlers here
        self._opcode_handlers: dict[int, OpcodeHandler] = {}
        self._builtins: dict[str, callable] = {}

    # -- Plugin Registration --

    def register_opcode(self, opcode: int, handler: OpcodeHandler):
        """Register a handler for an opcode number.

        The handler signature is: handler(vm, instruction, code) -> None
        The handler reads operands from the instruction, manipulates vm state
        (push, pop, set pc, etc.), and advances the pc.
        """
        self._opcode_handlers[opcode] = handler

    def register_builtin(self, name: str, func: callable):
        """Register a built-in function by name.

        When CALL_FUNCTION encounters a BuiltinFunction value, it calls
        the registered implementation.
        """
        self._builtins[name] = func

    # -- The Eval Loop (universal) --

    def execute(self, code: CodeObject) -> list[VMTrace]:
        """Execute a CodeObject, returning a trace of every step."""
        traces = []
        while not self.halted and self.pc < len(code.instructions):
            trace = self.step(code)
            traces.append(trace)
        return traces

    def step(self, code: CodeObject) -> VMTrace:
        """Execute one instruction and return a trace."""
        instruction = code.instructions[self.pc]
        stack_before = list(self.stack)
        pc_before = self.pc

        handler = self._opcode_handlers.get(instruction.opcode)
        if handler is None:
            raise InvalidOpcodeError(
                f"No handler registered for opcode 0x{instruction.opcode:02X}"
            )
        handler(self, instruction, code)

        return VMTrace(
            pc=pc_before,
            instruction=instruction,
            stack_before=stack_before,
            stack_after=list(self.stack),
            variables=dict(self.variables),
        )

    # -- Universal Stack Helpers --

    def push(self, value):
        """Push a value onto the operand stack."""
        self.stack.append(value)

    def pop(self):
        """Pop and return the top value from the stack."""
        if not self.stack:
            raise StackUnderflowError("Cannot pop from empty stack")
        return self.stack.pop()

    def peek(self):
        """Return the top value without popping it."""
        if not self.stack:
            raise StackUnderflowError("Cannot peek at empty stack")
        return self.stack[-1]

    # -- Universal Call Stack Helpers --

    def push_frame(self, frame: CallFrame):
        """Save a call frame (entering a function)."""
        self.call_stack.append(frame)

    def pop_frame(self) -> CallFrame:
        """Restore a call frame (returning from a function)."""
        if not self.call_stack:
            raise VMError("Cannot return: call stack is empty")
        return self.call_stack.pop()

    def reset(self):
        """Reset VM to initial state for reuse."""
        self.stack.clear()
        self.variables.clear()
        self.locals.clear()
        self.pc = 0
        self.halted = False
        self.call_stack.clear()
        self.output.clear()
```

### Error Hierarchy

```
VMError (base)
├── StackUnderflowError      # Pop from empty stack
├── DivisionByZeroError      # a / 0
├── UndefinedNameError        # Variable not defined
├── InvalidOpcodeError        # Unknown opcode
├── InvalidOperandError       # Operand out of bounds
├── TypeError                 # Wrong type for operation (e.g., "hello" - 1)
├── IndexError                # List index out of range
├── KeyError                  # Dict key not found
└── ValueError                # Invalid value (e.g., int("hello"))
```


## Part 3: The Generic Compiler Framework

### The GenericCompiler Class

```python
class GenericCompiler:
    """A pluggable bytecode compiler that walks ASTNode trees.

    Languages register handlers for grammar rule names.
    The compiler dispatches on ASTNode.rule_name to the right handler.
    """

    def __init__(self):
        self.instructions: list[Instruction] = []
        self.constants: list = []
        self.names: list[str] = []
        self._dispatch: dict[str, CompileHandler] = {}

    # -- Plugin Registration --

    def register_rule(self, rule_name: str, handler: CompileHandler):
        """Register a compilation handler for a grammar rule.

        Handler signature: handler(compiler, node: ASTNode) -> None
        The handler uses compiler.emit(), compiler.add_constant(), etc.
        to produce bytecode for this AST construct.
        """
        self._dispatch[rule_name] = handler

    # -- The AST Walker --

    def compile_node(self, node):
        """Compile an AST node by dispatching on its rule_name.

        For Token nodes: compiles the literal value (INT, STRING, NAME, etc.)
        For ASTNode nodes: looks up the rule_name in the dispatch table.
        For single-child nodes with no handler: passes through to the child.
        """
        if is_token(node):
            self._compile_token(node)
            return

        handler = self._dispatch.get(node.rule_name)
        if handler:
            handler(self, node)
        elif len(node.children) == 1:
            # Pass-through: many grammar rules exist only for precedence
            # (e.g., "statement" wraps "simple_stmt" wraps "small_stmt")
            # and have exactly one child. Just recurse into it.
            self.compile_node(node.children[0])
        else:
            raise CompilerError(f"No handler for rule: {node.rule_name}")

    def compile(self, ast) -> CodeObject:
        """Compile a complete AST into a CodeObject."""
        self.compile_node(ast)
        self.emit(0xFF)  # HALT
        return CodeObject(
            instructions=list(self.instructions),
            constants=list(self.constants),
            names=list(self.names),
        )

    # -- Universal Emission Helpers --

    def emit(self, opcode: int, operand=None):
        """Append an instruction to the output."""
        self.instructions.append(Instruction(opcode, operand))

    def add_constant(self, value) -> int:
        """Add a constant to the pool (deduplicating). Returns its index."""
        if value in self.constants:
            return self.constants.index(value)
        self.constants.append(value)
        return len(self.constants) - 1

    def add_name(self, name: str) -> int:
        """Add a name to the name pool (deduplicating). Returns its index."""
        if name in self.names:
            return self.names.index(name)
        self.names.append(name)
        return len(self.names) - 1

    def current_offset(self) -> int:
        """Return the index of the next instruction to be emitted."""
        return len(self.instructions)

    # -- Jump Patching --
    #
    # Forward jumps are tricky: when compiling an if-statement, you need
    # to emit JUMP_IF_FALSE to skip the body, but you don't know HOW FAR
    # to jump until you've compiled the body. Solution:
    #
    #   1. Emit a placeholder: JUMP_IF_FALSE 0 (target unknown)
    #   2. Remember the index of that instruction
    #   3. Compile the body
    #   4. "Patch" the placeholder with the actual target
    #
    # This is called "backpatching" and it's used by every real compiler.

    def emit_jump(self, opcode: int) -> int:
        """Emit a jump with a placeholder target. Returns the patch index."""
        index = self.current_offset()
        self.emit(opcode, 0)  # 0 is the placeholder
        return index

    def patch_jump(self, index: int):
        """Patch a previously emitted jump to target the current offset."""
        self.instructions[index] = Instruction(
            self.instructions[index].opcode,
            self.current_offset()
        )

    def patch_jump_to(self, index: int, target: int):
        """Patch a previously emitted jump to a specific target."""
        self.instructions[index] = Instruction(
            self.instructions[index].opcode,
            target
        )
```

### Pass-Through Rules

Many grammar rules exist purely to encode operator precedence. For example, in
the Starlark grammar:

```
statement = compound_stmt | simple_stmt ;
simple_stmt = small_stmt { SEMICOLON small_stmt } NEWLINE ;
small_stmt = return_stmt | break_stmt | ... | assign_stmt ;
```

When `statement` matches and has exactly one child (`simple_stmt`), there's
nothing to *do* — just compile the child. The `compile_node` method handles this
automatically: if no handler is registered and the node has one child, it
recurses into the child.

This means we only need explicit handlers for rules that actually *do something*:
emit instructions, branch, define functions, etc.


## Part 4: Starlark's Opcode Set

Starlark registers the following opcodes with the generic VM. Each opcode is a
single byte value grouped by category.

### Stack Operations (0x00–0x0F)

| Opcode | Hex | Operand | Stack Effect | Description |
|--------|-----|---------|-------------|-------------|
| LOAD_CONST | 0x01 | index | → value | Push `constants[index]` onto the stack |
| POP | 0x02 | — | value → | Discard the top value |
| DUP | 0x03 | — | v → v v | Duplicate the top value |
| LOAD_NONE | 0x04 | — | → None | Push the None singleton |
| LOAD_TRUE | 0x05 | — | → True | Push boolean True |
| LOAD_FALSE | 0x06 | — | → False | Push boolean False |

**Why dedicated LOAD_NONE/TRUE/FALSE?** These singletons appear constantly in
real programs. Dedicated opcodes avoid wasting constant pool slots. CPython does
the same — `LOAD_CONST None` works but `LOAD_NONE` is one byte shorter.

### Variable Operations (0x10–0x1F)

| Opcode | Hex | Operand | Stack Effect | Description |
|--------|-----|---------|-------------|-------------|
| STORE_NAME | 0x10 | index | value → | `variables[names[index]] = pop()` |
| LOAD_NAME | 0x11 | index | → value | `push(variables[names[index]])` |
| STORE_LOCAL | 0x12 | index | value → | `locals[index] = pop()` |
| LOAD_LOCAL | 0x13 | index | → value | `push(locals[index])` |
| STORE_CLOSURE | 0x14 | index | value → | Store into a closure cell |
| LOAD_CLOSURE | 0x15 | index | → value | Load from a closure cell |

**Named vs local storage:** Named variables use dictionary lookup (`O(n)` worst
case). Local slots use array indexing (`O(1)`). Inside functions, the compiler
assigns each variable a slot number. This is exactly how CPython, the JVM, and
every serious VM handles function-local variables.

**Closure cells:** When a nested function captures a variable from its enclosing
scope, both scopes share a Cell object. LOAD_CLOSURE and STORE_CLOSURE read and
write through this indirection. This is the standard closure implementation — see
CPython's LOAD_DEREF/STORE_DEREF.

### Arithmetic Operations (0x20–0x2F)

| Opcode | Hex | Operand | Stack Effect | Description |
|--------|-----|---------|-------------|-------------|
| ADD | 0x20 | — | a b → result | `a + b` (int, float, str concat, list concat) |
| SUB | 0x21 | — | a b → result | `a - b` |
| MUL | 0x22 | — | a b → result | `a * b` (also `str * int` for repetition) |
| DIV | 0x23 | — | a b → result | `a / b` (float division) |
| FLOOR_DIV | 0x24 | — | a b → result | `a // b` (integer division) |
| MOD | 0x25 | — | a b → result | `a % b` (also `str % tuple` for formatting) |
| POWER | 0x26 | — | a b → result | `a ** b` |
| NEGATE | 0x27 | — | a → -a | Unary minus |
| BIT_AND | 0x28 | — | a b → result | `a & b` |
| BIT_OR | 0x29 | — | a b → result | `a \| b` |
| BIT_XOR | 0x2A | — | a b → result | `a ^ b` |
| BIT_NOT | 0x2B | — | a → ~a | Bitwise complement |
| LSHIFT | 0x2C | — | a b → result | `a << b` |
| RSHIFT | 0x2D | — | a b → result | `a >> b` |

**Operand ordering:** For binary ops, `a` is pushed first (deeper in stack) and
`b` is pushed second (top). The handler pops `b` then `a`. This is the universal
convention — JVM, CLR, CPython all do it this way.

**Type-polymorphic ADD:** In Starlark (like Python), `+` does different things
depending on the types:
- `int + int → int`
- `float + float → float`
- `int + float → float` (promotion)
- `str + str → str` (concatenation)
- `list + list → list` (concatenation)
- `tuple + tuple → tuple` (concatenation)
- Everything else → TypeError

The handler checks the types at runtime and dispatches accordingly. This is
called **dynamic dispatch** and it's how every dynamically-typed language works.

### Comparison Operations (0x30–0x3F)

| Opcode | Hex | Operand | Stack Effect | Description |
|--------|-----|---------|-------------|-------------|
| CMP_EQ | 0x30 | — | a b → bool | `a == b` |
| CMP_NE | 0x31 | — | a b → bool | `a != b` |
| CMP_LT | 0x32 | — | a b → bool | `a < b` |
| CMP_GT | 0x33 | — | a b → bool | `a > b` |
| CMP_LE | 0x34 | — | a b → bool | `a <= b` |
| CMP_GE | 0x35 | — | a b → bool | `a >= b` |
| CMP_IN | 0x36 | — | a b → bool | `a in b` (membership test) |
| CMP_NOT_IN | 0x37 | — | a b → bool | `a not in b` |
| NOT | 0x38 | — | a → bool | Logical not (truthy → False, falsy → True) |

**Note on `in`:** The `in` operator tests membership. For lists and tuples, it's
a linear scan. For dicts, it checks keys. For strings, it checks substrings.

### Control Flow (0x40–0x4F)

| Opcode | Hex | Operand | Stack Effect | Description |
|--------|-----|---------|-------------|-------------|
| JUMP | 0x40 | target | — | Set PC to target (unconditional) |
| JUMP_IF_FALSE | 0x41 | target | value → | Pop; if falsy, jump to target |
| JUMP_IF_TRUE | 0x42 | target | value → | Pop; if truthy, jump to target |
| JUMP_IF_FALSE_OR_POP | 0x43 | target | value → value? | If falsy: jump, keep value. If truthy: pop, continue |
| JUMP_IF_TRUE_OR_POP | 0x44 | target | value → value? | If truthy: jump, keep value. If falsy: pop, continue |

**Why JUMP_IF_FALSE_OR_POP?** This is for short-circuit boolean evaluation.
Consider `a and b`:
- Evaluate `a`, push result
- JUMP_IF_FALSE_OR_POP (to end): if `a` is falsy, the whole `and` is `a`
  (keep it on stack, skip `b`)
- If `a` is truthy: pop it, evaluate `b`, the whole `and` is `b`

CPython has the same pair of opcodes for exactly this reason.

**Truthiness rules** (what counts as "falsy"):
- `False`, `None`, `0`, `0.0`, `""`, `[]`, `()`, `{}` → falsy
- Everything else → truthy

### Function Operations (0x50–0x5F)

| Opcode | Hex | Operand | Stack Effect | Description |
|--------|-----|---------|-------------|-------------|
| MAKE_FUNCTION | 0x50 | flags | code [defaults] [closure] → func | Create a function value |
| CALL_FUNCTION | 0x51 | argc | func arg1...argN → result | Call function with N positional args |
| CALL_FUNCTION_KW | 0x52 | argc | func arg1...argN names → result | Call with keyword args |
| RETURN | 0x53 | — | value → (to caller) | Return from function |

**MAKE_FUNCTION flags:**
- Bit 0: has default values tuple on stack
- Bit 1: has closure cells tuple on stack

**Calling convention:**

To call `f(1, 2, key=3)`:
1. Push the function object `f`
2. Push positional args: `1`, `2`
3. Push keyword arg values: `3`
4. Push keyword arg names tuple: `("key",)`
5. Emit `CALL_FUNCTION_KW 3` (3 total args)

The VM's handler:
1. Pops the keyword names tuple
2. Pops all argument values
3. Matches positional args to parameter names in order
4. Matches keyword args to parameter names by keyword
5. Fills in default values for any unmatched parameters
6. Handles `*args` and `**kwargs` collection
7. Creates a new call frame, sets up local slots, jumps to function code

This calling convention is modeled directly on CPython's.

### Collection Operations (0x60–0x6F)

| Opcode | Hex | Operand | Stack Effect | Description |
|--------|-----|---------|-------------|-------------|
| BUILD_LIST | 0x60 | count | item1...itemN → list | Create list from top N stack items |
| BUILD_DICT | 0x61 | count | k1 v1...kN vN → dict | Create dict from N key-value pairs |
| BUILD_TUPLE | 0x62 | count | item1...itemN → tuple | Create tuple from top N stack items |
| LIST_APPEND | 0x63 | — | list value → list | Append value to list (for comprehensions) |
| DICT_SET | 0x64 | — | dict key value → dict | Set dict[key] = value (for comprehensions) |

**Why LIST_APPEND and DICT_SET?** Comprehensions like `[x*2 for x in range(5)]`
build a list incrementally inside a loop. The compiler emits:
1. `BUILD_LIST 0` (empty list)
2. Loop body: compute `x*2`, `LIST_APPEND`
3. The list grows with each iteration

### Subscript & Attribute Operations (0x70–0x7F)

| Opcode | Hex | Operand | Stack Effect | Description |
|--------|-----|---------|-------------|-------------|
| LOAD_SUBSCRIPT | 0x70 | — | obj key → value | `obj[key]` |
| STORE_SUBSCRIPT | 0x71 | — | obj key value → | `obj[key] = value` |
| LOAD_ATTR | 0x72 | index | obj → value | `obj.names[index]` |
| STORE_ATTR | 0x73 | index | obj value → | `obj.names[index] = value` |
| LOAD_SLICE | 0x74 | flags | obj [start] [stop] [step] → value | `obj[start:stop:step]` |

**LOAD_SLICE flags:** Bits indicate which of start/stop/step are present on the
stack (any absent ones default to None). This avoids pushing unnecessary None
values for common cases like `lst[1:]` or `lst[::-1]`.

### Iteration Operations (0x80–0x8F)

| Opcode | Hex | Operand | Stack Effect | Description |
|--------|-----|---------|-------------|-------------|
| GET_ITER | 0x80 | — | iterable → iterator | Convert iterable to an iterator |
| FOR_ITER | 0x81 | target | iterator → iterator value | Get next item, or jump to target if exhausted |
| UNPACK_SEQUENCE | 0x82 | count | seq → item1...itemN | Unpack sequence into N items |

**Iterator protocol:** `GET_ITER` wraps any iterable (list, dict, tuple, string,
range) in an iterator object that tracks position. `FOR_ITER` calls the
iterator's `next()` method. If there's a next item, it pushes it (keeping the
iterator on the stack for the next iteration). If exhausted, it pops the iterator
and jumps past the loop body.

This is exactly CPython's `GET_ITER` + `FOR_ITER` pair.

**For-loop compilation:**
```
for x in items:       GET_ITER
    body              FOR_ITER (to after loop)  ← loop start
                      STORE_LOCAL x
                      <body>
                      JUMP (to loop start)
                      ← FOR_ITER jumps here when exhausted
```

### Module Operations (0x90–0x9F)

| Opcode | Hex | Operand | Stack Effect | Description |
|--------|-----|---------|-------------|-------------|
| LOAD_MODULE | 0x90 | index | → module | Load module by path (`names[index]`) |
| IMPORT_FROM | 0x91 | index | module → module value | Extract `names[index]` from module |

These support Starlark's `load()` statement:
```python
load("//path:file.star", "symbol_a", renamed = "symbol_b")
```

### I/O Operations (0xA0–0xAF)

| Opcode | Hex | Operand | Stack Effect | Description |
|--------|-----|---------|-------------|-------------|
| PRINT | 0xA0 | — | value → | Print value, capture in `vm.output` |

### VM Control (0xF0–0xFF)

| Opcode | Hex | Operand | Stack Effect | Description |
|--------|-----|---------|-------------|-------------|
| HALT | 0xFF | — | — | Stop execution |


## Part 5: Starlark's Value Types

The Starlark VM operates on these runtime value types. Each has defined behavior
for truthiness, equality, ordering, hashing, and supported operations.

### Type Table

| Type | Truthy | Hashable | Iterable | Mutable | Freezable |
|------|--------|----------|----------|---------|-----------|
| int | ≠ 0 | yes | no | no | n/a (immutable) |
| float | ≠ 0.0 | yes | no | no | n/a (immutable) |
| str | ≠ "" | yes | yes (chars) | no | n/a (immutable) |
| bool | == True | yes | no | no | n/a (immutable) |
| NoneType | never | yes | no | no | n/a (immutable) |
| list | ≠ [] | **no** | yes | yes | **yes** |
| dict | ≠ {} | **no** | yes (keys) | yes | **yes** |
| tuple | ≠ () | yes* | yes | no | n/a (immutable) |
| function | always | no | no | no | n/a |
| builtin | always | no | no | no | n/a |

*Tuples are hashable only if all their elements are hashable.

### Type Coercion Rules

When arithmetic involves mixed types, Starlark promotes:
- `int + float → float` (int is promoted to float)
- `int + str → TypeError` (no implicit conversion)
- `bool + int → int` (bool is a subtype of int: True=1, False=0)

### Operator Dispatch

The ADD handler, for example:
```
def handle_add(vm, instruction, code):
    b = vm.pop()
    a = vm.pop()
    if isinstance(a, int) and isinstance(b, int):
        vm.push(a + b)
    elif isinstance(a, float) or isinstance(b, float):
        vm.push(float(a) + float(b))
    elif isinstance(a, str) and isinstance(b, str):
        vm.push(a + b)
    elif isinstance(a, list) and isinstance(b, list):
        vm.push(a + b)
    elif isinstance(a, tuple) and isinstance(b, tuple):
        vm.push(a + b)
    else:
        raise TypeError(f"unsupported + for {type_name(a)} and {type_name(b)}")
    vm.pc += 1
```

### Freezing

Starlark has a unique feature: after a module finishes loading, all its values
are **frozen** — made deeply immutable. This prevents one BUILD file from
mutating data that another BUILD file depends on, ensuring deterministic builds.

When frozen:
- Lists become immutable (append, insert, pop all raise errors)
- Dicts become immutable (assignment, pop, clear all raise errors)
- Functions that captured mutable values now hold frozen versions

The Starlark VM wrapper adds a `freeze()` method that walks all values in the
global scope and marks them as frozen.


## Part 6: Starlark Compiler — Rule Handlers

The Starlark compiler registers a handler for each grammar rule that needs
compilation. Here are the key patterns.

### Compilation Patterns

#### Assignment: `x = expr`

```
Grammar: assign_stmt = expression_list assign_op expression_list
                     | expression_list augmented_assign_op expression_list
                     | expression_list

Compilation:
    compile(right-hand side)    # pushes value
    STORE_NAME <index>          # pops value, stores in variable

For augmented assignment (x += 1):
    LOAD_NAME <index>           # push current x
    compile(right-hand side)    # push 1
    ADD                         # compute x + 1
    STORE_NAME <index>          # store result back in x
```

#### If/elif/else

```
Grammar: if_stmt = "if" expression COLON suite
                   { "elif" expression COLON suite }
                   [ "else" COLON suite ]

Compilation:
    compile(condition)
    JUMP_IF_FALSE → elif_or_else
    compile(if_body)
    JUMP → end
  elif_or_else:
    compile(elif_condition)         # (for each elif)
    JUMP_IF_FALSE → next_elif_or_else
    compile(elif_body)
    JUMP → end
  else:
    compile(else_body)
  end:
```

#### For loop

```
Grammar: for_stmt = "for" loop_vars "in" expression COLON suite

Compilation:
    compile(iterable_expression)
    GET_ITER
  loop_start:
    FOR_ITER → loop_end
    STORE_LOCAL <loop_var>      # (or UNPACK_SEQUENCE for multiple vars)
    compile(body)               # break → JUMP loop_end
    JUMP → loop_start           # continue → JUMP loop_start
  loop_end:
```

#### Function definition

```
Grammar: def_stmt = "def" NAME LPAREN [ parameters ] RPAREN COLON suite

Compilation:
    # 1. Compile the body as a separate CodeObject
    inner_compiler = GenericCompiler()
    register_starlark_rules(inner_compiler)
    body_code = inner_compiler.compile(suite)

    # 2. In the outer code, load the body CodeObject and create a function
    code_index = compiler.add_constant(body_code)
    LOAD_CONST <code_index>
    # push default values if any
    MAKE_FUNCTION <flags>
    STORE_NAME <function_name>
```

#### Short-circuit `and` / `or`

```
a and b:
    compile(a)
    JUMP_IF_FALSE_OR_POP → end    # if a is falsy, result is a
    compile(b)                     # if a is truthy, result is b
  end:

a or b:
    compile(a)
    JUMP_IF_TRUE_OR_POP → end     # if a is truthy, result is a
    compile(b)                     # if a is falsy, result is b
  end:
```

#### List comprehension: `[x*2 for x in items if x > 0]`

```
    BUILD_LIST 0                   # empty result list
    compile(items)
    GET_ITER
  loop:
    FOR_ITER → end
    STORE_LOCAL <x>
    compile(x > 0)                 # if-filter
    JUMP_IF_FALSE → loop           # skip this element
    compile(x * 2)                 # the expression
    LIST_APPEND                    # append to result list
    JUMP → loop
  end:
```

### Complete Rule → Handler Mapping

Here are all 55 grammar rules and how they're handled:

| Rule | Handler | Notes |
|------|---------|-------|
| file | compile_file | Compile each statement child |
| statement | pass-through | Single child |
| simple_stmt | compile_simple_stmt | Handle semicolons, skip NEWLINE |
| compound_stmt | pass-through | Single child |
| small_stmt | pass-through | Single child |
| return_stmt | compile_return | Compile value (or None), emit RETURN |
| break_stmt | compile_break | Emit JUMP to loop end |
| continue_stmt | compile_continue | Emit JUMP to loop start |
| pass_stmt | compile_pass | No-op (emit nothing) |
| load_stmt | compile_load | LOAD_MODULE + IMPORT_FROM |
| load_arg | (handled by load_stmt) | |
| assign_stmt | compile_assign | Regular or augmented assignment |
| assign_op | (handled by assign_stmt) | |
| augmented_assign_op | (handled by assign_stmt) | |
| if_stmt | compile_if | Conditional jumps with backpatching |
| for_stmt | compile_for | GET_ITER + FOR_ITER loop |
| loop_vars | (handled by for_stmt) | UNPACK_SEQUENCE if multiple vars |
| def_stmt | compile_def | Nested CodeObject + MAKE_FUNCTION |
| suite | compile_suite | Compile body statements |
| parameters | (handled by def_stmt) | Parameter analysis |
| parameter | (handled by def_stmt) | |
| expression_list | compile_expr_list | May BUILD_TUPLE if multiple |
| expression | compile_expression | Ternary or pass-through |
| lambda_expr | compile_lambda | Like def but inline |
| lambda_params | (handled by lambda) | |
| lambda_param | (handled by lambda) | |
| or_expr | compile_or | JUMP_IF_TRUE_OR_POP |
| and_expr | compile_and | JUMP_IF_FALSE_OR_POP |
| not_expr | compile_not | NOT opcode |
| comparison | compile_comparison | CMP_* opcodes |
| comp_op | (handled by comparison) | |
| bitwise_or | compile_binary_op | BIT_OR |
| bitwise_xor | compile_binary_op | BIT_XOR |
| bitwise_and | compile_binary_op | BIT_AND |
| shift | compile_binary_op | LSHIFT / RSHIFT |
| arith | compile_binary_op | ADD / SUB |
| term | compile_binary_op | MUL / DIV / FLOOR_DIV / MOD |
| factor | compile_factor | NEGATE / BIT_NOT / pass-through |
| power | compile_power | POWER |
| primary | compile_primary | Atom + suffixes |
| suffix | (handled by primary) | DOT / call / subscript / slice |
| subscript | (handled by primary) | |
| atom | compile_atom | Literals, names, collections |
| list_expr | compile_list | BUILD_LIST or comprehension |
| list_body | (handled by list_expr) | |
| dict_expr | compile_dict | BUILD_DICT or comprehension |
| dict_body | (handled by dict_expr) | |
| dict_entry | (handled by dict_expr) | |
| paren_expr | compile_paren | Grouping or tuple |
| paren_body | (handled by paren_expr) | |
| comp_clause | compile_comprehension | FOR_ITER loop with filters |
| comp_clause_rest | (handled by comp_clause) | |
| call_args | (handled by primary) | |
| argument | (handled by primary) | |
| slice_expr | (handled by primary) | LOAD_SLICE |


## Part 7: Starlark Built-in Functions

Starlark provides approximately 30 built-in functions. These are registered with
the VM's built-in registry and invoked via CALL_FUNCTION when the callee is a
BuiltinFunction value.

### Type Conversion

| Function | Signature | Description |
|----------|-----------|-------------|
| `bool` | `bool(x=False)` | Convert to bool. Follows truthiness rules. |
| `int` | `int(x=0)`, `int(x, base=10)` | Convert to int. Strings parsed in given base. |
| `float` | `float(x=0)` | Convert to float. |
| `str` | `str(x="")` | Convert to human-readable string. |
| `list` | `list(iterable=())` | Create list from iterable. |
| `dict` | `dict(**kwargs)`, `dict(pairs)` | Create dict. |
| `tuple` | `tuple(iterable=())` | Create tuple from iterable. |

### Sequence Operations

| Function | Signature | Description |
|----------|-----------|-------------|
| `len` | `len(x)` | Length of string, list, tuple, or dict. |
| `range` | `range(stop)`, `range(start, stop[, step])` | Return an iterable range of ints. |
| `sorted` | `sorted(iterable, *, key=None, reverse=False)` | Return new sorted list. |
| `reversed` | `reversed(sequence)` | Return reversed iterator. |
| `enumerate` | `enumerate(iterable, start=0)` | Yield (index, value) pairs. |
| `zip` | `zip(*iterables)` | Yield tuples of parallel items. |

### Aggregation

| Function | Signature | Description |
|----------|-----------|-------------|
| `min` | `min(iterable)`, `min(a, b, ...)` | Smallest item. |
| `max` | `max(iterable)`, `max(a, b, ...)` | Largest item. |
| `sum` | `sum(iterable, start=0)` | Sum of numeric items. |
| `all` | `all(iterable)` | True if all items are truthy. |
| `any` | `any(iterable)` | True if any item is truthy. |
| `abs` | `abs(x)` | Absolute value. |

### Introspection

| Function | Signature | Description |
|----------|-----------|-------------|
| `type` | `type(x)` | Return type name as string ("int", "list", etc.) |
| `dir` | `dir(x)` | Return list of attribute names. |
| `hasattr` | `hasattr(x, name)` | True if x has the named attribute. |
| `getattr` | `getattr(x, name[, default])` | Get attribute or default. |
| `hash` | `hash(x)` | Return hash value (for dict keys). |
| `repr` | `repr(x)` | Return debug string representation. |

### Output

| Function | Signature | Description |
|----------|-----------|-------------|
| `print` | `print(*args, sep=" ")` | Print values, captured in `vm.output`. |
| `fail` | `fail(msg)` | Halt execution with an error message. |


## Part 8: Starlark Restrictions

The Starlark VM wrapper adds these language-specific restrictions on top of the
generic VM framework.

### No Recursion

Starlark forbids recursion to guarantee termination. The Starlark VM maintains a
set of currently-executing functions. When CALL_FUNCTION is about to invoke a
function, it checks whether that function is already on the call stack. If so, it
raises an error.

```
Error: function "factorial" called recursively
```

This is a Starlark-specific restriction. The generic VM has no such check, and a
Python VM would not add one.

### Freezing

After a Starlark module finishes evaluating (all top-level statements have
executed), the VM calls `freeze()` on all values in the global scope. Frozen
values reject any mutation:

```
Error: cannot modify frozen list
```

The freeze is deep — a frozen list's elements are also frozen, a frozen dict's
values are also frozen, etc.

### No While Loops

Starlark has no `while` statement. This is enforced by the grammar (there's no
`while_stmt` rule), so it's already handled at parse time. The compiler and VM
don't need special logic for this.

### Reserved Keywords

Using Python keywords that aren't in Starlark (`class`, `import`, `while`,
`try`, etc.) is caught at lex time — the lexer raises an error before the parser
or compiler ever see them.


## Part 9: Reuse Roadmap — From Starlark to Python

The pluggable architecture means adding Python support requires:

### What Stays Identical (the generic framework)

- The entire GenericVM class (stack, eval loop, call stack, tracing)
- The entire GenericCompiler class (emit, constant pool, jump patching)
- All data structures (CodeObject, Instruction, CallFrame, Cell)
- Error hierarchy

### What Python Adds (new opcodes, registered as plugins)

| Feature | New Opcodes | Notes |
|---------|------------|-------|
| While loops | (reuse JUMP, JUMP_IF_FALSE) | Same opcodes, different compiler rule |
| Try/except/finally | SETUP_EXCEPT, POP_EXCEPT, RAISE, END_FINALLY | Exception table in CodeObject |
| Classes | LOAD_BUILD_CLASS, STORE_ATTR, LOAD_ATTR | Already have ATTR opcodes |
| With statement | SETUP_WITH, WITH_CLEANUP | Context manager protocol |
| Generators | YIELD_VALUE, YIELD_FROM, RESUME | Requires coroutine support in VM |
| Import | IMPORT_MODULE, IMPORT_NAME, IMPORT_FROM | More complex than load() |
| Global/nonlocal | (reuse STORE_NAME with scope flags) | Compiler scope analysis |
| Decorators | (no new opcodes — syntactic sugar) | Compiled as function calls |
| F-strings | FORMAT_VALUE, BUILD_STRING | String interpolation |
| Walrus operator | (reuse STORE_LOCAL/STORE_NAME) | := assignment expression |

### What Python Adds (new value types)

- `set`, `frozenset` — unordered collections
- `bytes`, `bytearray` — binary data
- `class` / `instance` — user-defined types with `__init__`, `__add__`, etc.
- `generator` — lazy iteration via yield
- `exception` — error values with traceback
- `complex` — complex numbers

### Estimated Reuse

- **Framework**: 100% reuse (GenericVM, GenericCompiler, data structures)
- **Opcodes**: ~70% of Starlark's opcodes are needed by Python too
- **Value types**: ~80% carry over (int, float, str, list, dict, tuple, bool, None, function)
- **Built-in functions**: ~90% carry over (len, range, sorted, print, etc.)

The main new work for Python: exception handling, classes, generators, import system.


## Part 10: Testing Strategy

### Unit Tests — VM Opcode Handlers

Test each opcode handler in isolation:
- Push values, execute one instruction, verify stack state
- Test edge cases: empty stack, type mismatches, overflow
- Test every value type combination for polymorphic ops (ADD with int, float, str, list)

### Unit Tests — Compiler Rule Handlers

Test each grammar rule handler:
- Parse a small Starlark snippet, compile it, verify the instruction sequence
- Test if/elif/else produces correct jump targets
- Test for-loop produces GET_ITER/FOR_ITER/JUMP pattern
- Test function def produces nested CodeObject

### Integration Tests — End-to-End

Source code → lex → parse → compile → execute → verify result:
- `x = 1 + 2 * 3` → x is 7
- `if True: x = 1\nelse: x = 2\n` → x is 1
- `result = [x*2 for x in range(5)]` → result is [0, 2, 4, 6, 8]
- `def add(a, b): return a + b\nx = add(3, 4)\n` → x is 7
- Full Starlark BUILD file snippets

### Restriction Tests

- Recursion: `def f(): f()\nf()\n` → error
- Freezing: execute module, then attempt mutation → error
- Reserved keywords: `class Foo:` → lexer error

### Coverage Target

- Generic framework (bytecode-compiler, virtual-machine): 90%+
- Starlark wrappers (starlark-ast-to-bytecode-compiler, starlark-vm): 90%+


## Part 11: Packages Summary

### Modifications to Existing Packages

| Package | Changes |
|---------|---------|
| `bytecode-compiler` (all 5 langs) | Add `GenericCompiler` class alongside existing `BytecodeCompiler`. Add `CompileHandler` type. Add `compile_node`, `emit_jump`, `patch_jump` helpers. Existing code untouched. |
| `virtual-machine` (all 5 langs) | Add `GenericVM` class alongside existing `VirtualMachine`. Add `OpcodeHandler` type. Add `register_opcode`, `register_builtin` plugin methods. Existing code untouched. |

### New Packages

| Package | Per Language | Description |
|---------|-------------|-------------|
| `starlark-ast-to-bytecode-compiler` | 5 | Registers all 55 grammar rule handlers with GenericCompiler. |
| `starlark-vm` | 5 | Registers all ~50 opcode handlers and ~30 built-ins with GenericVM. |

**10 new packages total** + backward-compatible extensions to 10 existing packages.
