# jit-compiler (C)

**CCPP02 port campaign — bucket A (pure-ISO), port #2.** The *management* layer of
a JIT: it decides which bytecode is worth compiling and tracks the native blocks
that would replace interpretation — but it deliberately does **not** generate
machine code. The C port of the Rust `jit-compiler` crate, a pure-ISO crate that
needs no OS, so it rides the `iso-harness` (links nothing, `-pedantic-errors` /
`/permissive-`).

> Not to be confused with os-platform's `jit` primitive, which allocates real
> executable memory. This package emits nothing and touches no OS — it is a pure
> bookkeeping layer, honest about the fact that the hard code-generation work is
> not done.

What it provides:

- **hot-path profiling** — a per-bytecode-offset execution counter;
- **threshold detection** — `observe_execution` reports the exact call on which a
  path *transitions* to hot (its count reaches the threshold);
- **a shell native-block registry** — install / look up / remove blocks whose
  machine-code buffer is always empty (the shape of a future JIT); and
- **deoptimization** — remove a block and hand it back to the caller.

```c
jit_config cfg;
jit_config_new(JIT_ISA_X86, /*hot_threshold=*/3, &cfg);   /* threshold must be > 0 */

jit_compiler *jit;
jit_compiler_create(&cfg, &jit);

int hot;
jit_compiler_observe_execution(jit, 24, &hot);  /* count 1 → hot=0 */
jit_compiler_observe_execution(jit, 24, &hot);  /* count 2 → hot=0 */
jit_compiler_observe_execution(jit, 24, &hot);  /* count 3 → hot=1  (the transition) */

const char *why[] = { "locals stay integers" };
const jit_native_block *blk;                    /* BORROWED (until next mutation) */
jit_compiler_install_shell_block(jit, 24, why, 1, &blk);

jit_native_block gone;
int found;
jit_compiler_deoptimize(jit, 24, &gone, &found); /* MOVES the block out to you */
jit_native_block_free(&gone);                    /* …so you free it */

jit_compiler_destroy(jit);
```

| Function | Purpose |
|----------|---------|
| `jit_config_new` | validated config (`hot_threshold > 0`, the Rust `assert!`) |
| `jit_compiler_create` / `jit_compiler_destroy` | make / free a compiler |
| `jit_compiler_observe_execution` | count one run; `*became_hot` = the hot transition (true once) |
| `jit_compiler_profile` | snapshot (count + `is_hot`) for an offset, if it has run |
| `jit_compiler_install_shell_block` | register a shell block (empty code, copied assumptions); returns a **borrow** |
| `jit_compiler_has_native_block` / `jit_compiler_native_block` | test / borrow a registered block |
| `jit_compiler_deoptimize` | remove a block and **move** it to the caller |
| `jit_compiler_config` | borrow the configuration |
| `jit_native_block_free` | free a *moved-out* block |

## Faithfulness notes

- **`became_hot` is the transition, not the state.** Rust returns
  `*count == hot_threshold`, true exactly on the call that reaches the threshold.
  `profile().is_hot` is the *state* (`count >= threshold`), true from then on.
- **`Option<T>` → status + flag.** `profile` and `deoptimize` return their
  `Option` through a `*found` flag beside the out-value.
- **Borrow vs. move.** `install_shell_block` / `native_block` return a pointer
  *into* the registry (Rust's `&NativeBlock`) — valid only until the next
  registry-mutating call, since `install` may `realloc` the backing array.
  `deoptimize` returns an *owned* block (Rust's `NativeBlock` by value); the
  caller frees it with `jit_native_block_free`. The store removes the slot
  **without** freeing the moved pointers, so there is no double-free.
- **`BTreeMap` → growable arrays.** Both maps (offset→count, offset→block) are
  only ever point-accessed by key, so their ordering is not observable; the C
  backs each with a plain array + linear scan (as `vault-revisions` does).
- **Fallible allocation.** The Rust aborts on OOM; the C returns `JIT_ERR_NOMEM`
  and unwinds cleanly (a failed install undoes its assumptions copy).

## Build & test

Pure ISO, no OS, no link libraries.

```sh
cd code/packages/c/jit-compiler
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 343 checks / 0 failed under gcc + clang with `-pedantic-errors`;
clean under ASan+UBSan; 0 leaks.

## Layout

```
jit-compiler/
├── include/jit_compiler/jit_compiler.h   # public API
├── src/jit_compiler.c                      # profiler + registry — one pure-ISO source
├── tests/jit_compiler_test.c               # the 4 Rust tests + ownership/NOMEM/invalid
├── tools/run.sh  · run.ps1                   # build via iso-harness (links nothing)
├── BUILD  · BUILD_windows                    # per-OS build drivers
└── .gitignore
```
