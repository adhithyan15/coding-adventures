# jvm-class-file (C++)

A **small JVM class-file parser + builder**, header-only, ISO C++17. A faithful
port of the Rust [`jvm-class-file`](../../rust/jvm-class-file) crate, in namespace
`ca::jvm_class_file`. It does two jobs:

1. parse a deliberately small, boring subset of the JVM `.class` format
2. build a minimal one-method class file for tests and bootstrap tooling

## Conservative by design

The parser treats its input as **untrusted**: every read goes through a
bounds-checked cursor (`ClassReader`), and any attacker-controlled length that
would run past the end of the buffer — or an out-of-range constant-pool index,
or an unknown tag — throws `ca::jvm_class_file::Error` instead of guessing. No
malformed input can read out of bounds.

## Model

The Rust structs map directly onto value types: `ClassFile` holds a
`std::vector<std::optional<ConstantPoolEntry>>` constant pool (Long/Double
occupy two slots, so the second is `std::nullopt`), plus `FieldInfo` /
`MethodInfo` vectors. `MethodInfo::code_attribute()` returns the first `Code`
attribute; resolvers (`get_utf8`, `resolve_class_name`, `resolve_name_and_type`,
`resolve_constant`, `resolve_fieldref`, `resolve_methodref`) mirror the crate.

## API

```cpp
#include "jvm_class_file.hpp"
namespace jc = ca::jvm_class_file;

jc::BuildMinimalClassFileParams p;
p.class_name = "demo/Example";
p.method_name = "main";
p.descriptor = "([Ljava/lang/String;)V";
p.code = {0xB1};        // return
p.max_locals = 1;

std::vector<std::uint8_t> bytes = jc::build_minimal_class_file(p);
jc::ClassFile cf = jc::parse_class_file(bytes);   // throws jc::Error on bad input
// cf.this_class_name == "demo/Example"
const jc::MethodInfo* m = cf.find_method("main");
```

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
