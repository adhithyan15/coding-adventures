# CLOC42 — bounded expression chains in closurec (#16009)

**Status:** implemented in the PR that fixes #16009.
**Crates:** `javascript-parser` (spine measure), `closurec` (limit, compiler
stack, diagnostic).

## Problem

A long flat chain such as `x + x + … + x` (about 1,000 terms, a few KB of
input) aborted closurec with `thread 'main' has overflowed its stack`
(SIGABRT, exit 134). There was no diagnostic and no output, and a caller could
not catch the failure.

Two facts combine:

1. The grammar parses a chain as one flat CST node (`additive_expression` with
   N operands), so the parser's rule-depth guard (`DEFAULT_MAX_RULE_DEPTH`,
   128) never sees it. The bridge then folds the chain into a left-nested typed
   AST: `((x + x) + x) + …`, N levels deep.
2. The passes and the emitter walk that AST recursively. DCE ran on the
   caller's stack and overflowed first. Constant-fold and the emitter each
   already ran on their own 128 MiB worker, so they only moved the ceiling.

Other deep shapes, such as nested parentheses, arrays, objects, blocks,
ternaries and unary chains, already fail cleanly at parse with the rule-depth
guard. The crashing shapes are exactly the flat chain rules:

| Grammar rule | Example |
|---|---|
| `additive_expression`, `multiplicative_expression`, `shift_expression`, `relational_expression`, `equality_expression`, `bitwise_*_expression`, `logical_*_expression`, `nullish_coalescing_expression`, `exponentiation_expression` | `a + b + c …`, `a && b && c …` |
| `member_expression`, `optional_chain_expression` | `a.b.c.d …` |
| `call_expression`, `new_expression` | `f()()() …` |

`javascript-parser` already measured these chains for its division-only entry
(`check_division_only_spines`, limit 512). closurec calls
`parse_javascript_typed` and `bridge::grammar_to_program` directly, so the
measure never ran on its path.

## Design

### 1. The spine measure becomes public

`javascript_parser::check_chain_spine(root, max_spine)` is the existing
iterative measure with the limit as a parameter:

```text
height(node) = max(height(child) for child) + width(node) + 1
width(node)  = CST children − 1   for the chain rules above
             = 0                  otherwise
```

The input is accepted iff every `height ≤ max_spine`. The walk keeps an
explicit stack, so measuring a hostile input cannot overflow either.

`width` counts every CST child, operator tokens included, so links cost
different amounts per shape:

| Link | Units per link |
|---|---:|
| `+ x`, `&& x`, `.a`, `?.a` | 2 |
| `[0]` | 3 |
| `()`, `(1)` | 1 |

On rejection it returns `ChainSpineError { max_spine, line, column }`, which
locates the CST node where the measure first crossed the limit. For one long
chain, that is the chain's first token. Its message reads:

```text
JavaScript expression chain at L:C is nested too deeply to compile (spine limit N)
```

The division-only entry keeps limit 512 and its existing message.

### 2. closurec bounds every typed parse

closurec checks every typed parse against `MAX_CHAIN_SPINE = 16_384` before
calling the bridge.

- **SIMPLE/ADVANCED pipeline.** A rejected input fails at the parse stage:
  `<LEVEL> compilation failed at parse stage: <message above>`. The exit
  status is 1, the same as any parse failure, and nothing is written to stdout
  or the output file.
- **Both `--externs` readers.** A too-deep externs file is handled like any
  other externs file that fails to parse. The function-name reader skips it,
  and the property-boundary reader disables property renaming.

### 3. The compile runs on a compiler thread

`run_compiler` runs the whole compile on one worker thread with a 1 GiB
stack (`COMPILER_STACK_SIZE`). That covers parse, bridge, every pass, the
emitter, externs, and dropping the AST. The reference compiler also runs on a
thread with an enlarged stack. The size is a reservation, so pages are touched
only as deep recursion needs them. Without the worker, the reachable depth
would depend on the platform's main-thread stack: 8 MiB on macOS and Linux,
1 MiB on Windows. With it, the depth is the same everywhere. A panic in the
worker is re-raised on the caller. If the thread can't be spawned (for
example because an address-space or thread limit can't hold the
reservation), the compile fails closed. It returns `CompilerError::CompilerThread`
and exits 2. Falling back to the caller's smaller stack would let an input
inside the bound overflow it.

Constant-fold already ran on its own worker. Its stack (`FOLD_STACK_SIZE`)
rises from 128 MiB to 1 GiB, because `fold_call` keeps a large frame and was
the binding limit for call chains.

### Why 16,384

Measured in a debug build on macOS aarch64 on 2026-10-19. Each figure is the
largest link count that compiled at ADVANCED:

| Shape | 128 MiB fold worker, before | 1 GiB compiler and fold stacks |
|---|---:|---:|
| `+ x` | ~15,000 | > 90,000 (probe cap) |
| `.a` | ~14,000 | > 90,000 (probe cap) |
| `()` | ~4,150 | ~33,400 |

Call chains are the binding shape, at about 33,400 units. A bound of 16,384
units keeps 2× headroom for them, and much more for every other shape and for
release builds.

The bound admits about 8,000 `+` operands. A release build of the compiler
before this change compiled 8,000-term chains and aborted at 16,000, so the
bound doesn't refuse anything that used to compile. The issue's 10,000-term
reproduction costs about 20,000 units, so it now fails cleanly.

### Memory cost

The reservations are virtual, but a hostile input inside the bound really
does touch its stack. A ~33 KB input of 16,000 calls touched about 630 MiB in
a debug build. The worst case per process is bounded by the reservations:
1 GiB for the compiler thread, 1 GiB for fold, 128 MiB for emit. A service
that compiles untrusted JavaScript in parallel should cap per-process memory.

## Upstream parity

The pinned oracle wasn't run on these inputs, so this bound is a closurec
safety limit and doesn't claim to match upstream. CCR-075 says to refuse what
Closure refuses; it doesn't require refusing exactly that set and nothing
else. If an oracle capture later shows upstream compiles deeper chains,
raising the bound means two steps:

1. Raise the stack reservations, or make the hot walks iterative.
2. Measure again and re-run `regression_chain_depth`.

## Tests

- **`javascript-parser`.** The measure:
  - charges each link by its CST children (two per `+ x`, one per call);
  - accepts at the limit and rejects one below it;
  - locates the chain;
  - charges every chain rule;
  - counts nested chains together;
  - leaves 5,000 sibling statements uncharged.
- **`closurec/tests/regression_chain_depth.rs`.**
  - The issue's 10,000-term reproduction exits 1 with the located diagnostic
    and no stdout, at SIMPLE and ADVANCED, and isn't killed by a signal.
  - For each of six shapes (`+`, `&&`, `.a`, `?.a`, `[0]`, `()`), the deepest
    chain the bound admits compiles at SIMPLE and ADVANCED, and one link more
    fails cleanly.
  - These run in a debug build, so a later change that grows stack frames
    enough to make the bound unsafe aborts this test in CI.
