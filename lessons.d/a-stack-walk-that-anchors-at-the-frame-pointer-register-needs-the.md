---
category: Rust
---

# A stack walk that anchors at the frame-pointer register needs the collector built with -Cforce-frame-pointers

**What went wrong.** `twig-aot` embeds `gc-core-capi` as a `--release`
staticlib, built by a nested `cargo build` in `build.rs`. The precise
collector reads `rbp` inside `__gc_collect_precise` and walks the
`[fp] = caller fp, [fp + 8] = return address` chain into compiled Twig
frames, whose stack maps name their reference slots. Release builds omit the
frame pointer, so the function never set `rbp`; on rustc 1.97 it still held the
Twig frame's own `rbp`. The walk began one frame too far out: the innermost
Twig frame was scanned conservatively as part of `[sp, start_fp)`, and
precise collection silently became conservative. Two `linux_x86_64_smoke`
differential tests (`precise kept 64`) failed on that host while CI, on a
newer stable rustc, passed. The crate's own comment had said the frame
pointer "holds under Rust's current x86-64-host defaults" and left enforcing
it to a later rung that never added it.

**Fix.** `build.rs` appends `-Cforce-frame-pointers=yes` to
`CARGO_ENCODED_RUSTFLAGS` for the nested build. The archive's
`__gc_collect_precise` now starts with `push rbp; mov rbp, rsp`, and both
tests pass.

**Next time.** When correctness depends on a codegen property (frame
pointers, inlining, a register's contents), enforce it in the build that
produces the code, and check it in the disassembly of the artifact actually
linked. Never rely on compiler defaults; they change between rustc versions
and profiles. A test that passes in CI and fails locally on another
toolchain is evidence of an unenforced property, not a flaky host.

**Caveat (security review), since resolved.** The flag fixes SysV x86-64 and
AArch64 only. On Windows x64, LLVM anchors `rbp` with `lea rbp, [rsp+N]` even
under `-Cforce-frame-pointers=yes`, so `[rbp]` is a local, not a saved frame
pointer. The durable fix was to stop reading the register at all: each precise
collect entry is now a naked trampoline (`push rbp; mov rbp, rsp; …; call
body` / `stp x29, x30, [sp, #-16]!; mov x29, sp; …; bl body`) that builds the
record and passes its address as an argument (gc-core-capi `stack_scan.rs`
section 2b). The x86-64 Linux differentials pass with that archive even when
it is built with `-Cforce-frame-pointers=no`. Prefer that shape from the start:
when a layout is load-bearing, write the few instructions that fix it rather
than asking the compiler for it. A naked function also gets no unwind info, so
add `.cfi_*` / `.seh_*` directives and check them with `llvm-readobj --unwind`
or `llvm-objdump --dwarf=frames`.
