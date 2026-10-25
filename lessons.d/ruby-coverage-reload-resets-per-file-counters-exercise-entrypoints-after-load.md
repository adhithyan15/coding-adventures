---
category: Testing & coverage
---

# Ruby Coverage reload resets per-file counters; exercise entrypoints after load

NN45's executable coverage harness first exercised native training/resume, then
used `load` to measure the main error/exit branch. Ruby Coverage replaced that
file's counters on reload, so the report incorrectly retained only the last
error-path execution (68.42%), not the earlier training calls.

Exercise methods after the final instrumented load, restore process globals in
ensure, and inspect the same absolute file's counters. The corrected harness
measures fresh training, resume and the real error branch at 100%. Keep the
80% floor: fix measurement order rather than lowering the threshold or claiming
an unmeasured child process contributed coverage.
