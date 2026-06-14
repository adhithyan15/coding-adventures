# F10: Declarative Lexer Mode Transitions

## Overview

This spec extends `F04-lexer-pattern-groups.md` with a **declarative transition
table**: a `.tokens` grammar can now say *"when token X is emitted, switch the active
group"* directly in the grammar file, instead of in a hand-written host-language
callback. The generic `GrammarLexer` interprets the table; no per-language imperative
lexer code is required.

F04 gave us pattern groups (named sets of token patterns) and an imperative
`on-token` callback that pushes/pops those groups on a stack. That works, but it has a
cost: every context-sensitive language must ship a hand-written callback in *every*
language port (Python, Ruby, TS, Go, Rust, Elixir), and the callbacks drift. Worse,
`es2022.tokens` already *declares* a `group template:` for template-literal
substitutions — but JavaScript's lexer never registers a callback, so the group is
dead code and template substitutions (and regex-vs-division) cannot be lexed.

F10 closes that seam. The transition logic moves from imperative callback into
declarative grammar data, emitted by the compiler as a static table and interpreted
once by the shared lexer engine.

### The motivating problem: ECMAScript is lexically context-dependent

ECMAScript's lexical grammar is context-dependent *by specification*. ECMA-262 defines
multiple **lexical goal symbols** — `InputElementDiv`, `InputElementRegExp`,
`InputElementTemplateTail` — and states that the syntactic context selects which one
the scanner uses at each position. Concretely:

- `a / b / c` is two division operators; `x = /b/c/g` is one regex literal. Same
  characters; the meaning depends on whether an expression or an operand precedes the
  `/`.
- ``  `head ${expr} tail`  `` requires the scanner to leave template mode at `${`, lex
  `expr` as ordinary tokens, and re-enter template mode at the closing `}`.

A scanner that runs as an independent pass cannot resolve these without context. The
standard solution (used by V8, SpiderMonkey, Acorn) is for the scanner to track a
small amount of lexical state — Acorn's `exprAllowed` flag. F10 makes that state
**declarative**: a flat "mode" register, toggled by a transition table.

### Relationship to existing specs

- **F04-lexer-pattern-groups.md**: the base. F10 keeps F04's groups, stack, callback,
  and backward-compat guarantees verbatim, and *adds* a declarative transition table
  beside the imperative callback. A grammar with no `transitions:` section behaves
  exactly as it does under F04 today.
- **F08-declarative-tokenizer-state-machines.md**: a *different* layer — a
  char-by-char `.lexer.states.toml` transducer for byte/code-point machines like HTML.
  F10 operates on the F04 regex-pattern `GrammarLexer` used by programming-language
  grammars. The two do not overlap; F10 is higher-level (regex patterns, not
  per-character transitions) and reuses the existing group machinery rather than a
  separate automaton.
- **lexer-parser-hooks.md**: batch pre/post-tokenize transforms. F10 transitions fire
  per-token *during* tokenization (like F04 callbacks), not as a batch pass.

## Design Principles

1. **Grammar files stay declarative, and now own the transitions.** F04 said "the
   grammar file never says 'when you see TOKEN_X, switch to group Y'." F10 amends that:
   the grammar file *may* say exactly that, in a structured `transitions:` table — but
   still as data, never as host-language code.
2. **One register, three actions (the flex unification).** A "mode" is just a named
   group plus a start-condition role. There is one active register — the top of the
   existing `group_stack`. `set-mode` replaces it without saving (the flat toggle);
   `push`/`pop` are F04's existing save/restore for nested regions. No second registry,
   no second pattern map.
3. **Data-driven, not generated code.** The compiler emits the table as pure data
   (exactly as it already emits `PatternGroup`s); the shared lexer engine interprets
   it. The disambiguation logic lives once, in the engine — not regenerated per
   language.
4. **Full backward compatibility.** No `transitions:` section and no `start_mode:` ⇒
   byte-identical to F04 behavior. The interpreter early-returns on an empty table.
5. **Callbacks remain an escape hatch.** A grammar may still register an `on-token`
   callback (F04). When both are present, the callback runs first, then the table
   refines. For grammars with neither, there is zero overhead.
6. **Honest about residuals.** The flat-mode model resolves the common cases. It does
   not resolve every `}`/`)` ambiguity; those require a brace/paren-kind stack and are
   documented as a follow-up that extends the *same* table (see [Residual Hard
   Cases](#residual-hard-cases)).

## Extended `.tokens` Format

### `start_mode:` directive (optional)

```
start_mode: default
```

Names the mode the lexer starts in. Defaults to `default` (the implicit F04 group).
Must name `default` or a declared group.

### `transitions:` section (optional)

```
transitions:
  on TOKENS [in MODE] -> ACTION [, ACTION ...]
```

Each indented line is one rule:

- **`TOKENS`** — one emitted token type name, or a parenthesised alternation
  `(A | B | C)`. Names match the *emitted* type (the alias target if the pattern was
  aliased with `-> TYPE`, e.g. `STRING`, not `STRING_DQ`). Promoted keywords match the
  literal `KEYWORD` with an optional value guard (next item).
- **`[in MODE]`** — optional guard; the rule fires only when the active mode equals
  `MODE`. Omit it to mean "in any mode."
- **`-> ACTION, ...`** — ordered actions applied immediately after the token is
  emitted. Actions:
  - `set-mode M` — replace the active mode (top-of-stack) without saving. The flat
    toggle.
  - `push G` — save the current mode and make `G` active (F04 `Push`).
  - `pop` — restore the saved mode (F04 `Pop`; no-op at the floor).
  - `enable-skip` / `disable-skip` — toggle skip-pattern processing (F04
    `set_skip_enabled`), for significant-whitespace regions.

Keyword value form, for disambiguating promoted keywords:

```
on KEYWORD="return" -> set-mode default
```

### Rules

- **Order matters.** Rules are matched top-to-bottom; the **first** matching rule wins
  and its actions apply (then matching stops for that token). This is deterministic and
  documented.
- **Transitions fire only on emitted tokens.** Suppressed tokens (F04 `suppress`) and
  synthetic `emit()` tokens do not trigger transitions — consistent with the
  `previous_token` lookbehind semantics.
- **Targets must exist.** Every `set-mode`/`push` target and every `in MODE` guard must
  name `default` or a declared group; the validator rejects undefined targets (a
  silent fallback to `default` would mask grammar bugs).
- **Reserved section names.** `transitions`, `modes`, and `start_mode` join F04's
  reserved set (`default`, `skip`, `keywords`, `reserved`, `errors`) and cannot be used
  as group names.

## Data Structures

### ModeTransition (new)

```rust
/// One declarative transition rule. Pure data; interpreted by the lexer.
pub struct ModeTransition {
    /// Emitted token type-names that trigger this rule (alias targets;
    /// "KEYWORD" for promoted keywords).
    pub on_tokens: Vec<String>,
    /// Optional keyword-value guard, e.g. Some("return"). When set, the rule
    /// fires only if the emitted token's value equals this.
    pub on_value: Option<String>,
    /// Optional active-mode guard. None = "in any mode".
    pub in_mode: Option<String>,
    /// Ordered actions applied after the token is emitted.
    pub actions: Vec<TransitionAction>,
    pub line_number: usize,
}

pub enum TransitionAction {
    SetMode(String),  // replace active mode (top-of-stack) without saving
    Push(String),     // save current mode, make target active
    Pop,              // restore saved mode
    EnableSkip,
    DisableSkip,
}
```

### TokenGrammar (extended)

Two fields are added beside F04's `groups`:

```rust
    /// The mode the lexer starts in. None => "default".
    pub start_mode: Option<String>,
    /// Declarative transition table. Empty => F04 behavior (no transitions).
    pub transitions: Vec<ModeTransition>,
```

When `transitions` is empty and `start_mode` is `None`, the grammar is byte-identical
in behavior to an F04 grammar.

## Lexer Interpreter Semantics

The transition table is the table-driven analogue of F04's callback. The model unifies
cleanly with the existing F04 group stack:

- The active pattern set is already selected by the top of `group_stack`
  (F04). F10 adds no second register.
- `group_stack` is initialised to `[start_mode]` (default: `["default"]`, unchanged).
- After a token is emitted — the same point where F04 applies callback push/pop, and
  after `previous_token`/`bracket_depths` are updated — the engine consults the table:

```
fn apply_transitions(emitted_token):
    if table is empty: return                  # F04 backward-compat
    active = group_stack.last()                # the current mode
    key = transition_key(emitted_token)        # e.g. "STRING", "RPAREN", "KEYWORD"
    for rule in rules_for(key):                # first-match-wins
        if rule.in_mode  and rule.in_mode  != active:        continue
        if rule.on_value and rule.on_value != token.value:   continue
        for action in rule.actions:
            SetMode(m) -> *group_stack.last_mut() = m   # flat: no depth change
            Push(g)    -> group_stack.push(g)           # nested: +1 depth
            Pop        -> if depth > 1 { group_stack.pop() }
            EnableSkip / DisableSkip -> skip_enabled = true / false
        break                                  # first matching rule wins
```

`SetMode`, `Push`, and `Pop` all converge on the single "active group =
`group_stack.last()`" register the matcher already reads — this is the clean
unification with F04's stack. `SetMode` keeps the depth constant (a flat toggle);
`Push`/`Pop` change it (nested regions).

### Flat modes inherit the default patterns

F04 matches a non-default group's patterns **exclusively** — correct for XML
regions (`tag`/`cdata` have their own small token sets). But a flat mode like
JavaScript's `div` is *nearly identical* to `default`: it differs only by reading
`/` as division instead of regex. Exclusive matching would force the `div` group
to redeclare the entire grammar. So F10 distinguishes the two by how a group is
*entered*:

- A group reached via **`set-mode`** is a **flat mode**: the matcher tries its own
  patterns first, then **falls through to the default group's patterns**. The mode
  only declares its *overrides* (`div` = `{SLASH_EQUALS, SLASH}`), which win on
  priority over the inherited `REGEX`.
- A group reached via **`push`** is a **nested region**: it stays **exclusive**
  (F04 semantics), so an XML `tag` region never matches default content.

The classification is derived automatically from the transition table (a group is
a flat mode iff some rule `set-mode`s to it and no rule `push`es it) — no extra DSL
surface. Because matching is first-pattern-wins by order, a flat mode's overrides
are simply tried ahead of the inherited defaults; nothing is removed.

**Coexistence with callbacks.** If an `on-token` callback is registered (F04), it runs
first and applies its actions, then the table runs and refines. The table is the
declarative default; callbacks are the escape hatch. For grammars with neither, neither
path runs.

**Indentation/layout modes.** `tokenize_indentation`/`tokenize_layout` use only the
default group; transitions are a no-op there (orthogonal, exactly as F04 groups are).

## JavaScript Profile (the first consumer)

> **Status (landed):** the regex-vs-division table below ships in
> `code/grammars/ecmascript/es2025.tokens` (closurec's default edition) and the
> regenerated `javascript-lexer` `_grammar.rs`. It closes **gap-092 / gap-115 /
> gap-119** — the three byte-identity fixtures `regex_div` / `div_chain` /
> `regex_after_return` are un-ignored and enforced. The shipped operator/keyword
> sets are supersets of the illustrative lists below (the full ES2025 punctuator
> table). Two follow-ups remain: (1) the **template-substitution** rules
> (gap-044, below) are NOT yet wired — templates need the brace-depth guard;
> (2) sibling editions (`es2022`…`es2024`) can adopt the same table when needed
> (trivial: copy the `start_mode`/`group div`/`transitions` block).

### Regex-vs-division (gap-115, gap-119)

Two flat modes: `default` = expression position (a `/` lexes as `REGEX`), `div` =
operand position (a `/` lexes as `SLASH`/`SLASH_EQUALS`). A program begins in
expression position. This is Acorn's `exprAllowed`, declared:

```
start_mode: default

transitions:
  # value-producing tokens -> the next slash is DIVISION
  on (NAME | NUMBER | STRING | BIGINT | REGEX | TEMPLATE_NO_SUB
      | PRIVATE_NAME | RPAREN | RBRACKET) -> set-mode div
  on KEYWORD="this"  -> set-mode div
  on KEYWORD="super" -> set-mode div

  # operators / openers / separators -> the next slash is a REGEX
  on (LPAREN | LBRACKET | LBRACE | COMMA | SEMICOLON | COLON
      | EQUALS | ARROW | QUESTION
      | PLUS | MINUS | STAR | SLASH | PERCENT | STAR_STAR
      | EQUALS_EQUALS | STRICT_EQUALS | NOT_EQUALS | STRICT_NOT_EQUALS
      | LESS_THAN | GREATER_THAN | LESS_EQUALS | GREATER_EQUALS
      | AND_AND | OR_OR | NULLISH_COALESCE | BANG | TILDE
      | PLUS_EQUALS | MINUS_EQUALS) -> set-mode default
  on KEYWORD="return"     -> set-mode default
  on KEYWORD="typeof"     -> set-mode default
  on KEYWORD="delete"     -> set-mode default
  on KEYWORD="void"       -> set-mode default
  on KEYWORD="in"         -> set-mode default
  on KEYWORD="instanceof" -> set-mode default
  on KEYWORD="new"        -> set-mode default
  on KEYWORD="do"         -> set-mode default
  on KEYWORD="else"       -> set-mode default
  on KEYWORD="case"       -> set-mode default
  on KEYWORD="yield"      -> set-mode default
  on KEYWORD="await"      -> set-mode default
```

The `div` mode declares only its slash **overrides** and inherits the rest from
`default` (see [Flat modes inherit the default patterns](#flat-modes-inherit-the-default-patterns)):

```
group div:
  SLASH_EQUALS = "/="
  SLASH = "/"
```

`++`/`--` deliberately emit **no** rule: Acorn leaves `exprAllowed` unchanged across
them (postfix on a value stays `div`; prefix inherits the surrounding expression
position), so the mode is inherited.

### Template substitutions (gap-044)

`es2022.tokens` already has `TEMPLATE_HEAD`/`TEMPLATE_MIDDLE` (each ends with `${`,
opening a substitution) and `TEMPLATE_TAIL`/`TEMPLATE_NO_SUB` (each closes one), plus a
`group template:` holding the middle/tail patterns. F10 drives them:

```
transitions:
  # opening a substitution: enter expression context, remember to return.
  on (TEMPLATE_HEAD | TEMPLATE_MIDDLE) -> push template, set-mode default
  # closing a substitution.
  on (TEMPLATE_TAIL | TEMPLATE_NO_SUB) -> pop, set-mode div
```

`TEMPLATE_HEAD` pushes `template` so the closing `}` is recognised by the
`TEMPLATE_MIDDLE`/`TEMPLATE_TAIL` patterns, and the substitution body is lexed in
`default` (expression) position. The first slice handles substitutions whose bodies
contain no nested unbalanced `{ }` (object/block literals inside `${...}`); nested-brace
tracking is the follow-up below.

## Residual Hard Cases

The flat-mode model closes gap-115/119/044. It does **not** resolve these, which need a
brace/paren-*kind* stack — exactly what Acorn uses its full context stack for. They are
documented here, not silently mis-handled, and a later slice can add depth/kind-guarded
rules to the *same* transition table (the lexer already tracks `bracket_depths`):

1. **`}` block-close vs object-literal-close vs template-substitution-close.** The flat
   mode cannot distinguish these without brace-depth. Template substitutions containing
   `{ }` object/block literals are out of scope for the first slice; plain `${expr}`
   works.
2. **`)` after `if (...)`/`while (...)`/`for (...)` vs after an operand.** After the
   former a `/` is a regex (`if (x) /re/.test(y)`); after the latter it is division
   (`(a)/b`). The flat model treats all `RPAREN` as value-producing (→ `div`), which is
   the common case and matches the gap-115 fixtures; the statement-head case is a
   documented residual.
3. **`{` statement-block vs object-literal at expression start.** Same family; handled
   adequately by start-of-statement → `default` and not load-bearing for the target
   gaps.

The flat model is strictly more correct than today, which mis-lexes `a/b/c`
unconditionally.

## Validation Changes

New checks in `validate_token_grammar` (extending F04's group checks):

1. `start_mode`, if set, must be `default` or a declared group → else error.
2. Each `in_mode`, and each `set-mode`/`push` target, must be `default` or a declared
   group → else error (no silent fallback).
3. `on_tokens` referencing an unknown effective token name → warning (built-ins like
   `NAME`/`NUMBER` and `KEYWORD`-by-value are allowed).
4. Duplicate or unreachable rules → warning.
5. **Security caps.** Bound the transition count and `on_tokens` length per rule (e.g.
   ≤ 4096 rules) so an untrusted `.tokens` file cannot blow up generated-code size; a
   malformed rule (missing `->`, unknown action) is a hard error.

## Backward Compatibility

A grammar with no `start_mode:` and no `transitions:`:

- parses to `start_mode = None`, `transitions = vec![]`;
- the lexer initialises `group_stack = ["default"]` (unchanged) and `apply_transitions`
  early-returns on the empty table — the emit path is byte-identical to F04;
- the compiler emits `start_mode: None, transitions: vec![]`; recompiling every existing
  grammar yields no behavioral diff (only two trivially-default fields appear).

Every existing `TokenGrammar { .. }` literal across all language ports must add the two
empty defaults; until each grammar adopts a table, behavior is exactly as today.

## Implementation Order

| Step | What | Commit |
|------|------|--------|
| 0 | This spec (F10) | `spec(lexer): F10 declarative mode transitions` |
| 1 | grammar-tools (Rust): `ModeTransition`/`TransitionAction`, `start_mode`/`transitions` fields, parsing, validation | `feat(grammar-tools): declarative lexer mode transitions` |
| 2 | lexer (Rust): `apply_transitions`, `start_mode` init, callback coexistence | `feat(lexer): interpret declarative mode transitions` |
| 3 | grammar-tools compiler (Rust): `transitions_src` codegen | `feat(grammar-tools): emit transition tables in compiled grammars` |
| 4 | Port grammar-tools + lexer to Python, Ruby, TS, Go, Elixir | `feat(grammar-tools): mode transitions (all languages)` |
| 5 | es2022/es2025 `.tokens` transitions + regenerate + close gap-115/119/044 | `feat(closurec): close gap-115/119/044 via declarative lexer modes` |

## Testing Strategy

### Grammar-tools tests
- Parse `start_mode:` and `transitions:` (alternation sets, `in MODE` guard,
  `KEYWORD="..."` value form, multi-action lines).
- Reject missing `->`, unknown action, undefined target mode.
- Existing `.tokens` files parse to empty defaults (backward compat).

### Lexer tests
- Regex-vs-division: `a/b/c`, `4/2/1`, `a/b+c/d`, single `a/b`; regex after operator,
  after `(`/`,`/`{`/`;`/`:`, and after `return`.
- Template: `` `a${b}c` ``, tagged `` tag`x${y}` ``; nested
  `` `a${`x${y}z`}b` `` documented as residual.
- `set-mode` keeps stack depth constant; `push`/`pop` change it; `pop` floors at
  `default`; skip enable/disable toggles.
- No-table grammars produce identical token streams to today (snapshot JSON/TOML/CSS).

### Compiler tests
- Emitted transition table compiles and round-trips; byte-stable across runs; empty
  table → `vec![]`.

### Integration (closurec)
- The re-enabled byte-identity fixtures pass; zero regressions across the existing
  harness; `cargo build --workspace` catches any missed `TokenGrammar` literal.

### Backward compatibility
- All existing lexer test suites pass unchanged across all six languages.
