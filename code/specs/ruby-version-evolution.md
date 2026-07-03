# Ruby version evolution

## Status

Companion to [ruby-parser.md](ruby-parser.md).  Lists the syntax
changes that landed in each Ruby release from 1.0 (1996) through
3.3 (2023), with notes on whether the change is **lexer-visible**,
**parser-only**, or **lib-only** (and therefore not our concern in
this layer).

We model 15 **era versions** — releases that actually changed surface
syntax.  Intermediate point releases (2.0.0 vs 2.0.1, 2.4 vs 2.4.5)
do not get their own grammar file; they inherit from the most
recent era at or before their release.

## Quick reference

| Era    | Released  | Major adds / breaks                                                            |
|--------|-----------|---------------------------------------------------------------------------------|
| 1.0    | 1996-12   | Baseline                                                                        |
| 1.6    | 2000-09   | `__END__`, `__FILE__`, `__LINE__`; reserved `BEGIN` / `END`                     |
| 1.8    | 2003-08   | Block-local `|x|` made stricter; multiple assign refinements                    |
| 1.9.1  | 2009-01   | `{a: 1}` hash, `->()` lambda, `#{}` in `:'..'` symbols, magic encoding         |
| 1.9.3  | 2011-10   | (no surface; fork point — 2.0 forks here)                                       |
| 2.0    | 2013-02   | Keyword args, `**`, `%i[]`, refinements (`using`)                               |
| 2.1    | 2013-12   | Required kwargs `key:`, rational `1r` / complex `1i`                            |
| 2.3    | 2015-12   | `&.`, `frozen_string_literal` magic comment                                     |
| 2.5    | 2017-12   | `rescue` directly inside `do...end` (no `begin`)                                |
| 2.6    | 2018-12   | Endless ranges `(1..)`, `then` in `case/when`                                   |
| 2.7    | 2019-12   | Numbered block params `_1`..`_9`, beginless ranges `(..5)`, `case/in` pattern matching (experimental) |
| 3.0    | 2020-12   | Pattern matching stable, endless method def `def f = ...`, rightward `=>` assignment, one-line `expr in pat` |
| 3.1    | 2021-12   | Hash shorthand `{x:}` (no value), anonymous block `&`                            |
| 3.2    | 2022-12   | Find pattern `[*, x, *]`, anonymous splat forwarding `*` / `**`                  |
| 3.3    | 2023-12   | (no surface; Prism becomes default parser)                                       |

## Detailed deltas

### 1.0 — baseline (1996-12)

Establishes the bulk of Ruby's syntax:
- `def` / `end`, `class` / `end`, `module` / `end`
  - **Default / optional parameters** (`def f(a = 1)`, `def f(a, b = a + 1)`)
    are part of the 1.0 baseline. They are evaluated at CALL time and may
    reference EARLIER parameters. As of `ruby-to-semantic-ir` 0.2.0 (changelog
    0.99.0), the grammar `param` rule is `[ "*" | "**" ] NAME [ EQUALS
    expression ]` and the frontend lowers the default into `Param.default`
    (param-scoped, observing `Feature::DefaultParams`) — closing the original
    Ruby-1.0 gap where the `= <default>` subtree was dropped and, in fact,
    did not even parse. See the `ruby-to-semantic-ir` CHANGELOG.
- `if` / `elsif` / `else` / `end`, `unless`, `while`, `until`, `for`
- `case` / `when` / `else` / `end`  (no `in` patterns yet)
- `begin` / `rescue` / `ensure` / `end`
- Blocks: `do ... end` and `{ ... }`
- String literals: `"..."`, `'...'`, backticks, `%q{...}` / `%Q{...}` / `%w{...}` / `%r{...}` / `%s{...}` / `%x{...}`
- Heredocs: `<<X`, `<<-X` (the `<<~` squiggly form is NOT in 1.0)
- Symbols: `:foo`, `:"foo bar"`
- Regex: `/.../` with flags `i`, `m`, `x`, `o`
- Hash with hash-rocket: `{ :a => 1 }`
- Range: `..`, `...`
- Operators: `+ - * / % ** == != < > <= >= <=> && || ! & | ^ ~ << >> = += -= *= /= %= **= &= |= ^= <<= >>= &&= ||= => . :: ?:`
- Globals (`$x`), instance vars (`@x`), class vars (`@@x`)
- `nil`, `true`, `false`, `self`
- `__FILE__`, `__LINE__` (already in 1.0)
- All percent literals exist
- All numeric literals: `0xFF`, `0o77`, `0b101`, `1_000`, `1.5e10`

**Lexer-visible new behaviour vs nothing**: this *is* the baseline.

### 1.6 — refinement of reserved words (2000-09)

- `BEGIN { ... }` / `END { ... }` blocks accepted at top level
- `__method__` NOT yet present
- Minor lex-state cleanup in MRI; from our perspective: no
  user-visible new tokens

**Lexer-visible**: `BEGIN` / `END` reserved (already were keywords
in 1.0, just less consistently).

### 1.8 — pre-modern stable (2003-08)

- Block parameters with semicolon-introduced locals: `do |x; y, z|`
  (y and z are block-local, not captured)
- Multiple assignment lhs/rhs splat refinements (`a, *b = [1, 2, 3]`)
- `respond_to?` etc. become standard (lib, not syntax)

**Lexer-visible**: block-local `;` inside `|...|` — needs a new
tiny state for "after `;` inside block params".

### 1.9.1 — the big break (2009-01)

The most disruptive syntax change in Ruby's history:

- **Hash shorthand**: `{a: 1, b: 2}` ≡ `{:a => 1, :b => 2}`
- **Lambda literal**: `->(x, y) { x + y }`
- **`__method__`** added (returns current method name as symbol)
- **`__ENCODING__`** added (returns current source encoding)
- **Magic encoding comment**: first or second line may say
  `# coding: utf-8` (or `# encoding: utf-8`); changes how byte
  sequences in string literals are interpreted
- **Block-local variable syntax** standardised: `do |x; y, z|`
  formally documented
- **String literal encoding** drives byte interpretation; affects
  `\u{XXXX}` escapes (1.9+ only)
- **Strings are no longer enumerable as bytes** — affects parsing
  of some constructs

**Lexer-visible**: yes — the hash-shorthand requires `NAME :` to
sometimes lex as a hash key (in `EXPR_LABEL` state), and lambda
`->` is a new token.

**Version gate**: if `version < 1.9.1`, reject `->`, `{a: 1}`
shorthand, and `\u{...}` escapes.

### 1.9.3 — fork point (2011-10)

No new surface syntax.  We pin this as a fork point because the
2.x series semantically starts here.

### 2.0 — keyword arguments (2013-02)

- **Keyword arguments**: `def foo(x:, y: 10)` defines two kwargs,
  the first required, the second with default
- **Double-splat**: `def foo(**rest)` collects all keyword args
- **`%i[a b c]`** and **`%I[]`** for symbol arrays
- **Refinements**: `module M; refine X do ... end; end`, `using M`
  at module/file scope
- **Module#prepend** (lib)
- **`Module#using`** (parsed but resolution is later)

**Lexer-visible**: `%i` / `%I` percent literals, `:` after a name
in method-def args needs to lex as kwarg-marker (new state needed
inside the parameter list).

### 2.1 — kwargs and numeric types (2013-12)

- **Required keyword args**: `def foo(x:)` (no default = required)
- **Rational literal**: `2.5r`, `3r`
- **Complex literal**: `2i`, `3.0i`
- **`Module#def_method_missing`** (lib)
- **`Binding#local_variable_set`** (lib)

**Lexer-visible**: number suffixes `r` and `i` extend the numeric
token grammar.

### 2.3 — safe navigation (2015-12)

- **Safe-navigation operator** `&.`: `obj&.foo` is `nil` if `obj` is
  `nil`, else `obj.foo`
- **`<<~`** (squiggly heredoc): strips common leading whitespace
- **`frozen_string_literal: true`** magic comment freezes all
  string literals in the file
- **Hash#dig / Array#dig** (lib)

**Lexer-visible**: `&.` is a new operator token, `<<~` is a new
heredoc opener with `Squiggly` kind in the heredoc spec.

### 2.5 — rescue in do/end (2017-12)

- **Implicit `begin`** for `rescue` inside `do...end` or method
  definitions: previously required wrapping in `begin...end`
- **`yield_self`** (lib, now `then`)

**Lexer-visible**: no new tokens.  Parser-only — `rescue` after a
`do...end`-style block body now reduces to a different production.

### 2.6 — endless ranges and `then` in case (2018-12)

- **Endless range**: `(1..)` means "1 to infinity"
- **Method composition** `>>` and `<<` are still binary operators
  (lib feature, doesn't change syntax)
- **`else` in `case` without `when`**: was always allowed, now formalised

**Lexer-visible**: `(1..)` requires that `..` followed by `)` /
`]` / newline / `,` is a *unary* range close, not "incomplete
operator".

### 2.7 — pattern matching arrives (2019-12)

- **Numbered block parameters**: `[1, 2, 3].each { puts _1 }`
- **Beginless range**: `(..5)` means "negative infinity to 5"
- **Pattern matching** with `case / in`: `case x in [a, b] then ... end`
- **`Comparable#clamp(range)`** (lib)
- **Method reference operator** `.:` was experimental; **removed**
  before final 2.7 (do not parse)

**Lexer-visible**: `_1` through `_9` need oracle-driven
reclassification (only valid inside un-explicit-parameter blocks);
beginless ranges require the same `..` symmetry as 2.6's endless
ranges.

**Parser-only**: `case / in` introduces a new grammar production
for patterns.

### 3.0 — pattern matching stable, endless methods (2020-12)

- **Pattern matching** stabilised (no more experimental warning)
- **Endless method definition**: `def foo(x) = x * 2`
- **Rightward assignment**: `42 => x` binds `x` to `42`
- **One-line pattern matching**: `expr in pattern` returns boolean
- **`Hash#except`** (lib)
- **`String#scrub` defaults** (lib)

**Lexer-visible**: `=>` already a token (hash rocket); rightward
assignment is a parser-level reinterpretation in statement position.
`def foo(x) = body` is parser-level — after `def NAME(args)`, the
parser looks for `=` and switches productions.

### 3.1 — hash shorthand without value (2021-12)

- **Hash shorthand**: `{x:}` ≡ `{x: x}` — bare label means "use the
  local of the same name"
- **Anonymous block forwarding**: `def foo(&); bar(&); end`
- **Pinning expressions in patterns**: `case x in ^(Foo.bar) then ... end`
- **`Struct.new` with keyword init** (lib)

**Lexer-visible**: bare-label hash entry needs the parser to know
"after `{` or `,`, a `NAME :` not followed by an expression is a
shorthand entry."  Parser-level disambiguation.

### 3.2 — find patterns and anonymous splats (2022-12)

- **Find pattern**: `case x in [*, 7, *] then ... end` — captures
  middle.
- **Anonymous splat forwarding**: `def foo(*); bar(*); end`
- **Anonymous double-splat**: `def foo(**); bar(**); end`
- **`Data.define`** for value classes (lib)

**Lexer-visible**: no new tokens.  Parser productions for patterns
are extended.

### 3.3 — pin the Prism era (2023-12)

- No surface syntax changes
- **Prism** parser becomes the default in MRI (replacing parse.y)
- Pinned as "what we emit by default"

## Inheritance rules

Each per-version TOML extends the immediately prior era:

```
ruby-1.0   (base)
  └─ ruby-1.6
       └─ ruby-1.8
            └─ ruby-1.9.1
                 ├─ ruby-1.9.3        (lib-only, identity inheritance)
                 ├─ ruby-2.0
                 │    └─ ruby-2.1
                 │         └─ ruby-2.3
                 │              └─ ruby-2.5
                 │                   └─ ruby-2.6
                 │                        └─ ruby-2.7
                 │                             └─ ruby-3.0
                 │                                  └─ ruby-3.1
                 │                                       └─ ruby-3.2
                 │                                            └─ ruby-3.3
```

A grammar file declares `extends = "ruby-X"` and supplies only the
overrides for tokens / states / actions added in that era.
`grammar-tools` resolves the inheritance graph at compile time and
emits one *flat* per-version state machine (no inheritance
indirection at runtime).

## Old-version policy

Versions 1.0 / 1.6 are forward-derived: the canonical TOML starts
from 1.8 and **removes** features that don't exist yet (lambda
`->`, hash shorthand, etc.).  This is more practical than writing
1.0 from scratch — most of its syntax overlaps 1.8.

When tests want to assert "this code is invalid in 1.0", they pass
`version="1.0"` and expect the lexer/parser to reject the lambda /
hash-shorthand / etc.  The TOML's `version_gate` action verb
(§5 in [ruby-lexer-state-machine.md](ruby-lexer-state-machine.md))
handles this declaratively.

## Caveats

- **YARV-specific syntax** like `__send__` is library-level, not
  parser-level.
- **`require`** and **`require_relative`** are method calls, not
  syntax; the parser does not understand cross-file references.
- **Heredoc `<<-` vs `<<~`**: `<<-` indents the terminator (1.0+);
  `<<~` strips common leading whitespace from the body (2.3+).
- **Hash methods `to_h`** (lib, not syntax).
- **Bignum / Fixnum**: unified into `Integer` in 2.4; **not lex-
  visible** (the lexer always emits `INT`, the type distinction is
  runtime).

## Out of scope

- Method-visibility keywords (`private`, `public`, `protected`):
  parsed as method calls, semantics applied later.
- `prepend` (2.0+), `extend`: method calls, not syntax.
- `String#tr_s`, `Array#zip`: library only.
- DTrace / TracePoint hooks: not surface syntax.
- The `it` implicit block param (3.4 preview): post-3.3, not
  covered by this spec.
