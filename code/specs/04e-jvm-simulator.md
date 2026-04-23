# 04e - JVM Simulator

## Overview

The Java Virtual Machine is a stack-based virtual machine defined by the Java
Virtual Machine Specification (JVMS). As of April 16, 2026, the latest Java SE
specification is Java SE 26, released in March 2026, and the corresponding
class file major version is 70.

For this repo, the right long-term goal is not a single monolithic
`jvm-simulator` package. The reusable seam is the class file pipeline:

```text
class bytes -> class file decoder -> constant pool / attribute decoders
            -> method bytecode decoder -> disassembler -> runtime model
            -> simulator
```

That mirrors the BEAM direction and makes the lower layers reusable for future
tools such as:

- class file inspectors
- bytecode validators
- compiler backends
- future JIT or ahead-of-time experiments
- cross-language ports

The existing `jvm-simulator` package should become a thin facade over smaller
packages rather than the place where all logic lives.

## Where it fits

```text
Source -> Lexer -> Parser -> JVM compiler -> class file / bytecode pipeline -> JVM simulator
```

The simulator is only the last stage. Most JVM complexity lives before
execution: parsing class files, decoding the constant pool, understanding
attributes, decoding method bodies, and resolving symbolic references.

## Core research findings

### 1. Versioning is mostly a class file problem, not an opcode-table problem

The JVM instruction set is much more stable than BEAM's. The moving parts
across Java releases are mostly:

- class file major/minor versions
- constant pool tags
- attributes
- verification rules
- linkage rules
- newer language features lowered onto old or mostly-stable opcodes

Inference from JVMS editions: the biggest post-1.0 execution-level addition is
`invokedynamic`; most newer Java features show up as new constant-pool entries
and attributes rather than large opcode-table churn.

### 2. A JVM simulator should target `.class` files and `Code` attributes

For educational traces we may expose a raw-bytecode entry point, but the real
input unit is a class file:

- magic: `0xCAFEBABE`
- minor / major version
- constant pool
- fields
- methods
- attributes

Each executable method body comes from a `Code` attribute containing:

- `max_stack`
- `max_locals`
- `code_length`
- instruction bytes
- exception table
- nested attributes

### 3. Public APIs should accept exact Java targets, but implementation should use profiles

We should support exact targets such as:

- `java_1_0_2`
- `java_8`
- `java_17`
- `java_21`
- `java_26`

Internally these resolve to immutable `JvmProfile` values that describe the
actual decoding and runtime rules.

## Exact version strategy

### Public target names

The public interface should accept exact release identifiers, not just raw major
numbers:

```python
simulate_class_file(data, target="java_17")
decode_class_file(data, target="java_1_4")
```

### Internal profile model

```python
@dataclass(frozen=True)
class JvmProfile:
    target: str
    classfile_major: int
    classfile_minor_mode: Literal["legacy", "preview_capable"]
    constant_pool_tags: frozenset[str]
    attribute_kinds: frozenset[str]
    verifier_mode: Literal["legacy", "stack_map_transition", "modern"]
    invocation_mode: Literal["pre_indy", "indy_capable"]
```

The target string is for user intent. The profile is what the decoder and
simulator actually use.

### Class file version map

From Oracle's JVMS version table, the exact Java release should map as follows:

| Java release | Class file major |
|--------------|------------------|
| 1.0.2 / 1.1 | 45 |
| 1.2 | 46 |
| 1.3 | 47 |
| 1.4 | 48 |
| 5.0 | 49 |
| 6 | 50 |
| 7 | 51 |
| 8 | 52 |
| 9 | 53 |
| 10 | 54 |
| 11 | 55 |
| 12 | 56 |
| 13 | 57 |
| 14 | 58 |
| 15 | 59 |
| 16 | 60 |
| 17 | 61 |
| 18 | 62 |
| 19 | 63 |
| 20 | 64 |
| 21 | 65 |
| 22 | 66 |
| 23 | 67 |
| 24 | 68 |
| 25 | 69 |
| 26 | 70 |

### Compatibility buckets

Exact targets should still collapse onto a smaller number of implementation
buckets:

- `legacy_45_48`: Java 1.0.2 through 1.4
- `classic_49_50`: Java 5 and 6
- `indy_transition_51_52`: Java 7 and 8
- `module_transition_53_55`: Java 9 through 11
- `modern_56_61`: Java 12 through 17
- `current_62_70`: Java 18 through 26

This keeps the public API exact while avoiding an explosion of duplicated logic.

## Proposed package structure

### Python-first package split

| Package | Responsibility | Primary output |
|---------|----------------|----------------|
| `jvm-version-profiles` | Exact-version lookup and compatibility buckets | `JvmProfile` |
| `jvm-classfile-decoder` | Parse `CAFEBABE` container and top-level structure | `DecodedClassFile` |
| `jvm-constant-pool-decoder` | Decode `cp_info` entries by tag and profile | `DecodedConstantPool` |
| `jvm-attribute-decoder` | Decode class, field, method, and code attributes | `DecodedAttribute` values |
| `jvm-bytecode-decoder` | Decode method `Code` bytes into instruction records | `JvmInstruction` list |
| `jvm-bytecode-disassembler` | Pretty-print and label decoded bytecode | `JvmDisassembly` |
| `jvm-verifier-model` | Stack/locals frame modeling, verifier-oriented type flow | `JvmFrameState` |
| `jvm-runtime-model` | Heap refs, frames, operand stack, locals, class/method refs | `JvmRuntimeState` |
| `jvm-vm-simulator` | Execute decoded method bodies | `ExecutionResult[JvmState]` |
| `jvm-simulator` | Backward-compatible facade package | thin composition layer |

### Why this split matters

- `jvm-classfile-decoder` is reusable by inspectors and compilers.
- `jvm-bytecode-decoder` is reusable by disassemblers and validators.
- `jvm-verifier-model` is reusable even before we implement full execution.
- `jvm-simulator` stays user-friendly while the internals stay modular.

## File format and decoder boundaries

### `jvm-classfile-decoder`

This package should stop at structural parsing:

- magic / version
- counts
- raw constant pool entry envelopes
- raw field / method headers
- raw attribute blobs

It should not interpret every attribute inline. That belongs to
`jvm-attribute-decoder`.

### `jvm-constant-pool-decoder`

Must handle the historical and modern tag set, including at least:

- `Utf8`
- `Integer`, `Float`, `Long`, `Double`
- `Class`, `String`
- `Fieldref`, `Methodref`, `InterfaceMethodref`
- `NameAndType`
- `MethodHandle`, `MethodType`
- `Dynamic`, `InvokeDynamic`
- `Module`, `Package`

### `jvm-attribute-decoder`

Needs a version-aware registry for attributes such as:

- `Code`
- `ConstantValue`
- `Exceptions`
- `InnerClasses`
- `Signature`
- `StackMapTable`
- `BootstrapMethods`
- `Module`
- `NestHost`, `NestMembers`
- `Record`
- `PermittedSubclasses`

Unknown attributes should be preserved as raw blobs, not discarded.

### `jvm-bytecode-decoder`

This package should decode instruction width and operands only. It should not
perform execution. Important details:

- branch offsets are relative to the current instruction
- `tableswitch` and `lookupswitch` require 4-byte alignment handling
- `wide` rewrites operand widths for a subset of instructions
- many instructions reference constant-pool indices rather than inline values

## Runtime model

The simulator should model real JVM execution concepts rather than flattening
everything into a generic stack VM:

- operand stack per frame
- local variable array per frame
- current method and current class
- constant pool attached to defining class
- invocation frames and return flow
- exception handler table lookup
- typed computational categories (`category 1` vs `category 2`)

Suggested state split:

- `JvmValue`
- `JvmFrame`
- `JvmThreadState`
- `JvmHeapRef`
- `JvmClassRef`
- `JvmMethodRef`

## JVM MVP execution slice

The first useful simulator slice should start with a single target profile:

- `java_8`

Reason:

- modern enough to be useful
- old enough to avoid modules, records, nestmates, and `ConstantDynamic`
- common real-world bytecode corpus

### MVP opcode families

- constants: `iconst_*`, `bipush`, `sipush`, `ldc`, `ldc_w`, `ldc2_w`
- loads/stores: `iload*`, `istore*`, `aload*`, `astore*`, `lload*`, `lstore*`
- stack ops: `pop`, `dup`, `dup_x1`, `swap`
- integer arithmetic: `iadd`, `isub`, `imul`, `idiv`, `irem`, `ineg`
- comparisons and branches: `ifeq`, `ifne`, `if_icmp*`, `goto`
- returns: `ireturn`, `areturn`, `return`
- fields and calls: `getstatic`, `putstatic`, `getfield`, `putfield`,
  `invokestatic`, `invokevirtual`, `invokespecial`
- object basics: `new`, `checkcast`, `instanceof`

### Defer for later

- full verifier enforcement
- monitors / synchronization
- `invokedynamic`
- interface default-method corner cases
- module system
- records / sealed / preview-only surface area

## Cross-language portability plan

We should not hardcode JVM metadata tables separately in every language port.
Instead, check in normalized repo-owned manifests such as:

- `jvm-classfile-versions.json`
- `jvm-opcodes.json`
- `jvm-attributes.json`
- `jvm-constant-pool-tags.json`

Each language implementation can consume those manifests while keeping runtime
logic native.

## Implementation phases

1. `jvm-version-profiles`
2. `jvm-classfile-decoder`
3. `jvm-constant-pool-decoder`
4. `jvm-attribute-decoder`
5. `jvm-bytecode-decoder`
6. `jvm-bytecode-disassembler`
7. `jvm-runtime-model`
8. `jvm-vm-simulator`
9. Backward-compatible facade updates in `jvm-simulator`

## References

- Oracle Java SE 26 specifications: https://docs.oracle.com/javase/specs/
- JVMS class file format: https://docs.oracle.com/javase/specs/jvms/se26/html/jvms-4.html
- JVMS instruction set: https://docs.oracle.com/javase/specs/jvms/se26/html/jvms-6.html
- OpenJDK JDK 26 project page: https://openjdk.org/projects/jdk/26/
