# jvm-class-file (C)

A **small JVM class-file parser + builder** in pure ISO C17. A faithful port of
the Rust [`jvm-class-file`](../../rust/jvm-class-file) crate. It does two jobs:

1. parse a deliberately small, boring subset of the JVM `.class` format
2. build a minimal one-method class file for tests and bootstrap tooling

## Conservative by design

The parser treats its input as **untrusted**. Every read goes through a
bounds-checked cursor with a *sticky* status: the moment an attacker-controlled
length would run past the end of the buffer (or a constant-pool index is out of
range, or a tag is unknown), it returns `JVM_ERR_FORMAT` with a diagnostic
instead of guessing or reading out of bounds. The Rust original relies on Rust's
panic-on-OOB slice indexing; this C port makes every bound explicit.

## What it parses

The class header (magic / version / access flags), the constant pool (Utf8,
Integer, Long, Double, Class, String, Fieldref, Methodref, NameAndType — with
Long/Double correctly occupying two slots), fields, methods, and the `Code`
attribute (with its nested attributes). Resolvers turn constant-pool indices
into UTF-8 strings, class names, `NameAndType` pairs, loadable constants, and
field/method references.

## API

```c
#include "jvm_class_file.h"

char err[128];
JvmBuildParams p = jvm_build_params_default();
p.class_name = "demo/Example";
p.method_name = "main";
p.descriptor = "([Ljava/lang/String;)V";
static const uint8_t code[] = {0xB1};      /* return */
p.code = code; p.code_len = sizeof code; p.max_locals = 1;

uint8_t *bytes; size_t len;
jvm_build_minimal_class_file(&p, &bytes, &len, err, sizeof err);

JvmClassFile *cf;
jvm_parse_class_file(bytes, len, &cf, err, sizeof err);
/* jvm_class_this_name(cf) == "demo/Example" */
jvm_class_free(cf);
free(bytes);
```

- `jvm_parse_class_file` / `jvm_class_free` — parse, then release.
- `jvm_class_this_name` / `_super_name` / `_version` / `_access_flags`,
  constant-pool inspection, and the resolvers `jvm_get_utf8` /
  `jvm_resolve_constant` / `jvm_resolve_fieldref` / `jvm_resolve_methodref`.
- `jvm_class_find_method` + `jvm_method_code` for method/code access.
- `jvm_build_minimal_class_file` (+ `jvm_build_params_default`).

Resolver outputs borrow strings from the class file (valid until `jvm_class_free`).

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
