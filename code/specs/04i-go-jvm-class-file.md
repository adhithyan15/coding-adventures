# 04i - Go JVM Class-File Package

## Overview

This spec defines the first non-Python port of the repository's reusable
`jvm-class-file` layer.

The package does **not** lower IR and does **not** know about Brainfuck or Nib.
Its job is smaller and more reusable:

- parse a conservative subset of `.class` files
- build a tiny but valid `.class` file directly from bytecode and metadata
- resolve constant-pool references that later backend tests can inspect

That makes it the foundation that a future Go `ir-to-jvm-class-file` package can
stand on.

## Why This Package Exists

`04g-ir-to-jvm-class-file.md` split the JVM lane into two layers:

1. `jvm-class-file`
2. `ir-to-jvm-class-file`

`04h-source-to-jvm-pipelines.md` then made the rollout order explicit for every
language bucket:

1. port `jvm-class-file`
2. port `ir-to-jvm-class-file`
3. add `brainfuck-jvm-compiler`
4. add `nib-jvm-compiler`

This spec is step 1 for Go.

## Package Path

```text
code/packages/go/jvm-class-file
```

Module path:

```text
github.com/adhithyan15/coding-adventures/code/packages/go/jvm-class-file
```

Package name:

```text
jvmclassfile
```

## MVP Scope

The Go port should match the current Python package's useful minimum, not the
full JVM specification.

### Supported constant-pool entries

- `Utf8`
- `Integer`
- `Long`
- `Double`
- `Class`
- `String`
- `NameAndType`
- `Fieldref`
- `Methodref`

Anything outside that subset may return a parse error for now.

### Supported class-file structure

The parser must understand:

- magic number and version
- constant pool
- class access flags
- this/super class
- method table
- opaque attributes
- structured parsing of the `Code` attribute

The parser may skip:

- interfaces beyond counting and skipping indices
- field contents beyond counting and skipping members
- exception table details beyond skipping them
- non-`Code` attributes as raw payload blobs

### Builder responsibilities

The builder only needs to create a **minimal single-class artifact**:

- one class
- one method
- one `Code` attribute
- optional integer and string constants in the constant pool
- default superclass `java/lang/Object`
- default class flags `ACC_PUBLIC | ACC_SUPER`
- default method flags `ACC_PUBLIC | ACC_STATIC`

This mirrors the current Python helper and is enough for round-trip tests and
for the next Go backend layer.

## Public API

The package should expose:

- class-file access flag constants:
  - `ACC_PUBLIC`
  - `ACC_STATIC`
  - `ACC_SUPER`
- `ClassFileFormatError`
- `JVMClassVersion`
- `JVMAttributeInfo`
- `JVMCodeAttribute`
- `JVMMethodInfo`
- `JVMClassFile`
- `JVMFieldReference`
- `JVMMethodReference`
- `ParseClassFile(data []byte) (*JVMClassFile, error)`
- `BuildMinimalClassFile(params BuildMinimalClassFileParams) ([]byte, error)`

The class model should also provide helper methods paralleling the Python API:

- `GetUTF8`
- `ResolveClassName`
- `ResolveNameAndType`
- `ResolveConstant`
- `ResolveFieldref`
- `ResolveMethodref`
- `LdcConstants`
- `FindMethod`

## Error Model

Malformed data should return `ClassFileFormatError`.

Examples:

- invalid magic
- truncated input
- unsupported constant-pool tag
- out-of-range constant-pool index
- asking for a constant type that does not match the referenced entry

## Testing Requirements

The package must exceed 80% coverage and include:

1. round-trip parse of a generated minimal class
2. invalid magic rejection
3. string/integer constant lookup coverage
4. field and method reference resolution from a real class-file fixture
5. at least one parse failure beyond magic-number validation

## Non-Goals For This Slice

This package does **not** yet need:

- stack-map-table support
- general-purpose class-file emission
- field emission helpers
- interface emission
- invokedynamic support
- IR lowering
- JVM execution

## Bottom Line

Go gets a truthful `jvm-class-file` seam first.

Once that exists and is tested, the next Go JVM step becomes much smaller:

```text
compiler-ir -> ir-to-jvm-class-file -> jvm-class-file -> .class
```
