---
category: Compiler / VM / language pipeline
---

# Python 2 flushes a pending softspace as a newline before an uncaught exception, so print a, b cannot lower without a trap handler

Planning LANG84 phase 7c slice 1 (Python 2 `print` statement), the obvious
lowering of `print a, b` was `write_i64 a; putchar ' '; print_i64 b`: it
matches CPython 2's bytecode order (`PRINT_ITEM a`, then evaluate `b`, then
`PRINT_ITEM b`, `PRINT_NEWLINE`), including where a trap in `b` stops output.

It is still wrong. `PRINT_ITEM` sets the stream's *softspace* flag, and before
printing an uncaught exception's traceback CPython 2 calls `Py_FlushLine`
(`Python/pythonrun.c`, `PyErr_PrintEx`/`PyErr_Display`), which writes `\n` if
softspace is set. So `print 1, 1/0` writes `1\n` to stdout, not `1`. A
`StdoutThenTrap` expectation of `1` would have been a wrong oracle. No IIR
lowering can emit that newline only when a later trap fires, because LANG83
traps are not catchable yet.

**Fix:** the slice admits `print a, b` and `print a,` in the grammar (they are
valid Python 2) and the compiler refuses both (`Unsupported`, naming the
softspace reason). Single-item `print x` is safe: `PRINT_NEWLINE` clears
softspace, so a trap in the next statement flushes nothing.

**Do differently:** when lowering an output statement with stream state
(softspace, buffered line state, `$|`, `sep=`), read what the interpreter does
on its *abnormal* exit path (`Py_FlushLine`, `atexit`, the error printer), not
just the opcodes of the normal path; the trap rows of the corpus are exactly
where the two differ.
