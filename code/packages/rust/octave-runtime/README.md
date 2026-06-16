# Octave Runtime

GNU Octave on the MATLAB stack. **Octave is to MATLAB what R is to S** — a
compatible reimplementation with a few syntactic additions. Item **MA-3e** of
the historical math-languages roadmap; see
[`MA01`](../../../specs/MA01-matlab-language.md) §5.

## How it works

Where R needed its own lexer/parser (its `_`/`->>` differences are pervasive),
Octave's departures from MATLAB are a small, *local* set of surface forms — so
this crate reuses the **entire** MATLAB frontend
([`matlab-runtime`](../matlab-runtime), and through it the MATLAB lexer/parser and
[`array-runtime`](../array-runtime)) behind a thin **source-compatibility shim**,
`octavify`, that normalizes Octave syntax to MATLAB before evaluation. The matrix
engine, the **GPU-lowering of `*`**, 1-based indexing, and control flow are all
inherited unchanged.

| Octave | becomes (MATLAB) |
|--------|------------------|
| `# comment` | `% comment` |
| `endif`/`endfor`/`endwhile`/`endfunction`/`endswitch`/`end_try_catch` | `end` |
| `!=` | `~=` |
| `!` | `~` |

The shim is **string- and comment-aware** (and handles MATLAB's transpose-vs-quote
ambiguity), so `'#tag'`, `"a != b"`, and `A'` are never rewritten.

```rust
use coding_adventures_octave_runtime::eval;
let out = eval("if !0\n  x = 6;\nendif\nx * x\n").unwrap();
assert!(out.contains("36"));
```

### Deferred

Octave's `++`/`--` and `do…until` (no MATLAB equivalent) are left as-is and
currently error; documented deferrals, plus everything `matlab-runtime` defers.

## Testing

```sh
cargo test -p coding-adventures-octave-runtime
```
