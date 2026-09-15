## 0.171.0 — 2026-07-03 — E4-dyn foothold: first runtime (non-foldable) string matrix cell

First matrix proof of a **runtime** string (`code/specs/lang-full-e4-dyn-strings.md`):

```basic
10 INPUT N
20 IF N > 0 THEN 50
30 LET A$ = "LO"
40 GOTO 60
50 LET A$ = "HI"
60 PRINT A$
70 END
```

Because `N` is read at run time, *which* literal `A$` holds at line 60 is **not
known at compile time** — `A$` is a genuinely dynamic string, unlike every prior
E4 cell where the compiler folds the string to a constant. Stdin `1` → `N=1>0` →
`A$="HI"` → prints `HI`.

Tagged on the **four already-dynamic backends** (`Vm`, `Jit`, `Jvm`, `Clr`),
which carry a reassigned-across-branches string slot natively (tagged value /
`java.lang.String` / `System.String`). The four static backends
(`NativeAot`/`Llvm`/`Wasm`) fold strings to compile-time constants and cannot yet
represent this value; the E4-dyn backend PRs (E4d-2 LLVM → E4d-3 WASM → E4d-4
native) extend **this cell's `backends` list** column-by-column as each lands its
runtime heap-string lowering on the E4d-1 `__twig_str_*` helpers.

This establishes the shared matrix cell the backend PRs prove themselves against
— resolving the ordering constraint that a static-backend runtime-string lowering
can't be matrix-proven until a frontend emits a non-foldable string.

