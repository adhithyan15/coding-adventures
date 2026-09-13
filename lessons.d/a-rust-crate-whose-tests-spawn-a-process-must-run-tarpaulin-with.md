---
category: Testing & coverage
---

# A Rust crate whose tests SPAWN A PROCESS must run tarpaulin with `--engine llvm`, or coverage dies with `SIGILL` while every test passes

Tarpaulin's default Linux engine is ptrace: it patches INT3 breakpoints into the live process and steps each thread back over them. Per tarpaulin's own developer docs, "any threads that have hit a breakpoint need it disabled and stepped back to the start of the instruction otherwise you'll get a SIGILL as the ptrace continue/step commands will continue that thread with the program counter in an invalid position." A `Command::spawn` from a breakpointed worker thread does exactly that — the forked/cloned child resumes mid-instruction and the whole run aborts with `Failed to run tests: Error running test - SIGILL raised in <pid>`, *after* printing `test result: ok`. **The symptom is a coverage crash, not a test failure — don't go looking for a bug in the test.** Fix: add `--engine llvm` (uses `-C instrument-coverage`, no ptrace, so spawning is a non-event). Do **not** "fix" this by dropping the integration test from coverage: for the three `chief-of-staff-*-approval` adapters the spawning integration test is what covers most of `src/`, so excluding it would put the crate under the 80% bar.
