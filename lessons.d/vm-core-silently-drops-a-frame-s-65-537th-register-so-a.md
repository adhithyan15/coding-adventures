---
category: Compiler / VM / language pipeline
---

# vm-core silently drops a frame's 65,537th register, so a straight-line frontend's largest accepted program fails as an undefined variable

**What went wrong.** LANG83 PR7c-py's budget test ran the largest
`py-float-print` program the trusted frontend budgets accept (1 MiB, 32,768
lines of `print(-1.2345678901234567e-308)`) through `run_module`. It failed
with `undefined variable "py65536"`, not a step-budget error.
`VMFrame::assign` caps a frame at `MAX_FRAME_REGISTERS` (65,536) distinct
names and silently drops every later new one, so the next read of it fails.
The Python release path gives every temporary a fresh `pyN`, three or more
per line, so any `main` past ~21,800 such lines hits it. That was true
before PR7c-py too, for ints as well as floats. The first draft made it
worse: a fresh register for each print's unused `call` result.

**Fix.** Every `call __py_write_float` writes its unused result into one
shared register (`py_repr`), so the PR adds no register per print. The test
runs the trusted-budget program at the register limit, not the source
limit. The cap itself is a separate issue: the frontend accepts programs
the VM cannot run.

**Next time.** When sizing a runner for "the largest program the limits
accept", check every VM limit, not only the step budget: frame registers
(`MAX_FRAME_REGISTERS`), frames, memory and output bytes. Run the extreme
program rather than reasoning about it. Bounds you estimate by reading
code are low too: PR7c-py's first per-print step bound, 512, was read off
the helper, and the measured dearest print is 700 steps
(`-1.7976931348623157e+308`). Measure with `VMCore::metrics_instrs`.
