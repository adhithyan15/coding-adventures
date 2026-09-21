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
for (label, src) in hostile_cases {
    // Display, NOT Debug: Debug escapes control characters itself and would
    // make the next assertion unfalsifiable. Display is also what production
    // formats with.
    let Err(e) = compile_source(src, "hygiene") else {
        panic!("`{label}` no longer fails, so it stopped exercising this path")
    };
    let msg = e.to_string();
    assert!(msg.chars().find(|c| c.is_control() && *c != '
').is_none());
    // Ceiling strictly between the real max and the mutation-defeated max.
    assert!(msg.len() < 2048);
}
```

A missed call site shows up there automatically, because the bad output
appears whether or not anyone remembered the site exists.

**Verify the guard is not vacuous — and check WHICH assertion fires.** The
first version of this test asserted over `format!("{e:?}")`. `Debug for str`
escapes control characters itself, so the control-character assertion could
never fire: the guard was vacuous on exactly the half it was written for. It
*was* "verified" by reverting the fix and watching the test go red — but the
failure came from the unrelated *length* assertion, and nobody checked which
one fired. A fourth review round caught it.

Two rules follow:

- A positive control must confirm the **specific** assertion you care about
  fires, not merely that the test fails. Read the panic message.
- Assert over the formatter **production** uses. Here that is `Display`:
  `lang-aot` formats this error with `{}` straight into the build log, while
  `Debug` sanitises on the way out and hides the very bug you are hunting.

**A multi-limb assertion needs one mutation per limb.** Fixing the Debug trap
above meant raising the length ceiling (4096 to 8192) so the length assertion
would stop pre-empting the control-character one. That fixed the diagnosis and
*disarmed the other limb*: the real maximum message was ~1194 bytes, so nothing
could reach 8192, and a reviewer deleted truncation from the helper entirely
with the test still green. The vacuousness had simply moved from one half of
the guard to the other.

A guard asserting N properties needs N mutations, each defeating exactly one
property, each confirming the matching assertion fires. Here:

| mutation | limb that must fire |
|---|---|
| helper returns text unescaped, truncation intact | control-character assertion |
| helper never truncates, escaping intact | length assertion |

Both were run, and the bound was then chosen to sit strictly between the real
maximum (~1194) and the mutated maximum (~5152). A threshold outside that
window cannot fail, whatever it is protecting.

**Count sites, not inputs.** That same test had seven inputs and a comment
claiming each reached a different message. Three of them were not lexable at
all, died in the lexer with an identical error, and never reached the code
under test — seven inputs, four sites. A `checked >= 5` floor on input count
said nothing about coverage. Assert on the number of distinct message families
actually reached.

Applies equally to any cross-cutting obligation implemented as "call this
helper": escaping, redaction, authorisation checks, quota accounting. If the
rule is "every X must go through Y", the test belongs downstream of every X,
not at each Y.

Related: `assert-structure-not-substrings` (positive controls, and tests that
prove nothing), and
`a-filtered-ci-test-invocation-stops-covering-a-test-that-no-longer` (the same
shape in CI configuration â€” coverage that looks present and is absent).
