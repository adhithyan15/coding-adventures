---
category: Compiler / VM / language pipeline
---

# Being more aggressive than the oracle is a divergence too, and the open-world rule that licenses a fold is about scope, not compilation level

CLOC28 taught `inline-variables` to resolve `o.a` against `var o = {a:1}`. The
four ADVANCED ladder rungs it targeted went green, the guards all held, and the
change looked finished. The ladder gate then failed on four rungs I had not
been thinking about — the **SIMPLE** halves of the same programs:

```
ladder_t5_object_simple: diverged from upstream with no ledger entry.
    upstream: var o={a:1,b:2};console.log(o.a);
    ours:     var o={a:1,b:2};console.log(1);
```

`inline-variables` runs at both levels, so the new capability came on at SIMPLE
for free. Upstream declines there, and a pass that optimizes where the
reference compiler does not is a divergence in the *dangerous* direction — the
one that ships miscompiles rather than missed bytes. Issue #15837 had already
flagged exactly this shape on the `let-const` rung ("we are more aggressive
than upstream ... being ahead of the oracle is how miscompiles ship") and I
still walked into it.

**The rule is scope, not level.**

The reflex fix is "gate it to ADVANCED". Probing the oracle first shows that is
the right gate here for the wrong reason:

```text
top-level     SIMPLE    var o={a:1,b:2};console.log(o.a);            =>  unchanged
top-level     ADVANCED  var o={a:1,b:2};console.log(o.a);            =>  console.log(1);
function-local SIMPLE   function f(){var o={a:1};return o.a}f();     =>  function f(){return 1}
function-local ADVANCED function f(){var o={a:1};return o.a}f();     =>  console.log(1);
```

Upstream folds a **function-local** object at SIMPLE. What stops it at the top
level is the open-world assumption — a top-level binding is a property of the
global object, and another script may read or replace it — which is the same
reasoning `run.rs` already gives for gating `remove-unused-vars` and
`treeshake`. The pass in question only collects top-level declarations, so
"ADVANCED only" and "closed-world only" happen to coincide *for what it can see
today*. Write the gate's comment to say that, or the next person widens the
pass to locals, keeps the level gate, and silently loses folds upstream makes.

**What to do.**

* When adding a capability to a pass that runs at more than one level, work out
  what it does at **every** level before running the suite, and probe the
  oracle at each. "The rungs I aimed at went green" is not coverage.
* Ask what *licenses* the transform, not just where upstream happens to do it.
  Open-world vs closed-world, not SIMPLE vs ADVANCED.
* A gate whose justification is narrower than its wording is a trap for the
  next change. Say which fact makes the two coincide and when that stops being
  true.
* The ladder gate earns its keep here: it fails on a rung that *starts*
  agreeing or disagreeing in either direction. Do not reach for the ledger to
  quiet it — an entry recording "we optimize where upstream does not" is a
  record of a bug, not a known gap.
