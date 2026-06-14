# JVM02 — JAR as the distribution unit (when closures arrive)

## Why this spec exists

Today (TW02 + JVM01), the JVM backend emits a **single `.class`
file** per Twig program.  Every top-level `(define (f ...))`
becomes another `invokestatic` static method on the same class —
see ``_build_callable_method`` in
``ir-to-jvm-class-file/src/ir_to_jvm_class_file/backend.py``.
That works fine for any program where every function is a
named, top-level, non-capturing function.

The model breaks the moment **closures** (anonymous lambdas with
captured variables) arrive.  Each closure needs its own class
file holding the captured environment as fields, and the
single-`.class` shape can't express that.  At that point the
right packaging unit becomes a **JAR**: a ZIP archive containing
multiple `.class` files plus a `META-INF/MANIFEST.MF` declaring
the entry point.

This spec scopes the JAR work so it lands as a coordinated
package change *before* TW02.5 (closures) starts, not retroactively.

## Sister tracks

| Spec   | Backend  | Outcome                                       |
|--------|----------|-----------------------------------------------|
| JVM01  | JVM      | recursion works on real `java` ✅             |
| JVM02  | JVM      | JAR distribution + multi-class output (this) |
| BEAM01 | BEAM     | Twig runs on real `erl`                       |

## Why JAR is enough (vs. modules / multi-release JARs)

JAR is the lowest-common-denominator JVM packaging format that:

- Supports any number of `.class` files in any package layout.
- Carries a `META-INF/MANIFEST.MF` declaring `Main-Class`, so
  `java -jar twig_main.jar` Just Works without the user knowing
  about classpath.
- Is just a renamed ZIP — Python's ``zipfile`` can produce one
  with no native deps.
- Is what real-world JVM toolchains produce for distribution.

Java modules (`module-info.class` + `jmod`) and multi-release
JARs are out of scope — those are deployment optimizations Twig
doesn't need.

## Package decomposition

### 1. New package: `jvm-jar-writer`

Pure ZIP-format writer.  Takes a list of `(path_within_jar,
class_bytes)` plus a manifest dict, produces JAR bytes.

```python
@dataclass(frozen=True)
class JarManifest:
    main_class: str | None = None
    extra_attributes: dict[str, str] = field(default_factory=dict)

def write_jar(
    classes: tuple[tuple[str, bytes], ...],   # (path/to/Foo.class, bytes)
    manifest: JarManifest,
) -> bytes: ...
```

Internally just ``zipfile.ZipFile`` with the canonical
`META-INF/MANIFEST.MF` first.

### 2. Extend `ir-to-jvm-class-file`

Add a sibling lowering function that takes an `IrProgram` and a
JVM-friendly multi-class layout (when closures are involved) and
produces **multiple** `JVMClassArtifact` objects.  The existing
single-class path stays as the fast path for closure-free programs.

```python
@dataclass(frozen=True)
class JVMMultiClassArtifact:
    classes: tuple[JVMClassArtifact, ...]
    main_class_name: str

def lower_ir_to_jvm_jar(
    ir: IrProgram,
    config: JvmBackendConfig,
) -> JVMMultiClassArtifact: ...
```

### 3. Extend `twig-jvm-compiler`

When the Twig program contains a `Lambda` form (TW02.5+),
`compile_source` returns a JAR instead of a single class.
`run_source` invokes ``java -jar <jar_path>`` instead of
``java -cp <dir> <ClassName>``.

The split is detected automatically — TW02 programs (no
lambdas) stay on the single-class path.

## Validation strategy

`twig-jvm-compiler/tests/test_real_jvm.py` gains:

- A test using a TW02.5 closure that captures a free variable,
  asserts the captured value survives the call.
- A test that ``java -jar`` invocation produces the same byte
  output as the existing ``java -cp <dir> Class`` shape for
  closure-free programs (parity check).

## Out of scope for v1

- **Code signing.**  No signed JARs.  Twig is educational, not
  distributed to untrusted environments.
- **Resources.**  No bundled non-class resources (no
  `properties` files, etc.).  Twig has no use for them.
- **Sealed packages, multi-release.**  As above — deployment
  optimizations Twig doesn't need.
- **Cross-jar references.**  Single-JAR distribution only.

## Risk register

- **`META-INF/MANIFEST.MF` line-length and encoding rules.**  JAR
  manifests use 72-byte line wrapping with continuation.
  Mitigation: writer encodes manifest entries through one helper
  that handles wrapping, with unit tests.
- **Class-name collisions with reserved JAR layout.**  Don't
  emit user classes under `META-INF/`.  Mitigation: validator at
  ``write_jar`` entry rejects any class path starting with
  `META-INF/`.
- **ZIP timestamp determinism.**  Default `zipfile` writes the
  current local time; tests asserting on byte-level identity
  will flake.  Mitigation: pass a fixed `date_time` (1980-01-01)
  for every entry.
- **Twig closure semantics under-specified at the AST level.**
  TW02.5 needs to land first; JVM02 implementation can't start
  until then.  This spec exists to make the JAR work **planned
  in advance** so it doesn't get bolted on.
