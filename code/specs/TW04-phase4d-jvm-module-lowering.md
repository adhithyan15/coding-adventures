# TW04 Phase 4d — JVM Module Lowering

## Purpose

Phase 4c established the platform-independent syscall convention for
`host/*` calls.  Phase 4d takes the next step: compile multi-module
Twig programs to multiple JVM `.class` files and lower cross-module
function calls to `invokestatic`.

After this phase, a Twig source tree like:

```scheme
;; a/math.tw
(module a/math
  (export add sub))

(define (add x y) (+ x y))
(define (sub x y) (- x y))

;; user/hello.tw
(module user/hello
  (import a/math))

(a/math/add 17 25)
```

compiles to two JVM classes — `a/math.class` and `user/hello.class` —
bundled into a JAR that runs on stock `java` and produces the correct
result.

## Scope

| In scope | Out of scope |
|----------|--------------|
| Each Twig module → one JVM `.class` | BEAM and CLR multi-module (Phases 4e–4f) |
| Cross-module function calls → `invokestatic` | Visibility enforcement (public vs. private methods) |
| Shared `TwigRuntime` register file | Separate compilation (all modules compiled together) |
| `compile_modules()` driver in `twig-jvm-compiler` | Incremental compilation / caching |
| Real-`java` end-to-end test | Full stdlib-in-Twig (Phase 4g) |

## Design

### Register-file sharing: `TwigRuntime`

The JVM calling convention uses a shared static `int[] __ca_regs`
array for all function parameters and return values.  In a
single-module program this array lives on the generated user class.
When multiple modules each define their own static `__ca_regs`, they
can't communicate — caller writes to its copy, callee reads from its
own copy.

**Solution: a shared `TwigRuntime` class.**

A new generated class `coding_adventures/twig/runtime/TwigRuntime`
owns the single canonical register file:

```java
public final class TwigRuntime {
    public static int[]    __ca_regs    = new int[256];
    public static Object[] __ca_objregs = new Object[256];
}
```

Every generated module class accesses registers via `getstatic` on
`TwigRuntime` rather than on itself.  The helper methods
(`__ca_regGet`, `__ca_regSet`, `__ca_syscall`, etc.) continue to live
on each module class, but they all read/write from `TwigRuntime`.

A new `JvmBackendConfig` field `external_runtime_class: str | None`
controls this.  When set to `"coding_adventures/twig/runtime/TwigRuntime"`,
every `getstatic`/`putstatic` for `__ca_regs` / `__ca_objregs`
targets the external class instead of `self.config.class_name`.
Single-module programs (`external_runtime_class = None`) are
completely unaffected — no behaviour change.

### Module name ↔ JVM class name

| Twig module name | JVM internal class name |
|------------------|-------------------------|
| `user/hello`     | `user/hello`            |
| `stdlib/io`      | `stdlib/io`             |
| `a/math`         | `a/math`                |
| `host`           | *(no class — virtual)*  |

JVM internal class names already use `/` as the package separator, so
the mapping is identity.  The same module name that appears in the
source `(module user/hello ...)` declaration is passed directly as
`class_name` to `JvmBackendConfig`.

### Cross-module call lowering

When module A calls function `f` exported by module B:

1. **Twig source**: `(b/f arg0 arg1)`
2. **Twig AST**: `(Apply (VarRef "b/f") args...)`
3. **`twig-jvm-compiler` IR emission**: emit parameter moves, then
   `IrOp.CALL IrLabel("b/f")` (a label containing `/`)
4. **`ir-to-jvm-class-file` lowering**: the `/` in the label signals a
   cross-module call.  Decompose `"b/f"` as class `"b"` method `"f"`.
   Emit `invokestatic b.f()I`.

The label decomposition rule: the last `/` separates class from
method.  So `"stdlib/io/println"` → class `"stdlib/io"`, method
`"println"`.

The caller-saves register snapshot/restore wraps the cross-module
`invokestatic` exactly as it does for same-class calls — the
convention is identical.

### `_discover_callable_regions` changes

Currently, `_discover_callable_regions` collects all CALL targets and
validates that each exists as a label in the current `IrProgram`.
Cross-module labels (containing `/`) cannot exist in the local
program; they must be excluded from this validation:

```python
target = _as_label(instruction.operands[0], "CALL target")
if "/" in target.name:
    continue          # cross-module — not a local callable region
callable_names.add(target.name)
```

Similarly, the second-pass consistency check skips cross-module CALL
targets.

### `twig-jvm-compiler` API extension

New public function:

```python
@dataclass(frozen=True)
class ModuleCompileResult:
    """One compiled module in a multi-module build."""
    module_name: str          # e.g. "stdlib/io"
    jvm_class_name: str       # e.g. "stdlib/io" (JVM internal)
    ir_program: IrProgram
    artifact: JVMClassArtifact
    is_entry: bool

@dataclass(frozen=True)
class MultiModuleResult:
    """Aggregate result of compiling a set of Twig modules."""
    runtime_artifact: JVMClassArtifact    # TwigRuntime shared reg file
    modules: list[ModuleCompileResult]    # in topological order
    entry_class_name: str

def compile_modules(
    modules: list[ResolvedModule],
    *,
    entry_module: str,
    optimize: bool = True,
) -> MultiModuleResult:
    ...
```

The `host` module is skipped (no `.class` emitted — its functions
lower to `IrOp.SYSCALL`, not `IrOp.CALL`).

### JAR bundling

`MultiModuleResult` carries all the `.class` bytes needed to build a
runnable JAR.  The driver (e.g. a `run_modules` function in
`twig-jvm-compiler`) bundles them with `jvm-jar-writer`:

```python
classes = [result.runtime_artifact]
for m in multi.modules:
    classes.append(m.artifact)
    # plus any closure / heap runtime classes from each module's multi_class_artifact
jar_bytes = write_jar(classes, JarManifest(main_class=multi.entry_class_name))
```

## Files changed

| File | Change |
|------|--------|
| `ir-to-jvm-class-file/src/…/backend.py` | `JvmBackendConfig.external_runtime_class`, `_discover_callable_regions` cross-module skip, cross-module CALL lowering, `build_runtime_class_artifact()` |
| `ir-to-jvm-class-file/src/…/__init__.py` | Export `build_runtime_class_artifact` |
| `twig-jvm-compiler/src/…/compiler.py` | `module_name_to_jvm_class()`, cross-module call in `_compile_apply`, `ModuleCompileResult`, `MultiModuleResult`, `compile_modules()`, optional `run_modules()` |
| `twig-jvm-compiler/src/…/__init__.py` | Export new public types/functions |
| `ir-to-jvm-class-file/CHANGELOG.md` | v0.17.0 |
| `twig-jvm-compiler/CHANGELOG.md` | new version entry |
| `code/specs/TW04-modules-and-host-package.md` | Phase 4d marked shipped |

## Tests

### `ir-to-jvm-class-file/tests/test_cross_module_call.py` (new)

- `test_cross_module_call_lowering_emits_invokestatic` — compile an
  `IrProgram` with `CALL IrLabel("a/b")` and assert the bytecode
  contains an `invokestatic` targeting `a.b()I`.
- `test_cross_module_call_skipped_in_callable_regions` — an `IrProgram`
  whose only CALL is `IrLabel("x/y")` does not raise
  `JvmBackendError("Missing callable labels")`.
- `test_runtime_class_artifact_fields` — `build_runtime_class_artifact()`
  returns a `JVMClassArtifact` whose constant pool references
  `__ca_regs` and `__ca_objregs` as `[I` / `[Ljava/lang/Object;`
  respectively.
- `test_external_runtime_class_used_for_regs` — when
  `external_runtime_class` is set, `getstatic` in the generated class
  names the runtime class, not the current class.

### `twig-jvm-compiler/tests/test_multi_module.py` (new)

- `test_compile_modules_returns_multi_module_result` — two-module
  program (a/math + user/hello) produces a `MultiModuleResult` with
  `runtime_artifact` and two `ModuleCompileResult` entries.
- `test_entry_module_has_main_wrapper` — the entry module's artifact
  has `main([Ljava/lang/String;)V` in its constant pool.
- `test_dep_module_has_no_main_wrapper` — the dependency module's
  artifact does NOT have a `main()V`.
- `test_cross_module_call_in_ir` — the entry module's `IrProgram`
  contains `IrOp.CALL IrLabel("a/math/add")`.
- `test_run_modules_on_real_java` *(skip if no java)* — `run_modules`
  on the two-module program returns the correct result.

## Acceptance criterion

The following runs on stock `java` and exits successfully:

```python
from twig import resolve_modules
from pathlib import Path
import tempfile

src = {
    "a/math.tw": """
        (module a/math (export add))
        (define (add x y) (+ x y))
    """,
    "user/hello.tw": """
        (module user/hello (import a/math))
        (a/math/add 17 25)
    """,
}
with tempfile.TemporaryDirectory() as tmp:
    for name, source in src.items():
        path = Path(tmp) / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source)
    modules = resolve_modules("user/hello", search_paths=[Path(tmp)])
    from twig_jvm_compiler import run_modules
    result = run_modules(modules, entry_module="user/hello")
    assert result.exit_code == 0
    # The program's HALT value (42) exits the JVM process
```
