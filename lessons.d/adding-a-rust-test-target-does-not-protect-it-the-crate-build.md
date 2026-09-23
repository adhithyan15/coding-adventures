---
category: BUILD files & dependency management
---

# Adding a Rust test target does not protect it: the crate BUILD file lists targets explicitly and nothing reports an omission

`code/packages/rust/lang-aot/BUILD` runs its tests by naming every target
explicitly (`cargo test -p lang-aot --lib --test a --test b …`), deliberately,
so that an omission is visible in review rather than hidden behind a bare
`cargo test -p lang-aot`.

The CLR09–CLR16 campaign (2026-09-19..20) added four test targets —
`clr_long_branches`, `clr_source_typed`, `clr_strict_flow`,
`clr_typed_scalars`, 19 tests between them — and added none of them to `BUILD`,
which had not been touched since 2026-09-07. For the entire campaign those
suites compiled in the check step and asserted nothing in CI. They all passed
locally when finally run by hand, so nothing was broken; the point is that
nothing would have been *reported* if something had been.

The failure mode is structural, not careless, which is why the warning already
sitting in that BUILD file's header did not prevent it:

- Adding `tests/foo.rs` is a complete, self-consistent, locally-green change.
- `cargo test` locally runs every target by auto-discovery, so the new suite
  passes in front of you.
- CI runs the *explicit list*, so the new suite is simply absent. Absence is
  not a failure, and no tool reports it.
- The diff that needed the extra line is in a different file from the one being
  written, in a different directory.

**What to do differently:** when adding a test target to a crate whose BUILD
names targets explicitly, add it to that BUILD line in the *same commit*. When
picking up any campaign that has been running across several PRs, diff the
crate's `tests/` directory against its BUILD list once before moving on:

```bash
comm -23 \
  <(ls code/packages/rust/<crate>/tests/*.rs | xargs -n1 basename | sed 's/\.rs$//' | sort) \
  <(grep -o -- '--test [a-z0-9_]*' code/packages/rust/<crate>/BUILD | cut -d' ' -f2 | sort)
```

Anything that prints is a target running in no CI command.

Related: `diff-based-ci-hides-latent-breakage` — a green main does not mean the
repo builds, and here a green PR did not mean the new tests ran.
