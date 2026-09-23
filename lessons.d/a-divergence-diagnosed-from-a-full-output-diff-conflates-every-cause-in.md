---
category: Testing & coverage
---

# A divergence diagnosed from a full-output diff conflates every cause in that output; probe the oracle with minimal isolated inputs before writing the diagnosis down

I filed CCR-073 (#15854) as *"the emitter places the statement terminator inside
a block-terminated `switch`/`try` rather than after it"*, reading it off the
before/after bytes of two ladder rungs:

```
upstream: var x=1;switch(x){case 1:console.log(1);break;default:console.log(2)};
ours:     var x=1;switch(x){case 1:console.log(1);break;default:console.log(2);}
```

`switch` on both sides, a semicolon that moved — the diagnosis wrote itself. It
was wrong.

Running the pinned oracle JAR over 31 minimal probes showed the real rule has
nothing to do with `switch` or `try`:

> Upstream terminates the **last statement of a program** even when it ends with
> `}`, and suppresses that terminator mid-stream.

Every one of these gets the trailing `;`: `switch`, `try/catch`, `try/finally`,
braced `if`, `for`, `for-in`, `for-of`, `while`, `with`, a bare block, a labeled
block, a function declaration, a class declaration. And the same `switch`
followed by another statement gets **no** terminator. `switch` and `try` were
simply the two constructs those two rungs happened to end a program with.

The cost was not the wrong sentence. It was that the issue pointed at
`emit_switch`/`emit_try`, where the bug is not, and the actual site —
`emit_program`'s trailing-normalization special case, which handled only
function and class declarations — would not have been found by anyone following
the issue. A blast-radius estimate and a CHANGELOG entry were both derived from
the wrong cause.

**What to do instead.** A differential diff tells you two outputs differ. It does
not tell you why, because a full output has every cause in it at once. Before
writing a diagnosis down where another agent will act on it, run the oracle over
inputs that isolate one construct each, and over the *negative* cases too — here,
the same construct in mid-stream position, which is what proved the rule was
positional rather than construct-specific. Then check the claimed rule against
the whole corpus: 752 of 768 committed goldens corroborated it, and the only two
JavaScript counterexamples turned out to be fixtures whose provenance was
`unverified`, i.e. pinning the bug as if it were correct.

**Second-order finding from the same probes, worth carrying separately.** The two
compilation levels run *different emitters*: `WHITESPACE_ONLY` goes through a
token-only path that never builds an AST. So a rung passing at `WHITESPACE_ONLY`
and failing at `SIMPLE` means "different code", not "optimization broke it", and
agreement at `WHITESPACE_ONLY` is not evidence about the AST emitter at all. Both
defects here were invisible at `WHITESPACE_ONLY` for exactly that reason.
