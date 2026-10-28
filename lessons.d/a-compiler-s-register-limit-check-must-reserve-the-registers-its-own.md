---
category: Compiler / VM / language pipeline
---

# A compiler's register-limit check must reserve the registers its own epilogue emits, or every just-over program is refused at the compiler's return instead of at a statement

**What went wrong.** The Python and Ruby IIR compilers' trusted budgets
(1 MiB, 256 Ki tokens) admitted a `main` of more than vm-core's 65,536
registers (`VMFrame::MAX_FRAME_REGISTERS`). The program compiled, then
vm-core refused it at load time with `RegisterLimitExceeded`, far from its
source (#18419). The earlier lesson about vm-core dropping the 65,537th
register had already said "the frontend accepts programs the VM cannot run".
Nobody added the check then.

When the check was added, a second trap appeared. `main` ends with a
compiler-written `const rN <- 0; ret rN`, and that exit register is always
the last new register. So in a program exactly one over the limit, the
register that does not fit is always the epilogue's. A plain "count as you
emit" check would blame no statement, or the last one by accident.

**Fix.** The ceiling is vm-core's constant re-exported
(`MAX_REGISTERS_PER_FUNCTION = VMFrame::MAX_FRAME_REGISTERS`), never a copied
number. The walker counts distinct `dest`s as vm-core does (parameters plus
non-parameter dests). While statements are lowered it reserves the
epilogue's one register (`dests + 1 ≤ limit`), and it drops the reserve for
the epilogue. That is exact (65,536 compiles and runs, 65,537 is refused),
and the `Budget` error names `main` and the statement that left no room. The
finished module also goes through vm-core's own `check_register_limit`, so
"compiles" implies "loads" by construction.

**Next time.** When a frontend raises its source or token budgets, find
every VM limit the bigger programs can now reach: frame registers, steps,
output. Refuse them in the compiler, using the VM's exported constant. If
the compiler emits code after the user's last statement, reserve that
code's resources up front so the error can point at source. Test both sides
of the boundary on the VM itself, not only the count.
