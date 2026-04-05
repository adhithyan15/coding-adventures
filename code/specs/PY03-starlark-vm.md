# PY03 — Starlark VM: The Python VM in Restricted Mode

## 1. Overview

Starlark is a deterministic dialect of Python designed by Google for BUILD file
configuration. It looks like Python, reads like Python, but cannot do anything
dangerous: no file I/O, no network access, no infinite loops, no exceptions.
Every Starlark program terminates, and running it twice with the same inputs
always produces the same outputs.

Our Starlark VM is **not a separate codebase**. It is the Python VM (spec PY02)
running with restriction flags enabled. The same lexer, parser, compiler, and
virtual machine serve both Python and Starlark. A `mode` parameter at each stage
controls which features are allowed.

This is a deliberate architectural choice. Maintaining two separate pipelines
(one for Python, one for Starlark) means duplicating bug fixes, duplicating
tests, and diverging semantics. Since Starlark is a strict subset of Python,
the right design is one pipeline with a restriction layer.

```
                          +-----------+
                          |  mode:    |
                          | "python"  |    Full Python — all features enabled
                          | "starlark"|    Restricted — subset only
                          +-----------+
                               |
                               v
  Source Code --> [ Lexer ] --> [ Parser ] --> [ Compiler ] --> [ VM ]
                   |              |               |              |
                   | rejects      | validates     | omits        | enforces
                   | banned       | AST against   | restricted   | freezing,
                   | tokens       | restrictions  | opcodes      | determinism
```

### What This Spec Replaces

The existing `starlark-vm` packages in this monorepo were built on a separate
Starlark-specific lexer/parser/compiler pipeline. This spec replaces that
approach. The new Starlark VM reuses the Python pipeline with restrictions,
reducing code duplication and ensuring Starlark semantics stay aligned with
Python semantics.

The public API surface remains the same: `Interpret`, `InterpretFile`,
`WithGlobals`, `WithFileResolver`. Callers do not need to change.


## 2. Relationship to the Python VM

Think of it like a car with a "sport mode" and an "eco mode." The engine is the
same — what changes is which capabilities are enabled. Here is how each stage
of the Python VM behaves when `mode = "starlark"`:

### Stage 1: Lexer (Token Filtering)

The lexer produces tokens from source text. In starlark mode, it **rejects
tokens** for keywords that Starlark does not allow. These are valid Python
keywords, but using them in a `.star` file is an error:

```
Rejected keywords:
  class    import    while     try       except
  finally  raise     with      yield     async
  await    global    nonlocal  del       assert
  from     as        is
```

When the lexer encounters one of these, it emits an error:

```
Error at line 5, col 1: 'class' is not allowed in Starlark.
  Starlark does not support user-defined types.
  Hint: use dicts or named function arguments instead.
```

Each rejected keyword gets a specific, helpful error message explaining why it
was removed and what to use instead. This is critical for the educational
experience — a user coming from Python should understand *why* the restriction
exists, not just that it exists.

### Stage 2: Parser (AST Validation)

The parser builds an Abstract Syntax Tree. In starlark mode, it **validates
the AST** against Starlark's structural rules. Some constructs use allowed
tokens but combine them in ways Starlark forbids:

| Rejected Pattern           | Why                                          |
|----------------------------|----------------------------------------------|
| Top-level `if` statement   | BUILD files should be declarative             |
| Top-level `for` loop       | BUILD files should be declarative             |
| Recursion (self-reference) | Guaranteed termination                        |
| `*` in load() arguments    | All imports must be explicit                  |
| Nested `def` inside `def`  | Keeps scope rules simple (debatable, see note)|
| Set literals `{1, 2, 3}`   | Not in Starlark spec (use list or tuple)      |

**A note on top-level restrictions.** In Starlark, `if` and `for` are only
allowed inside function bodies. At the top level of a `.star` file, you can
only have: assignments, function definitions, `load()` calls, and expressions.
This keeps BUILD files readable as declarations:

```python
# ALLOWED at top level:
name = "my_library"                          # assignment
load("//tools:rules.star", "py_library")     # load
py_library(name = name, srcs = ["lib.py"])   # expression (function call)

def helper(x):        # function definition
    if x > 0:         # if is fine INSIDE a function
        return x
    return -x

# NOT ALLOWED at top level:
if condition:          # ERROR: top-level if
    do_something()
for item in items:     # ERROR: top-level for
    process(item)
```

### Stage 3: Compiler (Opcode Restriction)

The compiler translates the AST into bytecode. In starlark mode, it **omits
opcodes** that correspond to removed features:

```
Omitted opcodes:
  BUILD_CLASS      — no class definitions
  IMPORT_NAME      — no import statements
  IMPORT_FROM      — no import statements
  SETUP_EXCEPT     — no try/except
  SETUP_FINALLY    — no try/finally
  SETUP_WITH       — no with statements
  SETUP_LOOP       — no while loops (for-loops use FOR_ITER)
  YIELD_VALUE      — no generators
  YIELD_FROM       — no generator delegation
  DELETE_NAME      — no del statement
  DELETE_ATTR      — no del statement
  DELETE_SUBSCR    — no del statement
```

If the compiler encounters an AST node that would require one of these opcodes,
it raises a compile-time error. This is a second line of defense — the parser
should have caught it already, but defense in depth matters.

### Stage 4: VM (Runtime Enforcement)

The VM executes bytecode. In starlark mode, it adds three behaviors:

1. **Freezing** (see Section 5)
2. **Recursion depth limiting** — max depth of 0 (no recursion at all)
3. **Deterministic built-ins only** (see Section 7)


## 3. What Starlark Keeps from Python

Starlark retains a substantial subset of Python. Here is the complete list
of kept features, organized by category:

### Values and Types

```python
# All primitive types
x = 42                     # int (arbitrary precision)
y = 3.14                   # float (IEEE 754 double)
s = "hello"                # string (Unicode)
b = True                   # bool
n = None                   # None

# Collections
items = [1, 2, 3]          # list (mutable until frozen)
pair = (1, 2)              # tuple (always immutable)
table = {"a": 1, "b": 2}  # dict (mutable until frozen, insertion-ordered)
```

### Operators

```python
# Arithmetic: +  -  *  /  //  %  **
# Comparison: ==  !=  <  >  <=  >=
# Boolean:    and  or  not
# Bitwise:    &  |  ^  ~  <<  >>
# Membership: in  not in
# Augmented:  +=  -=  *=  /=  //=  %=  &=  |=  ^=  <<=  >>=

# Comparison chaining works:
if 0 <= x < 10:
    print("single digit")
```

### Functions

```python
# Regular functions
def greet(name, greeting="hello"):
    return "{} {}!".format(greeting, name)

# *args and **kwargs
def flexible(*args, **kwargs):
    return (args, kwargs)

# Lambda expressions
double = lambda x: x * 2

# Multiple return values
def swap(a, b):
    return b, a

# Closures
def make_adder(n):
    return lambda x: x + n
```

### Control Flow (Inside Functions Only)

```python
def classify(x):
    # if/elif/else
    if x > 0:
        label = "positive"
    elif x < 0:
        label = "negative"
    else:
        label = "zero"

    # for loops over finite collections
    results = []
    for item in [1, 2, 3]:
        results.append(item * 2)

    # break and continue
    for i in range(100):
        if i % 2 == 0:
            continue
        if i > 10:
            break
        results.append(i)

    return label, results
```

### Comprehensions

```python
# List comprehension
squares = [x * x for x in range(10)]

# Dict comprehension
inverted = {v: k for k, v in original.items()}

# Nested comprehension
flat = [cell for row in matrix for cell in row]

# Filtered comprehension
evens = [x for x in numbers if x % 2 == 0]
```

### String Operations

```python
s = "hello world"
s.upper()                  # "HELLO WORLD"
s.split(" ")               # ["hello", "world"]
s.startswith("hello")      # True
s.replace("world", "all") # "hello all"
"{}={}".format("a", 1)    # "a=1"
s[0:5]                     # "hello" (slicing)
```


## 4. What Starlark Removes from Python

Every removed feature has a specific rationale tied to one of three goals:
**termination** (programs always finish), **determinism** (same result every
time), or **simplicity** (configuration should be easy to understand).

### Removed for Termination

| Feature         | Problem                                    | Alternative           |
|-----------------|--------------------------------------------|-----------------------|
| `while` loops   | Can loop forever: `while True: pass`       | Use `for` over finite |
| Recursion       | Can overflow stack: `f(f(x))`              | Use iteration         |

### Removed for Determinism

| Feature             | Problem                                        | Alternative           |
|---------------------|------------------------------------------------|-----------------------|
| `import`            | File system access, module search paths vary   | Use `load()`          |
| `global`/`nonlocal` | Mutable global state breaks parallelism        | Pass values as args   |
| `is` / `is not`     | Identity depends on object allocation          | Use `==` / `!=`       |
| `del`               | Affects iteration order, gc-dependent behavior | Just stop referencing |
| `eval()`/`exec()`   | Dynamic code defeats static analysis           | Not available         |

### Removed for Simplicity

| Feature          | Problem                                     | Alternative               |
|------------------|---------------------------------------------|---------------------------|
| `class`          | OOP is overkill for config files            | Use dicts, structs        |
| `try`/`except`   | Errors should halt, not be hidden           | Use `fail()` for errors   |
| `with`           | No resources to manage (no files, no locks) | Not needed                |
| `yield`          | Generators add lazy evaluation complexity   | Use list comprehensions   |
| `async`/`await`  | No concurrency in config evaluation         | Not needed                |
| Top-level `if`   | Config files should be declarative          | Move logic into functions |
| Top-level `for`  | Config files should be declarative          | Use comprehensions or def |


## 5. Freezing (Starlark-Specific)

Freezing is the most important feature Starlark adds that Python does not have.
After a `.star` file finishes executing, **every global value becomes permanently
immutable**. This is not optional — it is automatic and irreversible.

### Why Freezing Exists

Build systems evaluate many `.star` files, often in parallel. If module A loads
a list from module B and then mutates it, module C (which also loaded from B)
sees the mutation. This is a data race. Freezing eliminates it entirely: once
a module is done, its exports cannot change.

```python
# file: constants.star
COLORS = ["red", "green", "blue"]     # mutable during execution

def get_colors():
    COLORS.append("yellow")           # OK — file is still executing
    return COLORS

ALL_COLORS = get_colors()             # ["red", "green", "blue", "yellow"]

# --- file execution ends here ---
# COLORS is now frozen. ALL_COLORS is now frozen.

# In another file that does load("constants.star", "COLORS"):
# COLORS.append("purple")  --> ERROR: cannot mutate frozen list
```

### How Freezing Works

Every mutable object (list, dict) carries a boolean `frozen` flag, initially
`False`. When a module finishes executing:

1. Walk all global variables
2. For each value, recursively set `frozen = True`
3. Nested structures are frozen transitively (a list inside a dict is also frozen)

On every mutation operation (`append`, `__setitem__`, `pop`, `update`, etc.),
the VM checks the `frozen` flag first:

```python
def list_append(self, value):
    if self.frozen:
        raise EvalError("cannot append to frozen list")
    self.items.append(value)
```

### What Gets Frozen

| Type   | Frozen behavior                                    |
|--------|----------------------------------------------------|
| `list` | No append, insert, pop, sort, reverse, or __setitem__ |
| `dict` | No __setitem__, pop, update, setdefault, or clear  |
| `int`, `float`, `string`, `bool`, `None` | Already immutable — no change |
| `tuple` | Already immutable — but contained mutables ARE frozen |
| Functions | Cannot be redefined (global rebinding is blocked) |


## 6. The load() Function

Starlark replaces Python's `import` with `load()`, a more restricted module
system designed for build files.

### Syntax

```python
load("//path/to/file.star", "symbol1", "symbol2")
load("//path/to/file.star", alias1 = "symbol1")
load("//path/to/file.star", "symbol1", alias2 = "symbol2")
```

- **First argument**: a string label identifying the file to load
- **Positional arguments**: names of symbols to import (bound to the same name)
- **Keyword arguments**: aliases (the key becomes the local name, the value is
  the name in the loaded file)

### Semantics

```python
# In //tools/rules.star:
_internal_helper = lambda x: x + 1    # private (starts with _)
py_library = lambda **kw: kw           # public

# In //packages/my_lib/BUILD.star:
load("//tools/rules.star", "py_library")          # OK
load("//tools/rules.star", "_internal_helper")     # ERROR: private symbol
load("//tools/rules.star", pl = "py_library")      # OK: pl is alias
```

### Key Rules

1. **Files are loaded once and cached.** If three files all load the same
   module, the module executes exactly once. Subsequent loads return the
   cached (frozen) namespace.

2. **Circular dependencies are rejected.** If A loads B and B loads A, the
   VM detects the cycle and reports an error with the full dependency chain:
   ```
   Error: circular load detected:
     //a.star loads //b.star
     //b.star loads //a.star
   ```

3. **Private names cannot be loaded.** Any symbol starting with `_` is
   private to its module. Attempting to load it is an error.

4. **Loaded values are frozen.** Since loaded modules have finished executing,
   all their exports are already frozen (see Section 5).

5. **load() is only valid at the top level.** You cannot call load() inside
   a function body.


## 7. Built-in Functions

The Starlark VM provides a curated set of built-in functions. These are a
subset of Python's built-ins, chosen for safety and utility in configuration.

### Type Constructors

```python
bool(0)          # False
dict(a=1, b=2)   # {"a": 1, "b": 2}
float("3.14")    # 3.14
int("42")        # 42
list((1, 2, 3))  # [1, 2, 3]
str(42)          # "42"
tuple([1, 2])    # (1, 2)
```

### Type Inspection

```python
type(42)                # "int"
type("hello")           # "string"
hasattr(obj, "name")    # True/False
getattr(obj, "name")    # value of obj.name
```

### Sequence Operations

```python
len([1, 2, 3])                    # 3
range(5)                          # [0, 1, 2, 3, 4]
range(1, 10, 2)                   # [1, 3, 5, 7, 9]
sorted([3, 1, 2])                 # [1, 2, 3]
sorted(items, key=lambda x: x.name)
reversed([1, 2, 3])               # [3, 2, 1]
enumerate(["a", "b"])             # [(0, "a"), (1, "b")]
zip([1, 2], ["a", "b"])           # [(1, "a"), (2, "b")]
```

### Aggregation

```python
all([True, True, False])   # False
any([False, False, True])  # True
min(3, 1, 2)               # 1
max(3, 1, 2)               # 3
sum([1, 2, 3])             # 6
```

### Functional

```python
list(map(str, [1, 2, 3]))           # ["1", "2", "3"]
list(filter(lambda x: x > 0, xs))   # positive values only
```

### String Utilities

```python
chr(65)       # "A"
ord("A")      # 65
repr([1, 2])  # "[1, 2]"
hash("key")   # deterministic integer hash
```

### Output and Errors

```python
print("debug info")     # writes to build log only (not stdout)
fail("something broke") # aborts execution with error message and stack trace
```

**Important:** `print()` in Starlark is a debugging tool. It writes to the
build log, not to stdout. It cannot be used for program output because
Starlark programs do not have "output" in the traditional sense — they
produce a build graph.


## 8. Determinism Guarantees

Starlark makes strong determinism guarantees. Running the same `.star` file
with the same inputs must produce bit-identical results every time, on every
platform. Here is how each source of nondeterminism is eliminated:

| Source of Nondeterminism | How Starlark Eliminates It            |
|--------------------------|---------------------------------------|
| Dict iteration order     | Guaranteed insertion order            |
| Set iteration order      | Sets are not in the language          |
| Hash randomization       | `hash()` returns deterministic values |
| Random numbers           | No random module                      |
| Current time             | No time module                        |
| Environment variables    | No os.environ access                  |
| File system              | No open(), no os.path                 |
| Network                  | No socket, no urllib                  |
| Object identity (`id()`) | No id() function, no `is` operator   |
| Garbage collection order | No finalizers, no __del__             |
| Float formatting          | IEEE 754 rules, repr is canonical    |
| String hashing           | Deterministic hash, not randomized   |

### Why This Matters for Build Systems

If a BUILD file produces different results on different machines (or different
runs), the build system cannot cache anything. Caching depends on the guarantee
that: "if the inputs haven't changed, the outputs haven't changed." Starlark's
determinism makes caching safe.


## 9. Standard Library (Minimal)

The Starlark VM ships with a tiny standard library, written in Python and
loaded at startup. This library provides functionality that Starlark programs
commonly need but that is not part of the core language.

### builtins.py

Contains the built-in functions described in Section 7. These are injected
into every module's global scope before execution begins.

### json.py

Provides `json.loads()` and `json.dumps()` for structured data exchange.
BUILD files sometimes need to read JSON configuration:

```python
load("@stdlib//json.star", "json")

config = json.loads('{"debug": true, "level": 3}')
print(config["debug"])   # True
```

Implementation is straightforward: parse JSON into Starlark dicts, lists,
strings, numbers, booleans, and None.

### path.py

Provides path manipulation for `load()` label resolution:

```python
load("@stdlib//path.star", "path")

path.join("a", "b", "c")     # "a/b/c"
path.dirname("a/b/c.star")   # "a/b"
path.basename("a/b/c.star")  # "c.star"
```

These operate on label strings, not filesystem paths. There is no actual
filesystem access.


## 10. Error Handling

Starlark has no `try`/`except`. Every error is fatal. This is intentional:
build configuration errors should not be silently swallowed.

### Error Categories

```
TypeError:     wrong type passed to operation
  "cannot add string to int"

KeyError:      dict key not found
  "key 'missing' not found in dict"

IndexError:    list index out of bounds
  "index 5 out of range for list of length 3"

ValueError:    right type but wrong value
  "invalid base for int(): 'abc'"

FrozenError:   attempted mutation of frozen value
  "cannot append to frozen list"

LoadError:     problem with load() statement
  "circular load detected: //a.star -> //b.star -> //a.star"

EvalError:     general evaluation failure
  "fail() called: 'missing required attribute: name'"
```

### Stack Traces

Every error includes a full stack trace showing the chain of function calls:

```
Error: KeyError: key 'missing' not found in dict
  at //tools/rules.star:15:8 in validate_target()
  at //packages/my_lib/BUILD.star:7:1 in <toplevel>
  loaded from //packages/my_lib/BUILD.star
```

### The fail() Function

`fail()` is Starlark's only mechanism for signaling errors. Rule authors
use it to validate inputs:

```python
def py_library(name, srcs, deps = []):
    if not name:
        fail("py_library requires a 'name' attribute")
    if not srcs:
        fail("py_library requires at least one source file in 'srcs'")
    # ... build the target
```


## 11. Testing Strategy

Testing the Starlark VM requires verifying both what it **accepts** and what
it **rejects**. The rejection tests are just as important as the acceptance
tests — a Starlark VM that accidentally allows `while` loops or `import`
statements is broken, even if everything else works.

### Test Categories

**1. Feature acceptance tests** — Verify that allowed features work correctly:

```python
# test_starlark_functions.py
def test_function_definition():
    result = starlark_eval("""
def add(a, b):
    return a + b
result = add(3, 4)
""")
    assert result["result"] == 7
```

**2. Feature rejection tests** — Verify that banned features produce clear errors:

```python
# test_starlark_restrictions.py
def test_rejects_while_loop():
    with pytest.raises(StarlarkError, match="while.*not allowed"):
        starlark_eval("while True: pass")

def test_rejects_class():
    with pytest.raises(StarlarkError, match="class.*not allowed"):
        starlark_eval("class Foo: pass")

def test_rejects_import():
    with pytest.raises(StarlarkError, match="import.*not allowed"):
        starlark_eval("import os")
```

**3. Freezing tests** — Verify that mutation after execution is blocked:

```python
def test_frozen_list():
    vm = StarlarkVM()
    vm.exec_file("constants.star")  # defines ITEMS = [1, 2, 3]
    # After execution, ITEMS is frozen
    with pytest.raises(FrozenError):
        vm.globals["ITEMS"].append(4)
```

**4. load() tests** — Verify module loading semantics:

```python
def test_load_caching():
    """Loading the same file twice returns the same (frozen) namespace."""
    vm = StarlarkVM()
    ns1 = vm.load("//a.star")
    ns2 = vm.load("//a.star")
    assert ns1 is ns2

def test_load_cycle_detection():
    """Circular loads produce a clear error."""
    # a.star: load("//b.star", "x")
    # b.star: load("//a.star", "y")
    with pytest.raises(LoadError, match="circular"):
        vm.load("//a.star")
```

**5. Determinism tests** — Verify identical results across runs:

```python
def test_deterministic_execution():
    """Same input produces bit-identical output."""
    result1 = starlark_eval_file("complex_build.star")
    result2 = starlark_eval_file("complex_build.star")
    assert result1 == result2
```

**6. Bazel compatibility tests** — Run against the official Starlark test suite
from `github.com/bazelbuild/starlark/tree/master/testdata` to verify conformance
with the spec.


## 12. Migration from Existing starlark-vm

The existing `starlark-vm` packages use a dedicated Starlark-specific pipeline:

```
Old architecture (being replaced):
  Starlark Lexer --> Starlark Parser --> Starlark Compiler --> Starlark VM
  (separate)        (separate)          (separate)            (separate)
```

The new architecture collapses this into mode flags on the Python pipeline:

```
New architecture:
  Python Lexer       --> Python Parser     --> Python Compiler  --> Python VM
  (mode: "starlark")    (mode: "starlark")    (mode: "starlark")   (mode: "starlark")
```

### Migration Steps

1. Add `mode` parameter to each Python VM stage (lexer, parser, compiler, VM)
2. Implement restriction logic in each stage (as described in Section 2)
3. Implement freezing (Section 5) and load() (Section 6)
4. Wire up the Starlark standard library (Section 9)
5. Verify against existing starlark-vm test suite
6. Verify against Bazel's official Starlark test suite
7. Update BUILD files to use new package paths
8. Remove old starlark-vm packages

### API Compatibility

The public API remains unchanged:

```python
# Old (separate starlark-vm package):
from starlark_vm import StarlarkVM
vm = StarlarkVM()
vm.exec_file("BUILD.star")

# New (Python VM in starlark mode):
from python_vm import PythonVM
vm = PythonVM(mode="starlark")
vm.exec_file("BUILD.star")

# Convenience alias (preserves old import path):
from starlark_vm import StarlarkVM  # wraps PythonVM(mode="starlark")
```

### What Changes Internally

| Component       | Old                          | New                              |
|-----------------|------------------------------|----------------------------------|
| Lexer           | Starlark-specific tokenizer  | Python lexer with token filter   |
| Parser          | Starlark-specific grammar    | Python parser with AST validator |
| Compiler        | Starlark-specific bytecode   | Python compiler with opcode mask |
| VM              | Starlark-specific runtime    | Python VM with freeze + load()   |
| Standard lib    | Baked into VM                | Separate .py files loaded at init|
| Test suite      | Starlark-only tests          | Python tests + Starlark subset   |


## 13. Implementation Order

The implementation follows the pipeline, building restriction support from
front to back:

1. **Lexer restrictions** — Add token rejection for banned keywords. This is
   the simplest change: a lookup table of rejected token types.

2. **Parser restrictions** — Add AST validation pass. After parsing succeeds,
   walk the tree and reject banned constructs (top-level if/for, class nodes,
   import nodes, etc.).

3. **Compiler restrictions** — Add opcode filtering. If an AST node would emit
   a banned opcode, raise a compile error.

4. **VM freezing** — Add the `frozen` flag to mutable objects and check it on
   every mutation operation. Add the post-execution freeze walk.

5. **load() implementation** — Add the module loading system with caching,
   cycle detection, and private name enforcement.

6. **Standard library** — Write builtins.py, json.py, and path.py in Python.
   Wire them into VM startup.

7. **Error messages** — Polish every error message to be educational. Each
   error should explain what went wrong, why the restriction exists, and what
   to do instead.

8. **Test suite** — Write comprehensive tests for all of the above, plus run
   the Bazel conformance suite.

Each step is independently testable and committable. The restriction at each
stage provides defense in depth: even if one stage misses a banned construct,
the next stage catches it.
