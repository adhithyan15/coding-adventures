---
category: Rust
---

# A pre-main hook in ELF .init_array does not run on macOS; Mach-O needs its own __DATA,__mod_init_func entry

LH-003c (#19099) made test-bootstrap fail a run whose fd 1 was closed at
start-up. Rust's std reopens a closed fd 0/1/2 as `/dev/null` inside
`lang_start`, before `main`, so the closed state can only be seen from a
loader constructor. The PR installed one with
`#[link_section = ".init_array"]` under `#[cfg(target_os = "linux")]`, but
gated the test `a_closed_stdout_exits_one_not_zero` on `#[cfg(unix)]`. The
macOS CI job ran the test with no hook, std's `/dev/null` looked like a
working stdout, and the child exited 0. The spec's known-limits table even
said "macOS: not detected". The test and the spec disagreed and nobody ran
the test on a Mac.

The fix: Mach-O has its own constructor table. A static in
`#[link_section = "__DATA,__mod_init_func"]` (with `#[used]`) holds one
function pointer that dyld calls before `main`. This is the same mechanism
the `ctor` crate uses. `rustup target add aarch64-apple-darwin` and
`cargo build --lib --target aarch64-apple-darwin` work on Linux with no Apple
linker, because an rlib is not linked. `ar x` the rlib and `objdump -h` the
object to confirm a `__mod_init_func` section of 8 bytes.

Do this: give a platform hook and the tests that rely on it exactly the same
`cfg` predicate. A test gated more widely than its implementation fails on the
platforms in between, and one gated more narrowly is a silent coverage hole.
When CI runs macOS, cross-build the rlib for an Apple target before pushing
any `link_section` or other platform-specific code.
