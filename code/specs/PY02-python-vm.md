# PY02 — Python Virtual Machine

## Overview

This spec describes a complete Python 3 virtual machine implemented from scratch
in Go, TypeScript, and Rust. The VM executes Python 3.0 through 3.12 semantics
using the infrastructure this monorepo already provides: grammar-driven lexers,
grammar-driven parsers, a pluggable bytecode compiler, and the `GenericVM` with
its opcode-registry architecture.

The goal is **not** to replace CPython. The goal is to understand every layer of
how Python actually works — from the moment source text enters the lexer, through
AST construction, bytecode emission, and finally runtime execution on a virtual
stack machine. Every design decision made by Guido van Rossum and the CPython
developers has a reason; by rebuilding the machine ourselves, we discover those
reasons firsthand.

### What makes this different from the Starlark VM?

Starlark (spec 14) is Python-minus-danger: no `while`, no classes, no exceptions,
no imports, no generators, no `async`. It is a configuration language that borrows
Python's syntax but deliberately removes Python's runtime complexity.

The Python VM adds all of that back:

| Feature | Starlark | Python VM |
|---------|----------|-----------|
| `while` loops | Removed (termination guarantee) | Full support |
| `class` | Removed (no OOP) | Full support, multiple inheritance, MRO |
| `try`/`except`/`finally` | Removed (errors halt) | Full exception hierarchy |
| `import` | Replaced with `load()` | Full module system with `sys.path` |
| `yield`/generators | Removed (too complex) | Full generator protocol |
| `async`/`await` | Removed (no concurrency) | Full coroutine support |
| `global`/`nonlocal` | Removed (no mutable shared state) | Full closure semantics |
| `with` statements | Removed (no resources) | Full context manager protocol |
| `del` | Removed | Full deletion semantics |
| Recursion | Disabled | Full support (with stack depth limit) |
| Decorators | Not applicable | Full `@decorator` syntax |
| `match`/`case` | Not applicable (3.10+) | Full structural pattern matching |

This is the full language. Every corner, every edge case, every piece of runtime
machinery that makes Python *Python*.

## Architecture

### The Pipeline

```
Python Source Code
        |
        v
+------------------+     Wraps GrammarLexer with version-aware
|  Python Lexer    |     .tokens files (python-3.0.tokens through
|  (version-aware) |     python-3.12.tokens). Handles f-strings,
+------------------+     walrus operator, match keywords per version.
        |
        v  Token Stream
+------------------+     Wraps GrammarParser with version-aware
|  Python Parser   |     .grammar files. Produces an AST whose
|  (version-aware) |     node types vary by target version.
+------------------+
        |
        v  Abstract Syntax Tree
+------------------+     Extends BytecodeCompiler with a Python
|  Python Compiler |     backend. Walks the AST, emits bytecode
|  (AST -> bytes)  |     instructions targeting the GenericVM.
+------------------+
        |
        v  CodeObject (bytecode + constants + names)
+------------------+     The existing GenericVM with Python-specific
|  GenericVM +     |     opcodes registered via register_opcode().
|  Python Opcodes  |     Maintains the Python object model at runtime.
+------------------+
        |
        v  Program Output
```

Each stage reuses existing abstractions — this is critical. We are not building
a separate Python implementation from scratch. We are *extending* the compiler
pipeline that already works for Starlark, teaching it the additional concepts
that Python requires.

### Reuse Strategy

**Lexer.** The `GrammarLexer` already tokenizes based on `.tokens` files. Python
needs versioned token files because the lexer grammar changed across releases:
- Python 3.0: base set
- Python 3.6: `FSTRING_START`, `FSTRING_MIDDLE`, `FSTRING_END` tokens
- Python 3.8: `COLONEQUAL` (`:=`, the walrus operator)
- Python 3.10: `SOFT_KEYWORD` for `match` and `case` (they are identifiers in
  earlier versions, keywords only in match context)
- Python 3.12: `FSTRING_*` tokens refined for PEP 701 (formalized f-string grammar)

**Parser.** The `GrammarParser` already builds ASTs from `.grammar` files. Python
needs versioned grammar files for structural changes:
- Python 3.8: assignment expressions (`x := expr`)
- Python 3.9: dictionary merge operators (`d1 | d2`)
- Python 3.10: `match`/`case` statements with structural pattern matching
- Python 3.11: exception groups (`except*`)
- Python 3.12: type parameter syntax (`class C[T]: ...`)

**Compiler.** The `BytecodeCompiler` has a backend registry. We add a `PythonBackend`
that knows how to compile Python AST nodes into bytecode. Much of the logic is
shared with the Starlark backend — `if`, `for`, function calls, list/dict
construction. The Python backend adds: `while`, `class`, `try`, `import`,
`yield`, `async`, `with`, `match`.

**VM.** The `GenericVM` dispatches opcodes through a registry. The Starlark VM
registers ~46 opcodes. The Python VM registers those same 46 plus ~50 more.
The eval loop, stack, and frame machinery are shared.

## Bytecode Design

### Design Principles

Every opcode in our bytecode corresponds to a single, atomic operation on the
value stack. This is the same design principle used by CPython, the JVM, and
the CLR. The reasons are:

1. **Simplicity.** Each opcode does one thing. The eval loop is a flat switch
   statement with no nested logic.
2. **Composability.** Complex operations are sequences of simple opcodes.
   `a.b.c()` becomes `LOAD_NAME a`, `LOAD_ATTR b`, `LOAD_ATTR c`, `CALL_METHOD 0`.
3. **Debuggability.** You can print the bytecode and trace execution step by step.
4. **Portability.** The bytecode is the same across Go, TypeScript, and Rust
   implementations. Only the eval loop differs (in language idiom, not semantics).

### Opcode Encoding

Each instruction is encoded as:

```
+--------+--------+--------+
| opcode |   arg (16-bit)  |
| 1 byte |  2 bytes, LE    |
+--------+--------+--------+
```

Opcodes without arguments still occupy 3 bytes (arg = 0). This simplifies the
instruction pointer arithmetic: `IP += 3` after every instruction, unless a
JUMP opcode overrides it.

For arguments larger than 65535, we use the `EXTENDED_ARG` prefix (same as
CPython): it shifts the next instruction's argument left by 16 bits.

### Existing Opcodes (Inherited from Starlark)

These opcodes already exist in the Starlark VM and are reused without change.
They form the computational core of the Python VM:

```
Category: Constants and Variables
  LOAD_CONST      index    ->  Push constants[index] onto the stack
  LOAD_NAME       index    ->  Push value of names[index] from local scope
  STORE_NAME      index    ->  Pop TOS, store into names[index] in local scope
  LOAD_FAST       index    ->  Push value of locals[index] (optimized local access)
  STORE_FAST      index    ->  Pop TOS, store into locals[index]

Category: Binary Arithmetic
  BINARY_ADD       -       ->  Pop b, pop a, push a + b
  BINARY_SUB       -       ->  Pop b, pop a, push a - b
  BINARY_MUL       -       ->  Pop b, pop a, push a * b
  BINARY_DIV       -       ->  Pop b, pop a, push a / b  (true division)
  BINARY_MOD       -       ->  Pop b, pop a, push a % b
  BINARY_FLOOR_DIV -       ->  Pop b, pop a, push a // b
  BINARY_POWER     -       ->  Pop b, pop a, push a ** b

Category: Unary Operations
  UNARY_NEG        -       ->  Pop a, push -a
  UNARY_NOT        -       ->  Pop a, push not a
  UNARY_INVERT     -       ->  Pop a, push ~a

Category: Bitwise Operations
  BINARY_AND       -       ->  Pop b, pop a, push a & b
  BINARY_OR        -       ->  Pop b, pop a, push a | b
  BINARY_XOR       -       ->  Pop b, pop a, push a ^ b
  BINARY_LSHIFT    -       ->  Pop b, pop a, push a << b
  BINARY_RSHIFT    -       ->  Pop b, pop a, push a >> b

Category: Comparison
  COMPARE_OP      op       ->  Pop b, pop a, push a <op> b
                               (op: EQ, NE, LT, GT, LE, GE, IN, NOT_IN,
                                IS, IS_NOT)

Category: Control Flow
  JUMP            target   ->  Set IP to target (unconditional)
  JUMP_IF_FALSE   target   ->  Pop TOS, jump to target if falsy
  JUMP_IF_TRUE    target   ->  Pop TOS, jump to target if truthy

Category: Collection Construction
  BUILD_LIST      count    ->  Pop count items, push list
  BUILD_DICT      count    ->  Pop count*2 items (key/value pairs), push dict
  BUILD_TUPLE     count    ->  Pop count items, push tuple
  BUILD_SET       count    ->  Pop count items, push set

Category: Function Operations
  CALL_FUNCTION   argc     ->  Pop argc args + callable, call, push result
  RETURN_VALUE     -       ->  Pop TOS, return from current frame
  MAKE_FUNCTION   flags    ->  Pop code + defaults, create function object

Category: Iteration
  FOR_ITER        delta    ->  Call __next__ on TOS iterator; push result,
                               or jump by delta if StopIteration
  GET_ITER         -       ->  Pop TOS, push iter(TOS)
  UNPACK_SEQUENCE count    ->  Pop TOS iterable, push count items

Category: Stack Manipulation
  POP_TOP          -       ->  Discard TOS
  DUP_TOP          -       ->  Push a copy of TOS
  ROT_TWO          -       ->  Swap TOS and TOS1
  ROT_THREE        -       ->  Rotate top three: TOS -> TOS2, TOS1 -> TOS,
                               TOS2 -> TOS1

Category: Attribute and Subscript Access
  LOAD_ATTR       name     ->  Pop TOS, push getattr(TOS, name)
  STORE_ATTR      name     ->  Pop value, pop obj, setattr(obj, name, value)
  LOAD_SUBSCR      -       ->  Pop key, pop obj, push obj[key]
  STORE_SUBSCR     -       ->  Pop key, pop obj, pop value, obj[key] = value
  BUILD_SLICE     argc     ->  Pop 2 or 3 args, push slice object

Category: Comprehension Helpers (partial)
  LIST_APPEND     offset   ->  Peek at list at stack[offset], append TOS
  DICT_SET_ITEM   offset   ->  Peek at dict at stack[offset], set key/value
```

That is approximately 46 opcodes — the Starlark instruction set.

### New Python Opcodes

These opcodes extend the Starlark set to support full Python semantics. Each
is described with its stack effect, purpose, and the Python construct it compiles.

#### Scope Management

Python has three scopes that Starlark lacks: global, nonlocal (closure), and
deletion. Starlark's scope model is simple: names are either local or built-in.
Python adds a full LEGB (Local, Enclosing, Global, Built-in) resolution chain.

```
LOAD_GLOBAL      index    ->  Push value of names[index] from global scope
STORE_GLOBAL     index    ->  Pop TOS, store into names[index] in global scope
DELETE_NAME      index    ->  Delete names[index] from local scope
DELETE_FAST      index    ->  Delete locals[index]
DELETE_GLOBAL    index    ->  Delete names[index] from global scope
```

**Why separate LOAD_GLOBAL from LOAD_NAME?** Performance. In CPython, LOAD_NAME
searches local, then enclosing, then global, then built-in scopes — four hash
table lookups in the worst case. LOAD_GLOBAL skips the local and enclosing
scopes entirely, going straight to global then built-in. When the compiler sees
a `global x` declaration, it emits LOAD_GLOBAL instead of LOAD_NAME for all
references to `x`, cutting lookup time in half.

#### Import System

```
IMPORT_NAME      index    ->  Pop 'fromlist', pop 'level', push __import__(names[index])
IMPORT_FROM      index    ->  Push getattr(TOS, names[index])  (TOS = module)
IMPORT_STAR       -       ->  Pop TOS module, import all names into local scope
```

The import machinery works in two steps. `import os.path` compiles to:

```
LOAD_CONST    0        # fromlist = None
LOAD_CONST    None     # level = 0 (absolute import)
IMPORT_NAME   "os.path"
STORE_NAME    "os"     # bind the top-level module
```

`from os.path import join, exists` compiles to:

```
LOAD_CONST    ("join", "exists")   # fromlist
LOAD_CONST    0                    # level = 0
IMPORT_NAME   "os.path"
IMPORT_FROM   "join"
STORE_NAME    "join"
IMPORT_FROM   "exists"
STORE_NAME    "exists"
POP_TOP                            # discard the module itself
```

#### Class System

```
LOAD_BUILD_CLASS  -      ->  Push the builtins.__build_class__ function
BUILD_CLASS       -      ->  (Not a real opcode — class creation is done by
                              calling __build_class__ via CALL_FUNCTION)
```

In CPython, `class Foo(Bar):` compiles to roughly:

```
LOAD_BUILD_CLASS                     # push __build_class__
MAKE_FUNCTION    "Foo"               # push the class body as a function
LOAD_CONST       "Foo"               # push the class name
LOAD_NAME        "Bar"               # push the base class
CALL_FUNCTION    3                   # __build_class__(body_fn, "Foo", Bar)
STORE_NAME       "Foo"               # bind the resulting class object
```

This is elegant: class creation is just a function call. The class body is a
function that, when called, executes the class body statements (defining methods,
class variables) and returns a namespace dict. `__build_class__` then creates the
actual class object from that namespace, the name, and the bases.

#### Exception Handling

```
SETUP_EXCEPT     delta    ->  Push an exception handler block at IP + delta
SETUP_FINALLY    delta    ->  Push a finally handler block at IP + delta
POP_EXCEPT        -       ->  Remove the current exception handler block
END_FINALLY       -       ->  Re-raise or continue after finally block
RAISE_VARARGS    argc     ->  Raise exception (argc=0: re-raise, 1: raise x,
                               2: raise x from y)
```

Exception handling uses a *block stack* — a second stack, separate from the value
stack, that tracks active exception handlers. When code enters a `try` block,
SETUP_EXCEPT pushes a handler record onto the block stack. If an exception occurs
(during any subsequent opcode), the VM:

1. Pops frames off the block stack until it finds a matching handler.
2. Sets the instruction pointer to the handler's target address.
3. Pushes the exception onto the value stack so `except` clauses can inspect it.

This is the same mechanism used by CPython, JVM (`athrow`), and CLR (`throw`).

A worked example — `try`/`except`/`finally`:

```python
try:
    x = dangerous()
except ValueError as e:
    print(e)
finally:
    cleanup()
```

Compiles to:

```
SETUP_FINALLY    L_finally        # finally handler at L_finally
SETUP_EXCEPT     L_except         # except handler at L_except
LOAD_NAME        "dangerous"
CALL_FUNCTION    0
STORE_NAME       "x"
POP_EXCEPT                        # no exception — remove except handler
JUMP             L_else           # skip to else/finally
L_except:
  DUP_TOP                         # exception is on stack
  LOAD_NAME      "ValueError"
  COMPARE_OP     EQ               # isinstance check (simplified here)
  JUMP_IF_FALSE  L_reraise
  STORE_NAME     "e"
  LOAD_NAME      "print"
  LOAD_NAME      "e"
  CALL_FUNCTION  1
  POP_EXCEPT
  JUMP           L_finally
L_reraise:
  END_FINALLY                     # re-raise if no handler matched
L_else:
L_finally:
  LOAD_NAME      "cleanup"
  CALL_FUNCTION  0
  POP_TOP
  END_FINALLY                     # exit finally block
```

#### Context Managers (with statements)

```
SETUP_WITH           delta   ->  Call __enter__ on TOS, push handler at delta
WITH_CLEANUP_START    -      ->  Begin with-block cleanup
WITH_CLEANUP_FINISH   -      ->  Finish with-block cleanup
```

The `with open("f") as fh:` statement compiles to a sequence that calls
`__enter__` on entry, stores the result, then ensures `__exit__` is called
on both normal exit and exception. The block stack machinery (same as exceptions)
handles the guarantee.

#### Generator System

```
YIELD_VALUE           -      ->  Pause execution, yield TOS to caller
YIELD_FROM            -      ->  Delegate to sub-iterator (yield from x)
GET_YIELD_FROM_ITER   -      ->  Ensure TOS is an iterator for yield-from
```

Generators are the most architecturally significant addition. In Starlark,
function calls are simple: push a frame, run to RETURN_VALUE, pop the frame.
Generators break this model — a function can *suspend* mid-execution and
*resume* later.

This requires the VM to support **frame suspension**: when YIELD_VALUE executes,
the current frame's state (instruction pointer, value stack, local variables) is
saved into the generator object. The frame is not popped. When `__next__()` is
called, the frame is restored and execution continues from where it left off.

```python
def counter(n):
    i = 0
    while i < n:
        yield i       # <-- frame suspends here
        i += 1        # <-- frame resumes here on next()
```

This is conceptually identical to a coroutine or a green thread. The generator
object is a *saved execution context* — exactly what an OS process is to the CPU.

#### Closure Support

```
LOAD_CLOSURE     index    ->  Push cell variable for creating closures
LOAD_DEREF       index    ->  Push value from enclosing scope's cell
STORE_DEREF      index    ->  Pop TOS, store into enclosing scope's cell
DELETE_DEREF     index    ->  Delete enclosing scope's cell variable
MAKE_CLOSURE     flags    ->  Pop closure vars + code + defaults, create closure
```

Closures require *cell objects* — mutable containers shared between an enclosing
function and its nested function. When a nested function references a variable
from its enclosing scope, the compiler:

1. Allocates a cell object for that variable in the enclosing function.
2. Stores the variable's value in the cell (not directly in locals).
3. Passes the cell to the nested function via MAKE_CLOSURE.
4. The nested function accesses the variable via LOAD_DEREF / STORE_DEREF.

This allows the nested function to see mutations made by the enclosing function
(and vice versa), which is what Python's `nonlocal` keyword enables.

#### String Formatting (f-strings)

```
FORMAT_VALUE     flags    ->  Pop value (and optional fmt_spec), push formatted string
BUILD_STRING     count    ->  Pop count string fragments, concatenate, push result
```

The f-string `f"Hello, {name}!"` compiles to:

```
LOAD_CONST       "Hello, "
LOAD_NAME        "name"
FORMAT_VALUE     0               # format with default str()
LOAD_CONST       "!"
BUILD_STRING     3               # concatenate three fragments
```

#### Method Optimization

```
LOAD_METHOD      index    ->  Optimized attribute load for method calls
CALL_METHOD      argc     ->  Optimized call for loaded methods
```

LOAD_METHOD is an optimization: instead of creating a bound method object
(which allocates), it pushes the unbound method and the instance separately.
CALL_METHOD then calls the method with the instance as the first argument.
This avoids one allocation per method call — significant in hot loops.

#### Loop Control

```
SETUP_LOOP       delta    ->  Push a loop block on the block stack
BREAK_LOOP        -       ->  Exit the current loop (unwind block stack)
CONTINUE_LOOP    target   ->  Jump to target (loop start), unwinding blocks
```

Starlark's `for` loops do not need BREAK or CONTINUE because Starlark does not
have those statements. Python does. The block stack tracks active loops so that
`break` and `continue` can unwind to the correct level, even when nested inside
`try`/`except` blocks.

#### Comprehension Helpers

```
SET_ADD          offset   ->  Peek at set at stack[offset], add TOS
MAP_ADD          offset   ->  Peek at dict at stack[offset], add TOS key/value
```

These complement the existing LIST_APPEND and DICT_SET_ITEM for set and dict
comprehensions.

#### Async Support (Python 3.5+)

```
GET_AWAITABLE      -      ->  Ensure TOS is an awaitable, push coroutine
GET_AITER          -      ->  Pop TOS, push async iterator (__aiter__)
GET_ANEXT          -      ->  Push awaitable from __anext__ on TOS async iterator
SETUP_ASYNC_WITH  delta   ->  Like SETUP_WITH but for async context managers
```

Async support layers on top of generators. An `async def` function returns a
coroutine object — which is structurally identical to a generator object (saved
frame state, suspendable execution), but uses `await` instead of `yield` as the
suspension mechanism.

The event loop (which we implement as a stdlib module, not in the VM core)
drives coroutines by calling `send(None)` repeatedly until they complete.

#### Pattern Matching (Python 3.10+)

```
MATCH_CLASS      count    ->  Pop TOS class + count attr names, match instance
MATCH_MAPPING     -       ->  Check if TOS is a mapping type
MATCH_SEQUENCE    -       ->  Check if TOS is a sequence type
MATCH_KEYS         -      ->  Pop keys tuple, check if TOS mapping has all keys
COPY_DICT_WITHOUT_KEYS -  ->  Pop keys, pop dict, push dict minus those keys
```

Pattern matching compiles to a series of tests and jumps — conceptually similar
to a chain of `if`/`elif` with `isinstance` checks, but expressed as dedicated
opcodes for clarity and performance.

## Runtime Object Model

### The Core Principle: Everything is a PyObject

In Python, *everything* is an object. The integer `42` is an object. The function
`print` is an object. The class `int` is an object. Even `None` and `True` are
objects. This is not just a conceptual statement — it is a concrete implementation
decision that shapes the entire VM.

Every value on the VM stack is a `PyObject`:

```
+------------------+
|    PyObject      |
+------------------+
| type: *PyType    |  <- pointer to the type object (int, str, list, ...)
| refcount: int    |  <- reference count for garbage collection
| value: any       |  <- the actual data (language-specific representation)
+------------------+
```

The `type` pointer is the key to Python's dynamism. When the VM executes
`BINARY_ADD`, it does not assume the operands are integers. It looks up
`type(a).__add__` and calls it. This is why you can define `__add__` on your
own class and have `+` work with it — the VM dispatches through the type system.

### Type Hierarchy

```
object (root of all types)
  +-- NoneType              (singleton: None)
  +-- bool                  (True, False — subclass of int)
  +-- int                   (arbitrary precision)
  +-- float                 (IEEE 754 double)
  +-- complex               (a + bj)
  +-- str                   (Unicode text)
  +-- bytes                 (byte sequences)
  +-- list                  (mutable sequence)
  +-- tuple                 (immutable sequence)
  +-- dict                  (hash map)
  +-- set                   (mutable hash set)
  +-- frozenset             (immutable hash set)
  +-- function              (callable with code + closure)
  +-- method                (bound function + instance)
  +-- type                  (metaclass — the type of types)
  +-- module                (namespace loaded via import)
  +-- NoneType              (the type of None)
  +-- slice                 (start:stop:step)
  +-- range                 (immutable numeric sequence)
  +-- property              (descriptor for managed attributes)
  +-- classmethod           (descriptor for class-level methods)
  +-- staticmethod          (descriptor for static methods)
  +-- super                 (proxy for MRO delegation)
  +-- BaseException         (root of exception hierarchy)
       +-- Exception
            +-- ValueError
            +-- TypeError
            +-- KeyError
            +-- IndexError
            +-- AttributeError
            +-- NameError
            +-- ImportError
            +-- StopIteration
            +-- RuntimeError
            +-- ... (full hierarchy)
  +-- generator             (suspended execution frame)
  +-- coroutine             (async suspended execution frame)
```

### Magic Methods (Dunder Protocol)

Magic methods are the interface between Python syntax and the runtime. Every
operator, every syntactic construct, maps to a magic method call:

```
Python Syntax        Method Called           When
─────────────        ─────────────           ────
x + y                x.__add__(y)            BINARY_ADD opcode
x[k]                 x.__getitem__(k)        LOAD_SUBSCR opcode
len(x)               x.__len__()             builtin len()
str(x)               x.__str__()             builtin str()
repr(x)              x.__repr__()            builtin repr()
x == y               x.__eq__(y)             COMPARE_OP EQ
hash(x)              x.__hash__()            dict key lookup
iter(x)              x.__iter__()            GET_ITER opcode
next(x)              x.__next__()            FOR_ITER opcode
x.attr               x.__getattr__("attr")   LOAD_ATTR (fallback)
x.attr = v           x.__setattr__("attr",v) STORE_ATTR
with x:              x.__enter__()           SETUP_WITH
                     x.__exit__(...)         WITH_CLEANUP
x(args)              x.__call__(args)        CALL_FUNCTION
bool(x)              x.__bool__()            truth testing
```

The VM implements this by looking up the magic method on the object's type and
calling it. For built-in types (int, str, list), these lookups are optimized
to avoid the overhead of a full attribute search — the VM knows that `int.__add__`
is always integer addition and can fast-path it.

### Method Resolution Order (MRO)

When a class inherits from multiple parents, Python uses the C3 linearization
algorithm to determine the order in which parent classes are searched for methods.

```python
class A: pass
class B(A): pass
class C(A): pass
class D(B, C): pass

# MRO of D: [D, B, C, A, object]
```

The C3 algorithm ensures:
1. **Monotonicity.** If B appears before C in D's MRO, then B appears before C
   in every subclass of D's MRO.
2. **Local precedence.** The order of bases in the class definition (`class D(B, C)`)
   is respected.
3. **No cycles.** The linearization is a DAG traversal that rejects cycles.

The `super()` function uses the MRO to determine which class's method to call
next. `super()` in a method of `D` does not always call `B`'s method — it calls
the next class in the MRO, which depends on the *runtime* class of the instance.

### Descriptor Protocol

Descriptors are the mechanism behind `property`, `classmethod`, `staticmethod`,
and custom attribute access. A descriptor is any object that defines `__get__`,
`__set__`, or `__delete__`:

```python
class Property:
    def __init__(self, fget):
        self.fget = fget
    def __get__(self, obj, objtype=None):
        return self.fget(obj)
    def __set__(self, obj, value):
        raise AttributeError("read-only")
```

When the VM executes `LOAD_ATTR`, it checks:
1. Is the attribute on the instance's `__dict__`? (instance attributes)
2. Is it on the type's `__dict__` (or any parent's)? If so, is it a descriptor?
   - Data descriptor (`__get__` + `__set__`): takes priority over instance dict.
   - Non-data descriptor (`__get__` only): instance dict takes priority.

This three-level lookup is what makes Python's attribute access both flexible
and predictable.

## Class System

### Class Creation

Class creation in Python is a function call to `__build_class__`. The compiler
produces bytecode that:

1. Creates a function from the class body.
2. Calls `__build_class__(body_fn, name, *bases, **kwargs)`.
3. `__build_class__` calls the body function with a fresh namespace dict.
4. The body function executes class-level statements (method definitions, etc.).
5. `__build_class__` calls the metaclass (default: `type`) with the name, bases,
   and the populated namespace to create the class object.

### Single and Multiple Inheritance

```python
class Animal:
    def speak(self): return "..."

class Dog(Animal):
    def speak(self): return "Woof"

class GuideDog(Dog):
    pass                           # inherits Dog.speak via MRO
```

Multiple inheritance works identically — the MRO determines which `speak` is
found first.

### super() Resolution

`super()` returns a proxy that delegates attribute lookups to the next class in
the MRO. In Python 3, `super()` with no arguments uses compiler magic:
the compiler injects a `__class__` cell variable into every method, and
`super()` reads it to determine the current class.

### Descriptors in Practice

```python
class Circle:
    def __init__(self, radius):
        self._radius = radius

    @property
    def radius(self):
        return self._radius

    @radius.setter
    def radius(self, value):
        if value < 0:
            raise ValueError("radius cannot be negative")
        self._radius = value
```

The `@property` decorator creates a descriptor object. When `c.radius` is
accessed, the VM's LOAD_ATTR finds the descriptor on `Circle.__dict__["radius"]`
and calls `descriptor.__get__(c, Circle)`, which calls the getter function.

## Exception System

### The Block Stack

The VM maintains a *block stack* per frame — a stack of records that track:
- The type of block (loop, except handler, finally handler, with handler)
- The handler address (where to jump on exception or break)
- The stack depth at entry (so the value stack can be unwound)

When an exception is raised (either by `RAISE_VARARGS` or by a runtime error
in any opcode), the VM enters its exception-handling loop:

```
1. Pop the block stack.
2. If the block is an except handler:
     a. Unwind the value stack to the saved depth.
     b. Push the exception, its type, and its traceback.
     c. Jump to the handler address.
3. If the block is a finally handler:
     a. Unwind the value stack.
     b. Push the exception state.
     c. Jump to the handler address (finally runs, then re-raises).
4. If the block is a loop: skip it (loops do not handle exceptions).
5. If no more blocks: pop the current frame and propagate to the caller.
6. If no more frames: print traceback and terminate.
```

### Exception Hierarchy

All exceptions inherit from `BaseException`. User-visible exceptions inherit
from `Exception`. This split exists so that `except Exception` does not catch
`KeyboardInterrupt` or `SystemExit`, which inherit directly from `BaseException`.

### Exception Chaining (raise from)

```python
try:
    x = int("abc")
except ValueError as e:
    raise RuntimeError("conversion failed") from e
```

This compiles to `RAISE_VARARGS 2`, which creates a `RuntimeError` with its
`__cause__` attribute set to the original `ValueError`. The traceback displays
both: "The above exception was the direct cause of the following exception."

## Import System

### Module Loading Pipeline

```
import foo.bar
    |
    v
1. Check sys.modules cache            -> return cached module if found
    |
    v
2. Search sys.path for foo/bar.py     -> find the source file
   or foo/bar/__init__.py
    |
    v
3. Create a new module object         -> module with __name__, __file__
    |
    v
4. Insert into sys.modules            -> prevent circular import loops
    |
    v
5. Execute the module's source code   -> lexer -> parser -> compiler -> VM
   in the module's namespace             (recursive: the VM runs itself)
    |
    v
6. Return the module object
```

Step 5 is the key insight: importing a module means **compiling and running** it.
The VM is re-entrant — it can execute multiple frames simultaneously (not in
parallel, but nested). When `import foo` executes, the VM pushes a new frame
for `foo.py`, runs it to completion, and then returns to the importing frame.

### Circular Import Detection

Because step 4 inserts the module into `sys.modules` *before* execution completes,
circular imports do not cause infinite recursion. If module A imports module B,
and module B imports module A, module B will see A's partially-initialized module
object (containing only the names defined before the `import B` statement).

This is a well-known Python gotcha. Our VM replicates it faithfully.

### Relative Imports

```python
from . import sibling          # level=1, import from same package
from .. import uncle           # level=2, import from parent package
from .sub import thing         # level=1, import 'thing' from sub-package
```

The `level` argument to IMPORT_NAME tells the VM how many package levels to
traverse upward from the current module's `__package__` attribute.

## Generator System

### Frame Suspension

When YIELD_VALUE executes:

```
1. Save the current frame's instruction pointer.
2. Save the current frame's value stack.
3. Save the current frame's local variables.
4. Pop the frame from the frame stack (but do not discard it).
5. Push the yielded value onto the *caller's* stack.
6. Return control to the caller.
```

When `generator.__next__()` is called:

```
1. Restore the saved frame onto the frame stack.
2. Push None onto the frame's value stack (the result of the yield expression).
3. Resume execution at the saved instruction pointer.
```

`generator.send(value)` is identical to `__next__()`, except step 2 pushes
`value` instead of `None`.

### Generator Expressions

```python
squares = (x**2 for x in range(10))
```

This compiles to an implicit function containing a `yield` in a `for` loop,
then immediately creates a generator by calling that function. The result is
lazy — values are computed one at a time as the generator is iterated.

### yield from (Delegation)

```python
def chain(*iterables):
    for it in iterables:
        yield from it
```

`yield from` delegates to a sub-iterator. It is not just syntactic sugar for
`for x in it: yield x` — it also forwards `send()`, `throw()`, and `close()`
to the sub-iterator, enabling transparent coroutine delegation.

## Standard Library Strategy

### Bootstrap vs. Native

The standard library is implemented in Python source code, not in Go/TypeScript/Rust.
This is the same approach CPython takes: most of the stdlib is `.py` files that
CPython executes through itself.

Our VM loads stdlib files at startup:

```
1. VM initializes with built-in types (int, str, list, dict, ...).
2. VM loads builtins.py — defines print(), len(), range(), etc.
3. VM loads sys.py — defines sys.path, sys.modules, sys.argv.
4. VM loads importlib.py — defines the import machinery.
5. Ready to execute user code.
```

Steps 2-4 are the bootstrap. The VM must be functional enough to run Python
code before the stdlib exists — this means built-in types and a minimal set of
built-in functions are implemented natively (in Go/TS/Rust), and everything else
is layered on top in Python.

### Packages

- **`python-vm`** — the VM runtime, opcode registry, object model (Go, TS, Rust)
- **`python-stdlib`** — the standard library in Python source files (not in initial scope)
- **`python-builtins`** — native builtins: `print`, `len`, `range`, `type`, `isinstance`, etc.
  (Go, TS, Rust — these must be native because they are needed before the stdlib loads)

The Starlark stdlib (`starlark-stdlib`) continues to exist independently.
The Python stdlib is a superset.

## Implementation Phases

### Phase A: Starlark Compatibility (P0)

**Goal:** Verify that the existing Starlark infrastructure works as the base for Python.

- [ ] Run all Starlark tests through the Python VM (should pass unchanged)
- [ ] Register existing 46 opcodes with the Python VM's opcode table
- [ ] Verify frame management, function calls, iteration work correctly
- [ ] Confirm bytecode encoding matches expectations

**Exit criteria:** 100% Starlark test pass rate through the Python VM.

### Phase B: Core Python (P1)

**Goal:** Support the Python features most commonly encountered in real code.

- [ ] `while` loops (SETUP_LOOP, BREAK_LOOP, CONTINUE_LOOP)
- [ ] `class` definitions (LOAD_BUILD_CLASS, class body compilation)
- [ ] Single inheritance, `super()`, instance/class attributes
- [ ] `try`/`except`/`finally` (SETUP_EXCEPT, SETUP_FINALLY, block stack)
- [ ] `raise` and exception hierarchy (BaseException through StopIteration)
- [ ] `import` and `from...import` (IMPORT_NAME, IMPORT_FROM, module objects)
- [ ] `global` and `nonlocal` declarations (LOAD_GLOBAL, STORE_GLOBAL, cells)
- [ ] `del` statement (DELETE_NAME, DELETE_FAST, DELETE_GLOBAL)
- [ ] `assert` statement
- [ ] Multiple assignment (`a, b = 1, 2`)
- [ ] Augmented assignment (`x += 1`)
- [ ] String formatting (FORMAT_VALUE, BUILD_STRING)

**Exit criteria:** Can run a 100-line Python program with classes, exceptions, imports.

### Phase C: Advanced Python (P2)

**Goal:** Support Python's more sophisticated features.

- [ ] Generators (`yield`, `yield from`, generator objects)
- [ ] Decorators (`@decorator` syntax, function and class decorators)
- [ ] `with` statements (SETUP_WITH, context manager protocol)
- [ ] Closures and `nonlocal` (`LOAD_CLOSURE`, `LOAD_DEREF`, cell objects)
- [ ] Multiple inheritance and C3 MRO
- [ ] Full magic method dispatch (`__add__`, `__getattr__`, `__call__`, etc.)
- [ ] Descriptor protocol (`property`, `classmethod`, `staticmethod`)
- [ ] Comprehensions (list, dict, set, generator expressions)
- [ ] Lambda expressions
- [ ] `*args` and `**kwargs`
- [ ] Default and keyword-only arguments
- [ ] Method optimization (LOAD_METHOD, CALL_METHOD)

**Exit criteria:** Can run a 500-line Python program with generators, decorators,
context managers, and comprehensive use of the class system.

### Phase D: Modern Python (P3)

**Goal:** Support features added in Python 3.5+.

- [ ] `async def` and `await` (coroutine objects, GET_AWAITABLE)
- [ ] `async for` and `async with` (GET_AITER, GET_ANEXT, SETUP_ASYNC_WITH)
- [ ] `match`/`case` structural pattern matching (3.10+)
- [ ] Exception groups and `except*` (3.11+)
- [ ] Walrus operator `:=` (3.8+)
- [ ] Positional-only parameters `/` (3.8+)
- [ ] Type parameter syntax (3.12+)
- [ ] f-string improvements (3.12+, PEP 701)

**Exit criteria:** Can run CPython's test suite subsets for these features.

## Testing Strategy

### Test Sources

1. **Starlark test suite.** Bazel's own Starlark conformance tests. Must pass
   unchanged in Phase A.

2. **CPython test suite (subsets).** CPython ships with extensive tests. We will
   use `test_grammar.py` (syntax), `test_types.py` (type behavior),
   `test_builtin.py` (builtins), and feature-specific tests as conformance
   targets.

3. **Custom conformance tests.** For each opcode and each Python feature, we
   write focused tests that exercise the specific bytecode sequences. These
   tests are owned by us and serve as the primary regression suite.

### Test Organization

```
tests/
  starlark/              # Starlark conformance (Phase A)
  python/
    opcodes/             # One test file per opcode
    features/
      classes.py         # Class system tests
      exceptions.py      # Exception handling tests
      generators.py      # Generator tests
      imports.py         # Import system tests
      async.py           # Async/await tests
      match.py           # Pattern matching tests
    conformance/
      cpython/           # Adapted CPython tests
```

### Testing Methodology

Each test is structured as:

```python
# Input: Python source code
source = """
def fib(n):
    if n <= 1: return n
    return fib(n-1) + fib(n-2)
print(fib(10))
"""

# Expected: bytecode disassembly (for compiler tests)
expected_bytecode = [LOAD_CONST, MAKE_FUNCTION, STORE_NAME, ...]

# Expected: runtime output (for VM tests)
expected_output = "55\n"
```

This three-level testing (source -> bytecode -> output) catches bugs at every
stage of the pipeline.

## Language Implementations

### Go (Primary)

Go is the primary implementation because it powers the build tool. The Go
Python VM uses:
- `grammar_tools` for loading `.tokens` and `.grammar` files
- `grammar_lexer` for tokenization
- `grammar_parser` for AST construction
- `bytecode_compiler` for code generation
- `virtual_machine` (GenericVM) for execution
- `interface{}` (or generics in Go 1.18+) for the PyObject value union

### TypeScript

The TypeScript implementation powers the interactive web visualizers. It uses:
- The same package structure as Go
- `unknown` type for the PyObject value union
- Web Workers for VM execution (non-blocking UI)

### Rust

The Rust implementation targets native performance. It uses:
- `enum PyObject { Int(i64), Str(String), ... }` for tagged union values
- Trait-based dispatch for magic methods
- `Rc<RefCell<T>>` for reference-counted mutable objects (matching Python's
  reference counting semantics naturally)

All three implementations must produce identical behavior for the same input.
The test suite runs against all three and compares output.
