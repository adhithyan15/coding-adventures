---
category: Cross-platform & Windows BUILD_windows
---

# PowerShell 5 build wrappers must decode UTF-8 shared harness source explicitly

NN47's first Windows build failed before compilation: Windows PowerShell 5
decoded the BOM-free UTF-8 shared ISO harness as ANSI. An em dash inside an
executable diagnostic became mojibake containing a smart quote and broke parsing.
The new wrapper reads the fixed trusted source with explicit UTF-8 before
dot-sourcing its scriptblock. The shared diagnostic also uses ASCII for existing
PowerShell 5 callers. Keep the shared compiler scheduler and strict warnings;
do not copy the scheduler or misclassify this parse/bootstrap failure as C code.
