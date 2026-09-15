## Unreleased — portable matrix text and mixed JVM arithmetic (VM-033/VM-034)

Matrix text-output expectations and BASIC differential comparisons now accept
CRLF as LF while preserving every other character. Brainfuck keeps its existing
exact comparison. Raw stdout remains in failure messages. This repairs Windows
LLVM BASIC multi-line output comparisons without changing generated programs.

Normal BUILD runs focused positive/negative comparison regressions and the
existing multi-line real, mixed DATA, and RND programs on every available
declared backend. Full non-ALGOL and ALGOL matrix CI coverage remains tracked
separately under VM-024 and VM-025.

Those executions exposed a second defect: the JVM compatibility pass narrowed
BASIC RND's i64 multiplication to i32 because all its literals fit in i32.
The overflow made later samples negative. Modules using floating-point values
now retain their integer widths across all functions: they already require
real Java instead of the integer-only simulator. The existing integer-only
simulator path remains covered, and RND's shared helper/product stays i64.

