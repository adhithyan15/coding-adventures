---
category: Compiler / VM / language pipeline
---

# An interpreter's step budget must be checked at the top of the step loop, not inside one kind of step

The `warrant-ir` reference interpreter checked `max_steps` inside the instruction loop and in `call_fn`. A function whose blocks hold no instructions, such as two blocks that `Jump` to each other, runs only terminator steps, so no check was ever reached and `run` never returned (KRN-006c, found while writing VER25 section 8.10). The budget exists to make every run end; a check placed in one kind of step only bounds the runs that happen to contain that kind.

The fix checks the budget at the top of every block visit and again before the terminator runs, so the cost a terminator charges is seen by the next step. The test runs the interpreter on a worker thread and waits with `recv_timeout`: a regression then fails in seconds with a message instead of hanging CI.

Do differently:
- Put a resource check at the single point every step passes through (the top of the dispatch loop), not next to the code that charges the cost.
- Test a budget with the cheapest possible non-terminating program: a cycle of empty blocks, a self-jump, a branch back to itself. A budget test that uses a program with instructions in it proves nothing about the terminators.
- Any test that can hang if its subject regresses must carry its own timeout, such as a thread plus `recv_timeout`.
