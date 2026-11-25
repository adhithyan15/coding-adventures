# test-bootstrap (C)

The C property-based testing engine, conforming to the shared `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1`, 25 cases) with exact generator outputs, exact
shrink minimals **and step counts**. Spec: `code/specs/VER05i-c-pbt-engine.md`.

Pure ISO C17, libc only, no extension and no third-party code. It compiles under
GCC, Clang and MSVC through the shared [`iso-harness`](../iso-harness/) with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror`.

## What it does

A property-based test checks a *claim about every input a generator can
produce*, not one example. The engine:

1. **generates** candidates from a deterministic SplitMix64 stream (same seed,
   same candidates, in every language lane);
2. **shrinks** a failing candidate to a witness a human can read
   (`[913,-4,77,0,2]` becomes `[8,5,8]`), keeping every generator's bounds;
3. **judges honestly**: zero cases run is never a pass, the reported witness is
   re-verified, a refused or starved check is never a pass, and in CI mode the
   verdict carries no seed and no witness.

## Where it sits

```text
 pbt_check(parent, seed, gen, prop, opts)        pbt_corpus_evaluate(json)
        |                                               |
   pbt_gen: sample + shrink function pointers  <--------+
        |   map filter flat_map one_of frequency list tuple custom
   integrated shrinkers (integer walk, in-place sequence shrinker)
        |
   pbt_budget tree  /  pbt_rng (SplitMix64, FNV-1a)  /  pbt_big (bignum)
```

It sits at L2 of the VER testing pyramid (runtime generative testing), above the
L0 harness and below the prover.

## Example

```c
#include <stdio.h>
#include <stdlib.h>

#include "pbt.h"

/* 1 = the property holds, 0 = it fails, negative = the property itself errored. */
static int sum_is_small(void *user, pbt_budget *scope, const pbt_val *v)
{
    long sum = 0;
    size_t i;
    (void)user; (void)scope;
    for (i = 0; i < v->u.list.n; i++) {
        sum += (long)pbt_big_to_i64(&v->u.list.items[i].u.i);
    }
    return sum <= 20;
}

int main(void)
{
    pbt_gen *u8 = pbt_gen_u8();
    pbt_gen *list = pbt_gen_list(u8, 0, 8);       /* constructors borrow: release yours */
    pbt_options o = pbt_options_default();
    pbt_prop prop = { sum_is_small, NULL };
    pbt_verdict v;
    int ok;

    pbt_gen_release(u8);
    o.ci = pbt_ci_from_value(getenv("CI")) ? PBT_CI_ON : PBT_CI_OFF;  /* the caller reads the environment */
    (void)pbt_check(NULL, 42, list, prop, &o, NULL, &v);
    if (!v.passed) {
        puts(v.message);   /* local mode: seed, initial and minimal witness; CI mode: only a digest */
    }
    ok = v.passed;
    pbt_verdict_free(&v);
    pbt_gen_release(list);
    return ok ? 0 : 1;
}
```

## Things worth knowing

- **Integers** are `pbt_big` (a fixed-limb bignum, up to 1000 bits), because
  `unsigned __int128` is not ISO C. A `pbt_val` of kind `INT` holds one inline.
- **Single-threaded.** There is no lock and no global state besides the test
  seams in `pbt_hooks.h`. The shrink budget is an explicit `pbt_budget *`
  parameter (pass `NULL` at the top level; a property that wants to run a nested
  check passes the scope it was handed).
- **Properties are `const`.** They must not mutate or free the value: the
  shrinkers hand them views into the working array rather than copies.
- **Errors are values.** Constructors return `NULL`, entry points return a
  `PBT_E_*` status, and a refused call yields a *failing* verdict, never a pass.
- **No timeout.** A property that never returns hangs the run (`property-timeout`
  is registered but never produced, as in every lane).
- A fixed `CI` redaction caveat applies: `repro-fnv` is a correlation id, not a
  secret; it can be brute-forced over a small witness space.

## Building and testing

```sh
sh BUILD              # every compiler found, strict ISO, then ASan + UBSan
sh tools/coverage.sh  # on-demand llvm-cov report (not a gate)
```
