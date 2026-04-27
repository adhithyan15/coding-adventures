# 04g — IR to JVM Class-File Backend

## Overview

This spec defines a **generic JVM class-file backend** for the repository's
lower-level ahead-of-time IR: `compiler_ir.IrProgram`.

The goal is not "compile Java source". The goal is:

- take the existing register-based IR used by Brainfuck and Nib
- lower it into verifier-friendly JVM bytecode
- package that bytecode into a real `.class` file
- keep the emitted classes deliberately boring so they work well with
  ordinary JVMs and with GraalVM Native Image

This backend sits **below** language-specific IR compilers and **above**
class-file packaging:

```text
Brainfuck / Nib source
  -> lexer / parser / type checker
  -> *-ir-compiler
  -> compiler_ir.IrProgram
  -> ir-to-jvm-class-file         (this spec)
  -> .class bytes
  -> java / jar / GraalVM native-image
```

This spec targets the **existing lower-level `compiler-ir` IR**, not the
higher-level `SIR00` / `IR00` semantic tree. Any frontend that wants this path
must first lower into `IrProgram`.

## Why This Fits The Current IR

The existing IR already has the properties a JVM backend needs:

- linear instruction stream
- explicit labels and branches
- explicit `CALL` / `RET`
- explicit static data declarations
- finite virtual register references (`v0`, `v1`, ...)
- integer-oriented arithmetic and byte/word memory operations

That makes the backend feasible **without** reflection, dynamic class loading,
`invokedynamic`, method handles, proxies, or any other dynamic JVM features.

For the current toy-language path, that is exactly what we want:

- Brainfuck lowers naturally to byte-array memory plus simple syscalls
- Nib lowers naturally to integer registers, branches, static data, and calls
- GraalVM Native Image sees plain classes and plain bytecode

## Design Goals

1. **Generic over `IrProgram`**

   The backend must not know about Brainfuck syntax, Nib syntax, or any specific
   source language. It consumes only `IrProgram`.

2. **Class files first, Java source never**

   The backend emits real class-file bytes directly. It does not go through a
   Java source-code detour.

3. **Conservative JVM subset**

   The emitted bytecode should use only classic, stable JVM instructions:

   - integer ops
   - array loads/stores
   - branches / `goto`
   - `invokestatic`
   - `getstatic` / `putstatic`
   - ordinary `return` / `ireturn`

   No `invokedynamic`, lambdas, reflection, or hidden bootstrap magic.

4. **Graal-friendly output**

   The generated classes should be easy inputs to:

   ```text
   native-image --no-fallback ...
   ```

5. **Readable package seams**

   Low-level class-file encoding belongs in `jvm-class-file`.
   IR lowering belongs in `ir-to-jvm-class-file`.
   Language-specific wrappers belong in thin orchestration packages later.

## Package Decomposition

### 1. `jvm-class-file`

This package already exists as a minimal decoder/builder package. It should grow
into the shared class-file infrastructure layer.

Responsibilities:

- class-file constants and enums
- constant-pool entry types
- method / field / attribute models
- class-file parsing
- class-file encoding
- constant-pool builder / deduplication helpers
- small bytecode-writing helpers for method bodies

This package must stay **language-agnostic** and **IR-agnostic**.

### 2. `ir-to-jvm-class-file`

This is the new generic backend package.

Input:

- `compiler_ir.IrProgram`
- backend config (`class_name`, class-file version, main-wrapper policy, syscall policy)

Output:

- `JVMClassArtifact`
  - `class_name`
  - `class_bytes`
  - label/method metadata useful for tests and future tooling

Responsibilities:

- validate that the IR is representable by the backend
- discover callable regions
- assign data-label offsets
- lower IR ops to JVM bytecode
- ask `jvm-class-file` to package the result into a real class file

### 3. Future thin wrappers

These are explicitly **not** part of the first backend package, but they are the
natural next layer:

- `brainfuck-jvm-compiler`
- `nib-jvm-compiler`

Each wrapper should orchestrate:

```text
source -> frontend -> IrProgram -> ir-to-jvm-class-file -> .class
```

## MVP Output Shape

The backend emits **one JVM class per IR program**.

For class `HelloBF`, the generated structure is:

```text
public final class HelloBF {
  private static int[]  REGS;
  private static byte[] MEMORY;

  static { ... }                 // allocate REGS + MEMORY, initialize data image

  public static void main(String[] args) { _start(); }

  public static int _start() { ... }
  private static int _fn_main() { ... }
  private static int _fn_other() { ... }

  private static void _syscall(int number) { ... }
  private static int _loadWord(int addr) { ... }
  private static void _storeWord(int addr, int value) { ... }
}
```

This shape is intentionally simple:

- one generated class
- one static register file
- one linear memory image
- one static method per callable IR region

## Class-File Version Strategy

The MVP backend should emit an **older, verifier-friendly class-file version**
instead of targeting the newest Java release.

Default target:

- **major version 49** (Java 5 era)

Why:

- keeps the emitted format simple
- avoids needing `StackMapTable` generation in the MVP
- remains loadable on modern JVMs and modern GraalVM distributions

Future work may raise the target version once the backend can emit richer
verification metadata.

## IR Runtime Model On The JVM

### Registers

All IR virtual registers live in one generated static field:

```text
private static int[] REGS;
```

Rules:

- `REGS[n]` stores IR register `v<n>`
- all registers are 32-bit signed JVM integers
- methods communicate through this array rather than through JVM parameters

This keeps the backend generic even though `IrProgram` does not yet carry
function-signature metadata.

### Memory

All `IrDataDecl`s are packed into one generated linear memory image:

```text
private static byte[] MEMORY;
```

Rules:

- each `IrDataDecl(label, size, init)` receives a byte offset
- offsets are assigned densely in declaration order
- `LOAD_ADDR dst, label` writes that byte offset into `REGS[dst]`
- byte memory accesses operate on `MEMORY[base + offset]`

### Word layout

The JVM backend defines `LOAD_WORD` / `STORE_WORD` as:

- 32-bit signed integers
- 4-byte **little-endian** layout in `MEMORY`

This matches the current WASM backend direction and keeps word semantics stable
across compiled targets.

### Data initialization

`<clinit>` performs the generated runtime setup:

- allocate `REGS` to `max_register_index + 1`
- allocate `MEMORY` to `sum(data_decl.size)`
- initialize each declared data range to its uniform `init` byte value

Because `IrDataDecl.init` is currently one repeated byte, initialization can be
done with simple counted loops generated by the backend. No reflection or custom
resource loading is needed.

## Callable Discovery

`IrProgram` does not explicitly mark function boundaries. The backend therefore
discovers callable regions using label conventions and control-flow rules.

Callable labels are:

- `program.entry_label`
- every label referenced by `CALL`

All other labels are treated as **internal branch targets** inside the nearest
enclosing callable region.

Validation rules:

- every `CALL` target must name a callable label
- `JUMP`, `BRANCH_Z`, and `BRANCH_NZ` must stay within the current callable region
- falling through from one callable region into another is invalid
- every callable region must end in `RET` or `HALT`

For current Nib IR, this maps naturally onto `_start` and `_fn_NAME`.
For current Brainfuck IR, this maps naturally onto `_start` plus internal loop labels.

## JVM Lowering Rules

The backend lowers IR instructions into method bodies using ordinary JVM stack
machine bytecode.

### Constants and addresses

| IR op | JVM lowering |
|------|---------------|
| `LOAD_IMM dst, imm` | materialize constant, store into `REGS[dst]` |
| `LOAD_ADDR dst, label` | materialize packed byte offset for `label`, store into `REGS[dst]` |

The emitter should use the shortest suitable constant form:

- `iconst_*`
- `bipush`
- `sipush`
- `ldc`

### Byte memory

| IR op | JVM lowering |
|------|---------------|
| `LOAD_BYTE dst, base, off` | load `MEMORY[REGS[base] + REGS[off]]`, zero-extend to int, store to `REGS[dst]` |
| `STORE_BYTE src, base, off` | compute address, truncate `REGS[src]` to byte, store into `MEMORY[...]` |

`LOAD_BYTE` must zero-extend:

```text
byte b -> int (b & 0xFF)
```

### Word memory

| IR op | JVM lowering |
|------|---------------|
| `LOAD_WORD dst, base, off` | call generated `_loadWord(addr)` helper |
| `STORE_WORD src, base, off` | call generated `_storeWord(addr, value)` helper |

Helpers keep the main lowering path simple and centralize little-endian logic.

### Arithmetic and bitwise ops

| IR op | JVM lowering |
|------|---------------|
| `ADD dst, lhs, rhs` | `iadd` |
| `ADD_IMM dst, src, imm` | load register + constant, `iadd` |
| `SUB dst, lhs, rhs` | `isub` |
| `AND dst, lhs, rhs` | `iand` |
| `AND_IMM dst, src, imm` | load register + constant, `iand` |

### Comparisons

Comparisons must materialize explicit `0` / `1` integer results because the IR
models booleans as ordinary integer registers.

| IR op | Result |
|------|--------|
| `CMP_EQ` | `REGS[dst] = 1` if equal else `0` |
| `CMP_NE` | `REGS[dst] = 1` if not equal else `0` |
| `CMP_LT` | signed less-than |
| `CMP_GT` | signed greater-than |

Lowering pattern:

```text
if_icmp<cond> L_true
iconst_0
goto L_done
L_true:
iconst_1
L_done:
...store into REGS[dst]...
```

### Labels and branches

| IR op | JVM lowering |
|------|---------------|
| `LABEL name` | bytecode label |
| `JUMP label` | `goto` |
| `BRANCH_Z reg, label` | load `REGS[reg]`, `ifeq label` |
| `BRANCH_NZ reg, label` | load `REGS[reg]`, `ifne label` |

Unlike WASM, JVM bytecode supports arbitrary reducible and non-reducible jumps,
so the backend does **not** need CFG restructuring for the MVP.

### Calls and returns

The backend ABI is:

- callables are generated as `static int method()`
- arguments already live in `REGS[v2]`, `REGS[v3]`, ...
- return value is the integer returned from the method and also conceptually
  lives in `REGS[v1]`

| IR op | JVM lowering |
|------|---------------|
| `CALL label` | `invokestatic Generated.label()I`, then store returned int into `REGS[1]` |
| `RET` | load `REGS[1]`, `ireturn` |

This preserves the current Nib calling convention without needing parameter
descriptors derived from separate signature metadata.

### Halt and meta ops

| IR op | JVM lowering |
|------|---------------|
| `HALT` | in `_start`, return `REGS[1]`; invalid elsewhere in MVP |
| `NOP` | `nop` |
| `COMMENT` | no emitted bytecode |

## Syscall Policy

The backend supports only the syscall subset used by the current frontends.

Generated helper:

```text
private static void _syscall(int number)
```

MVP syscall table:

- `1` — write one byte from `REGS[4] & 0xFF` to `System.out`
- `2` — read one byte from `System.in`, write result to `REGS[4]`
- `10` — treated as program exit / halt helper

Initial EOF rule for syscall `2`:

- EOF becomes `0`

Any other syscall number is a backend/runtime error.

This is enough for Brainfuck I/O and leaves room for future Nib runtime helpers.

## Validation Rules

Before code generation, the backend must validate:

- entry label exists
- all branch and call labels exist
- all branch targets stay within the current callable
- all `CALL` targets are callable labels
- `RET` appears only inside callable regions
- `HALT` appears only in the entry callable in MVP
- register indices are non-negative
- data declaration sizes are non-negative
- overlapping data labels are impossible because the backend owns packing

Any invalid program must fail fast with a backend-specific error type rather than
emitting malformed class files.

## Public API

The new package should expose a small API surface:

```python
@dataclass(frozen=True)
class JvmBackendConfig:
    class_name: str
    class_file_major: int = 49
    class_file_minor: int = 0
    emit_main_wrapper: bool = True

@dataclass(frozen=True)
class JVMClassArtifact:
    class_name: str
    class_bytes: bytes
    callable_labels: tuple[str, ...]
    data_offsets: dict[str, int]

def lower_ir_to_jvm_class_file(
    program: IrProgram,
    config: JvmBackendConfig,
) -> JVMClassArtifact: ...
```

The `jvm-class-file` package should also expose general encoding primitives,
not just `build_minimal_class_file()`.

## Relationship To Existing JVM Packages

This backend should align with the existing repository direction:

- `jvm-class-file` is the reusable class-file seam
- `jvm-simulator` stays execution-oriented
- `ir-to-jvm-class-file` becomes the generic compiler backend

That means the generated classes should be:

- parseable by `jvm-class-file`
- eventually executable through a more complete JVM simulator stack
- directly consumable by real JVMs and GraalVM

## Test Strategy

Every package in this path must exceed 80% coverage, with backend packages
targeting 95%+ where practical.

Required tests:

1. **Class-file structure tests**

   Generate a class, parse it back through `jvm-class-file`, and verify:

   - class name
   - version
   - methods present
   - descriptors
   - `Code` attributes exist

2. **Opcode lowering tests**

   One focused test per supported IR opcode family:

   - constants
   - memory
   - arithmetic
   - comparisons
   - branches
   - calls
   - syscalls

3. **Fixture tests from real IR**

   Compile real frontend-produced IR:

   - Brainfuck: empty program, `+`, loop, cat, hello-world
   - Nib: arithmetic, `if`, `for`, function call, static data access

4. **Execution smoke tests**

   Run generated classes on a real JVM in CI where available.

5. **Graal smoke tests**

   Future CI job:

   ```text
   IrProgram -> .class -> native-image -> executable
   ```

   At minimum, one Brainfuck and one Nib fixture should complete successfully.

## Future Extensions

- replace global `REGS` with per-method JVM locals where signature metadata exists
- add precise method descriptors instead of the uniform `()I` callable ABI
- emit JAR files and manifests
- support richer syscalls / host runtime helpers
- generate debug metadata (`LineNumberTable`, local-variable info)
- raise class-file target version and emit `StackMapTable`
- add an optimization pass that lowers common register-array traffic into local-slot caching
- add language-specific wrapper packages such as `brainfuck-jvm-compiler` and `nib-jvm-compiler`

## Bottom Line

Yes: the existing `compiler-ir` is sufficient for a real JVM class-file backend.

The clean MVP is:

- generic `IrProgram` input
- one generated class
- static `REGS`
- static packed `MEMORY`
- one static method per callable label
- conservative, Graal-friendly bytecode

That gives the repository a true JVM compilation path without introducing any
dynamic JVM features that would work against the statically-compiled direction.
