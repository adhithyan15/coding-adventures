# A struct literal inside a platform `cfg` gate is invisible to the other platforms' compilers (cowsay, PR #12168)

`rust/programs/cowsay` failed only on `build (macos-latest)` with

```
error[E0063]: missing field `wrap` in initializer of `layout_ir::TextContent`
```

`layout-ir` had gained a `wrap: bool` field on `TextContent`. Every other
construction site in the repo was updated at the time; the one that was missed
sat inside `#[cfg(target_vendor = "apple")] fn render_cowsay_png_metal`.

The generalisable point is that this is **not** the usual cross-platform bug.
It has nothing to do with path separators, line endings, locale, filesystem
case-sensitivity, `/tmp` vs `$TMPDIR`, terminal width, or Unicode — the list
you reach for when a job is red on one OS only. Those all describe code that
*runs* differently. Here the code did not *exist* for two of the three
compilers, so no amount of correctness on Linux or Windows could say anything
about it. When a **compile** error (rather than a test failure) is
platform-specific, suspect a `cfg` gate before suspecting behaviour. The clue
was in the timing: `FAILED 12.2s` means it compiled and stopped, not that a
long build ran or a test executed.

Three consequences worth internalising:

1. **Grepping for construction sites is necessary but not sufficient — grep
   for gated ones specifically.** The existing lesson "grep every consumer for
   record construction, not just pattern matches" (Haskell section) assumes
   your compiler will show you the consumers. It will not show you the ones
   behind a `cfg` for a platform you are not building. After changing a shared
   struct, run `grep -rn "cfg(target_" --include=*.rs` across the consumers you
   found and check each gated site against the target it is gated to.

2. **Prefer un-gating the data from the platform code.** The durable fix was
   not filling in the field, it was moving the struct literal out of the gated
   function into an un-gated helper — `layout_ir` is pure data, and only the
   *rendering* needed Metal/CoreText. All three legs then type-check the
   literal, so the next field addition fails on Ubuntu in minutes instead of on
   macOS in months. Gate the code that genuinely cannot compile elsewhere, and
   no more than that. `#[cfg_attr(not(target_vendor = "apple"),
   allow(dead_code))]` handles the resulting dead-code warning — and note that
   a lint allowance does **not** suppress type checking, which is exactly why
   this works.

3. **You can verify an Apple-gated path from Windows or Linux.** `cargo check`
   type-checks without linking, so `rustup target add aarch64-apple-darwin`
   followed by `cargo check --all-targets --target aarch64-apple-darwin`
   compiles the gated body with no Mac involved. This works whenever no
   dependency has a `build.rs` driving the `cc` crate. Do the same for
   `cargo clippy --target <t> --all-targets -- -D warnings`, once per target.

   **Always pair it with a control run.** Restore the pre-fix file
   (`git checkout origin/main -- <path>`, having first copied your version to
   the scratchpad — `git stash` is unsafe in these shared worktrees) and
   confirm the cross-check reproduces CI's exact error at the exact line, then
   restore your version. A check that cannot fail proves nothing; the control
   is what turns a clean run into evidence.

Finally, the reason it rotted at all: cowsay had **no `BUILD` file**, so the
build tool never discovered it and no CI leg ever compiled it. A `cfg`-gated
literal inside an unwatched package is doubly invisible. Before concluding "CI
is green, so this package is fine", confirm the package is actually in the
build graph — `ls <pkg>/BUILD`.
