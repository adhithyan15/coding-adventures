---
category: Cryptography & security review
---

# Centralising an escaping helper does not centralise the decision to call it: assert the invariant over outputs

`source-preprocessor`'s `PpError::quote` escapes control characters and
truncates attacker-supplied text before it reaches a diagnostic. Its doc
comment said, reasonably:

> Escaping here rather than at each call site means a new interpolation cannot
> forget.

That is false, and three consecutive security-review rounds found it false in a
different place each time:

1. Round 1 â€” `RootedFs`'s messages interpolated the include spelling raw.
   Fixed.
2. Round 2 â€” the engine's include-cycle message and the MacroOct dialect's
   `describe` still did. Fixed.
3. Round 3 â€” `MemoryFs::resolve` still did. And `MemoryFs` is **not**
   test-only: `macrooct-iir-compiler` builds one for every `compile_source`,
   which is what `lang-aot`'s `compile_source_to_iir` calls. So
   `@include "<ESC>[2Jpwned"` put a raw terminal escape into the build log
   from source alone, and a 5 KB spelling produced a 5 KB diagnostic.

Each round fixed the sites that were *looked at*. The helper existed and was
correct the whole time. **Centralising a helper centralises the
implementation, not the decision to use it** â€” and a call site that never
calls it is invisible to every test that only exercises call sites someone
already thought of.

**What actually worked:** assert the invariant over **outputs**, not over call
sites. One test drives a batch of hostile inputs through the *public* entry
point and asserts that no resulting diagnostic contains a control character or
exceeds a length bound:

```rust
for src in hostile_cases {
    let Err(e) = compile_source(src, "hygiene") else { continue };
    let msg = format!("{e:?}");
    assert!(msg.chars().find(|c| c.is_control()).is_none());
    assert!(msg.len() < 4096);
}
```

A missed call site shows up there automatically, because the bad output
appears whether or not anyone remembered the site exists.

**Verify the guard is not vacuous.** Revert the fix, confirm the test fails,
restore it. This one was checked that way and did fail on the reverted bug â€” a
hygiene test that passes because its inputs never reach the bad path is worse
than none, since it reads as coverage.

Applies equally to any cross-cutting obligation implemented as "call this
helper": escaping, redaction, authorisation checks, quota accounting. If the
rule is "every X must go through Y", the test belongs downstream of every X,
not at each Y.

Related: `assert-structure-not-substrings` (positive controls, and tests that
prove nothing), and
`a-filtered-ci-test-invocation-stops-covering-a-test-that-no-longer` (the same
shape in CI configuration â€” coverage that looks present and is absent).
