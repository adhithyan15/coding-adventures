# jvm-jar-writer

Pure ZIP-format writer for JVM JAR archives.  Takes a list of
`(path_within_jar, class_bytes)` plus an optional manifest dict
and produces JAR bytes suitable for `java -jar` invocation.

This is JVM02 Phase 1 — see the
[JVM02 spec](../../../specs/JVM02-jar-as-distribution-unit.md)
for context.  The next phase (multi-class lowering in
`ir-to-jvm-class-file`) and the closure work in
`twig-jvm-compiler` (TW02.5) build on top of this.

## Why a JAR (when single `.class` works for non-closure programs)

Today's JVM backend emits a single `.class` per Twig program and
that's correct for any closure-free program: every top-level
`(define (f ...))` becomes another `invokestatic` static method
on the same class.  See [`backend.py`](../ir-to-jvm-class-file/src/ir_to_jvm_class_file/backend.py).

The model breaks the moment **closures** (anonymous lambdas with
captured variables) arrive — each closure needs its own class
file holding the captured environment as fields, and a single
`.class` file can't express that.  JAR (a ZIP archive containing
multiple `.class` files plus `META-INF/MANIFEST.MF`) is the
lowest-common-denominator JVM packaging that handles this.

## Quick start

```python
from jvm_jar_writer import JarManifest, write_jar

class_bytes = ...  # bytes from ir-to-jvm-class-file
jar_bytes = write_jar(
    classes=(("com/example/Main.class", class_bytes),),
    manifest=JarManifest(main_class="com.example.Main"),
)
# Write to disk and run with: java -jar Main.jar
```

## What this package does NOT do

- **No bytecode generation.**  This package writes the JAR
  envelope; class file bytes come from `ir-to-jvm-class-file`.
- **No JAR signing.**  Twig is educational, not distributed to
  untrusted environments.
- **No multi-release JARs / Java modules.**  Plain JAR only.
- **No resource bundling.**  Only `.class` entries (and the
  auto-injected manifest).

## Design notes

- ZIP entries use **deterministic timestamps** (1980-01-01) so
  byte-equivalent inputs produce byte-equivalent JARs.  Tests can
  assert on JAR-byte equality.
- Manifest follows the standard 72-byte line wrapping, written
  through one helper that handles continuation lines.
- The validator at `write_jar` entry rejects any class path
  starting with `META-INF/` — that's reserved JAR layout.

## Sister packages

- [`ir-to-jvm-class-file`](../ir-to-jvm-class-file/) — produces
  the `.class` bytes that this package bundles.
- [`twig-jvm-compiler`](../twig-jvm-compiler/) — the eventual
  consumer (TW02.5+, when closures land).
- [`jvm-class-file`](../jvm-class-file/) — the upstream library
  that knows the JVM `.class` format.
