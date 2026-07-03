# sir-variadic-params — variadic parameter kinds for SIR `Param` (M3)

## Status

New. Closes the **v0 def-side variadic limitation** recorded in
`ruby-to-semantic-ir`'s `lower_def_statement` (and `lower_block`):

> the splat-ness of a param is LOST at the SIR level. `Param` has no variadic
> flag, so a splat param lowers to a regular `Param` with the bare Name … the
> parameter [is treated] as positional rather than variadic.

So `def f(*rest); end` and `def g(**opts); end` currently lower to `f(rest)` /
`g(opts)` — the emitted Python/TypeScript declares a *fixed positional*
parameter, and a caller using the variadic-ness (`f(1, 2, 3)`) breaks. This is
the **definition** side; Q9c already handles the **call** side (`f(*arr)` →
`*arr` / `...arr`). M3 makes the def side faithful.

This is a **`semantic-ir` core schema change** (a new field on `Param`), hence
this spec precedes the implementation per the repo's specs-first rule.

## Core schema change

`Param` gains a `kind` field:

```rust
/// How a parameter binds its arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParamKind {
    /// An ordinary positional parameter (`x`). The default.
    #[default]
    Required,
    /// A rest parameter (`*rest`) — collects trailing positional arguments
    /// into a sequence.
    Rest,
    /// A keyword-rest parameter (`**opts`) — collects trailing keyword
    /// arguments into a map.
    KwRest,
}

pub struct Param {
    pub name: String,
    pub sir_type: Option<SirType>,
    pub kind: ParamKind,   // NEW
    pub span: Span,
}
```

`ParamKind::Required` is the default, so every existing `Param { … }`
construction is updated to add `kind: ParamKind::Required` (semantic-ir, the
ruby/twig lowerers, and backend tests). Backends read parameters by field
access (`p.name`), so the added field does **not** break their reads — only
literal *constructions* change.

The SIR text printer renders `*name` / `**name` for the two variadic kinds so a
round-tripped module is faithful.

## Validation (`semantic_ir::validate`)

Per parameter list (a `Function`'s `params`):

- **At most one** `Rest` parameter and **at most one** `KwRest` parameter.
- Ordering (Ruby-faithful, light v0): a `Rest` precedes any `KwRest`; both
  follow the required positionals. A violation is a validation error, not a
  panic.

The reserved trailing block parameter `__sir_block__` (Q9e) is always
`Required` and always last; it is unaffected.

## Frontend lowering (`ruby-to-semantic-ir`)

`lower_def_statement` (and the endless-def / block-param paths) already detect
the leading `*` / `**` prefix token on a `param` node — today they *skip* it and
emit a bare `Required` `Param`. The change: when the prefix is present, set
`kind: Rest` (`*`) or `KwRest` (`**`); otherwise `Required`. The lossy-limitation
comment is removed. No grammar change (the parser already accepts `*`/`**` on
def params).

## Backend emission

| `ParamKind` | Python (`semantic-ir-to-python`) | TypeScript (`semantic-ir-to-typescript`) |
|---|---|---|
| `Required` | `name` | `name` |
| `Rest`     | `*name` | `...name` |
| `KwRest`   | `**name` | *(v0 limitation, see below)* |

- **Python** has faithful native forms for both — `*args` and `**kwargs`.
- **TypeScript** has native rest (`...name`) for `Rest`. JavaScript has **no
  keyword-argument call form**, so a `KwRest` def parameter has no faithful
  native declaration. v0: emit it as a trailing ordinary object parameter
  (`name`) and document the limitation — the call side (Q10f) already collapses
  `**h` into a single merged trailing object, so a `KwRest` def parameter binds
  that object. (Mirrors the existing TS double-splat call-position treatment.)
- **Go / Rust backends** are deferred (out of scope for the Python+TS plan).
  They must still **compile** against the new field; their parameter emission
  may treat `Rest`/`KwRest` as a best-effort positional for now (documented),
  since the Ruby frontend is the only producer of variadic params and those
  backends do not yet accept the Ruby feature surface.

## Verification

- **Core**: a `validate` unit test rejecting two `Rest` params / a `KwRest`
  before a `Rest`; a printer round-trip test for `*x` / `**x`.
- **Frontend**: lowering-assertion tests — `def f(*r)` → one `Param{kind:Rest}`;
  `def g(**o)` → `Param{kind:KwRest}`; `def h(a, *r)` → `[Required, Rest]`.
- **Backends**: emitted-shape tests — Python `def f(*r):` / `def g(**o):`;
  TypeScript `function f(...r)`; plus execution-proof through `python3`/`node`
  (skip gracefully if the interpreter is absent) that a variadic def collects
  its arguments (`def f(*a); a.length; end; f(1,2,3)` → `3`).
- `cargo test -p semantic-ir -p ruby-to-semantic-ir -p semantic-ir-to-python -p semantic-ir-to-typescript`
  (plus a compile check of the go/rust backends).

## Implementation notes (divergence from the original spec)

Two faithfulness refinements surfaced during implementation and were added
beyond the table above:

- **Python Rest-param list normalization.** Python's `*rest` binds a *tuple*,
  but SIR sequence semantics (and Ruby's `*rest`, an `Array`) require a *list* —
  every downstream sequence op (`len`, indexing, dispatched `.map`/`.length`)
  is keyed to `list`. So the Python backend rebinds each `Rest` param to
  `list(...)` in the function prologue. (`**opts` already binds a `dict`,
  matching SIR's map, so no fixup. TypeScript's `...rest` is already a real JS
  `Array` = SIR sequence, so it needs none either.)
- **OOP-import gating widened.** Making a rest param *useful* means calling
  Array methods on it (`def f(*a); a.length; end`), which is method dispatch
  (`BuiltinCall("__method__", …)`). Both backends' `uses_oop` previously gated
  the `sir-runtime-oop` import only on the OOP *features* (Classes/Modules/…),
  so a class-less dispatch program emitted an undefined `call_method`. `uses_oop`
  now also fires on the `__method__` / `__scope__` dispatch builtins. (Pre-existing
  latent gap, fixed here because M3's execution-proof is the first class-less
  dispatch program exercised end-to-end.)

## Out of scope (documented, honest)

- Required-keyword params (`def f(a:)`) and optional-with-default params
  (`def f(a = 1)`) — separate from the rest/kwrest variadic kinds; not added
  here.
- TS keyword-rest faithful semantics (no JS kwargs) — the v0 object-parameter
  treatment above.
- Go / Rust faithful variadic emission — deferred with the rest of those
  backends.
