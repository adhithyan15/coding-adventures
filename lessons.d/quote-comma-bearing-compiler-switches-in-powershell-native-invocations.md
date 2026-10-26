---
category: Cross-platform & Windows BUILD_windows
---

# Quote comma-bearing compiler switches in PowerShell native invocations

NN48's sanitizer command initially failed in PowerShell parsing, before Clang
was invoked: unquoted -fsanitize=address,undefined was treated as comma syntax.
Pass that entire native switch as one quoted argument. Do not infer a compiler
or sanitizer defect from a shell parser error. With the switch quoted and the
LLVM sanitizer DLL directory in process PATH, the actual Clang ASan/UBSan suite
completed all 943 checks with zero failures. No source or warning policy change
was needed for this host invocation mistake.

A later deeply nested python -c/exec probe also failed in quoting before any
child was invoked. Replace that disposable probe with the actual saved,
Ruff-formatted shell-free validator rather than layering more shell escapes.
The saved validator actually ran five native children and compared full
uninterrupted/resumed model state through 3002.
