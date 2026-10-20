---
category: Compiler / VM / language pipeline
---

# A BEAM heap overrun that only one CI shard hits is a missing test_heap; audit the disassembly instead of rerunning

lang-aot's `sparse_real_and_string_value_copies_run_on_beam` aborted erl with
`beam/erl_gc.c, line 775: <0.10.0>: Overrun stack and heap` on main-build
shard 2 of PR #17759, twice in a row. It passed on the Linux legs of #17745
and #17750, in five earlier main runs, and locally on macOS under OTP 27 and
28, with small-heap `+hms` flags, and with temp paths of 1 to 60 characters.
Every one of those runs looked like evidence for "flaky".

It was deterministic codegen. `put_list` writes its two words without
checking for room; reserving them is the compiler's job. Disassembling the
generated module (`beam_disasm:file/1`, which the test leaves in its temp
dir) and walking it with a small script showed every `put_list` was covered
by a preceding `test_heap` since the last call except four. All four came
from string literals (`"OK"`, `"HI"`): iir-to-beam's `str_const` arm reserved
nothing. Whether the unreserved write reached the stack depended on how full
the heap was when the literal was built, and that depends on everything the
process did before.

It was the third time for this backend. BEAM10 fixed the identical omission
in `alloc_array` and `array_set`, with a test comment warning that "it passed"
is not evidence.

What to do instead:
- On any `Overrun stack and heap`, `size_object: bad tag`, or segfault from
  emitted BEAM, disassemble the module and audit each allocating instruction
  (`put_list`, `put_tuple2`, float boxes) against a `test_heap` since the
  last GC point, before you rerun anything.
- Prove a fix with a stress test that makes the overrun certain: thousands of
  allocations in one function. Here, 3,000 literals crashed the emulator
  deterministically on macOS once the reservation was removed.
- A `test_heap` is a GC point. Anything live in an X register above `live`
  must be parked in a Y slot around it, the same as across a call.
