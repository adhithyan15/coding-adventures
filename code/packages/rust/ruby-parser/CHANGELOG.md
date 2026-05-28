# Changelog

All notable changes to the `coding-adventures-ruby-parser` crate will be documented in this file.

## [0.34.0] - 2026-05-26

### Added (Phase 8a (FC) — additional arithmetic / bitwise / shift op-assigns)

Extended the `assignment` rule to recognise six more compound-assignment operators:

```ebnf
assignment = NAME ( EQUALS | "+=" | "-=" | "*=" | "/=" | "%=" | "**=" | "<<=" | "&=" | "|=" | "^=" | "||=" | "&&=" ) expression ;
```

Combined with the lexer's `fuse_compound_assigns` companion pass (extended in this phase to recognise `%`, `**`, `<<`, `&`, `|`, `^` as left operands), Ruby's complete compound-assign family on local variables is now parseable — minus `>>=`, which is tracked as a follow-up.

Regenerated `_grammar.rs` via `grammar-tools compile-grammar`.

### Tests

- `coding-adventures-ruby-parser`: 143 → **147** (+4):
  - `test_parse_modulo_op_assign`
  - `test_parse_power_and_shift_op_assigns`
  - `test_parse_bitwise_op_assigns`
  - `test_parse_plain_assignment_still_works_after_8a` (regression)

### v0 deferred limitations

- `>>=` is NOT yet accepted because the 1.8-era lexer state machine splits `>>` into two `>` tokens.  A dedicated `>>` pre-fusion pass is the natural follow-up; it slots in cleanly because the post-`>>=` lowering reuses the same Phase 8a desugar path.

## [0.33.0] - 2026-05-26

### Added (Phase 7f — Ruby 3.1 hash value-omitted shorthand `{x:, y:}`)

Extended grammar rule:

```ebnf
hash_entry = NAME COLON expression | NAME COLON | expression "=>" expression ;
```

The new `NAME COLON` alternative (no trailing expression) is placed AFTER `NAME COLON expression` so PEG ordered-choice tries the longer form first.  When the longer form fails (e.g. `x:` followed by `,` or `}`), the parser cleanly backtracks to the value-omitted shape.

The grammar change is purely additive — every prior shape (`{x: 1}`, `{a => b}`, `{x: 1, y: 2}`) parses identically.

Regenerated `_grammar.rs` via `grammar-tools compile-grammar`.

### Tests

- `coding-adventures-ruby-parser`: 140 → **143** (+3):
  - `test_parse_hash_value_shorthand_pure` — `{x:, y:}` produces two `hash_entry` subnodes each with ZERO `expression` children.
  - `test_parse_hash_value_shorthand_mixed` — `{x:, y: 5}` produces one entry with 0 expression children and one with 1.
  - `test_parse_hash_value_shorthand_regression_existing_form` — `{x: 1, y: 2}` still produces entries with 1 expression child each.

## [0.32.0] - 2026-05-25

### Added (Phase 7e — Ruby 3.0 rightward assignment `expr => var`)

New grammar rule:

```ebnf
rightward_assignment = expression "=>" NAME ;
```

Placed AFTER `modifier_statement` (so `x = 1 if cond` still wins) and BEFORE `assignment` in the `statement` alternation.

The `=>` token is shared with hash literals (`{ "a" => 1 }`), but the rightward-assignment rule only matches at the statement level — PEG cleanly backtracks when `=>` sits inside `{...}`.

Regenerated `_grammar.rs` via `grammar-tools compile-grammar`.

### Lowering (in `ruby-to-semantic-ir`)

A new helper `lower_rightward_assignment` lowers identically to a regular `assignment`:

| Source              | SIR shape                                                |
|---------------------|----------------------------------------------------------|
| `1 + 2 => sum`      | `LetBinding(sum, BuiltinCall("+", [IntLit 1, IntLit 2]))` |
| `42 => x`           | `LetBinding(x, IntLit 42)`                               |
| (re-bind) `5 => x`  | `Assign(x, IntLit 5)` + `Feature::MutableBindings`       |

Rightward assignment is purely syntactic — `expr => var` and `var = expr` produce identical SIR — so downstream emitters need no new code paths.

### v0 deferred limitations

- Compound rightward forms (`expr += var`, etc.) are NOT supported in Ruby itself.
- Modifier-suffix combinations like `expr => var if cond` are deferred (modifier_statement only accepts `assignment | method_call_no_paren | method_call | expression_stmt` as its LHS).

### Tests

- `coding-adventures-ruby-parser`: 136 → **140** (+4 grammar tests):
  - `test_parse_rightward_assignment_with_literal` — `1 => x`.
  - `test_parse_rightward_assignment_with_binary_expression` — `1 + 2 => sum`.
  - `test_parse_rightward_assignment_with_call` — `foo(1, 2) => result`.
  - `test_parse_rightward_assignment_does_not_break_normal_assignment` — regression.

## [0.31.0] - 2026-05-25

### Added (Phase 7d — Ruby 3.0 `case/in` pattern matching)

The existing `case_statement` rule is extended to accept either `when_clause` or `in_clause` repetitions in any source order:

```ebnf
case_statement = "case" expression { when_clause | in_clause } [ else_clause ] "end" ;
in_clause      = "in" pattern { !"when" !"in" !"else" !"end" statement } ;
pattern        = array_pattern | hash_pattern | literal_pattern | binding_pattern ;
literal_pattern   = NUMBER | STRING | symbol_literal | KEYWORD ;
binding_pattern   = NAME ;
array_pattern     = LBRACKET [ pattern { COMMA pattern } ] RBRACKET ;
hash_pattern      = LBRACE [ hash_pattern_pair { COMMA hash_pattern_pair } ] RBRACE ;
hash_pattern_pair = NAME COLON [ pattern ] ;
```

Order in `pattern`: array/hash come first so their `[`/`{` openers shadow the literal/binding forms; literal comes before binding so concrete constants don't get swallowed as bindings.

Regenerated `_grammar.rs` via `grammar-tools compile-grammar`.

The `when_clause` body lookaheads were extended with `!"in"` so `when`/`in` can interleave cleanly in the rule's repetition (even though semantically Ruby disallows mixing — this is a grammar concern, not a semantic one).

### Lowering (in `ruby-to-semantic-ir`)

`lower_case_statement` now collects both `when_clause` and `in_clause` subnodes in source order.  A new helper `lower_in_clause_pattern` dispatches on pattern kind:

| Pattern        | cond                                            | body-prefix stmts        |
|----------------|-------------------------------------------------|--------------------------|
| `in 1`         | `BuiltinCall("==", [scrut, IntLit(1)])`         | `[]`                     |
| `in nil`       | `BuiltinCall("==", [scrut, NilLit])`            | `[]`                     |
| `in y`         | `BoolLit(true)`                                 | `[LetBinding(y, scrut)]` |
| `in [1, 2]`    | `BuiltinCall("__pattern_match__", [scrut, raw])` | `[]`                    |
| `in {name: y}` | `BuiltinCall("__pattern_match__", [scrut, raw])` | `[]`                    |

The `__pattern_match__` marker carries the raw source text of the pattern (joined Token values via a depth-first walk) so downstream emitters can re-derive the structural matching at codegen time.  Same marker-builtin pattern as Phase 6v rescue/ensure, Phase 6y `__interp__`, Phase 7a `backtick`.

### v0 deferred limitations

- Array / hash patterns lower as a `__pattern_match__` marker — no structural decomposition or sub-bindings yet.  Downstream emitters can re-parse the marker's StrLit raw text.
- Hash pattern shorthand `{name:}` doesn't bind `name` at SIR level (deferred along with structural decomposition).
- Pin operators (`^x`), find patterns (`[…, *, …]`), and class patterns (`SomeClass(x)`) are not yet parsed.
- A bare-name body inside `in` (e.g. `in y; y; end`) hits a pre-existing grammar quirk where `method_call_no_paren` greedily consumes the closing `end` as a one-arg method-call argument — unrelated to Phase 7d, observable since Phase 6h.  Workaround: wrap the body in a paren-call (`puts(y)`) or any non-bare form.

### Tests

- `coding-adventures-ruby-parser`: 131 → **136** (+5 grammar tests):
  - `test_parse_case_in_with_literal_pattern` — `in 1`.
  - `test_parse_case_in_with_binding_pattern` — `in y`.
  - `test_parse_case_in_with_array_pattern` — `in [1, 2]`.
  - `test_parse_case_in_with_hash_pattern` — `in {name: y}`.
  - `test_parse_case_when_still_works_after_in_clause_addition` — regression.

## [0.30.0] - 2026-05-25

### Added (Phase 7c — Ruby 3.0 endless method definitions `def foo = expr`)

New grammar rule:

```
endless_def_statement = "def" NAME [ LPAREN [ params ] RPAREN ] EQUALS expression ;
```

Placed **before** `def_statement` in the `statement` alternation so PEG tries the endless form first; when the `=` isn't present the parser falls through to the block-bodied `def`.  The two forms cannot conflict because the endless form requires `=` immediately after the signature, while the block form expects a newline + body statements + `end`.

Regenerated `_grammar.rs` via `grammar-tools compile-grammar`.

### Lowering (in `ruby-to-semantic-ir`)

A new helper `lower_endless_def_statement` mirrors `lower_def_statement`'s parameter extraction and scope-isolation logic, but the body is the single trailing `expression` Node:

```
endless_def_statement → Function {
    name,
    params,
    body: Block { stmts: [], value: <lowered expression> },
    ...
}
```

Both the program-level pre-pass (`collect_def_statements`) and the class/module nested pass (`collect_def_statements_from_body`) now dispatch on rule name to handle both `def_statement` and `endless_def_statement`.

### v0 deferred limitations

- Inherits Phase 6s's lossy-splat limitation: `def foo(*args) = args.sum` parses but the `*` prefix on the param is dropped at SIR level (`Param` has no variadic flag).
- The endless form is not nested under classes / modules in any SIR-significant way (matches the block-bodied def's v0 limitation).

### Tests

- `coding-adventures-ruby-parser`: 128 → **131** (+3 grammar tests):
  - `test_parse_endless_def_no_params` — `def hello = 1`.
  - `test_parse_endless_def_with_params` — `def add(x, y) = x + y`.
  - `test_parse_endless_def_does_not_break_block_def` — regression for the block-bodied form.

## [0.29.0] - 2026-05-25

### Added (Phase 7b — heredocs in parser)

The lexer's Phase-3c body capture + Phase-4o opener-variant handling (`<<EOF`, `<<-EOF`, `<<~EOF`) finalise every heredoc into a single `TokenType::String` token whose value is the verbatim canonical form `<<TAG\n<body>TAG` (with `<<~TAG`'s common-indent stripping pre-applied).  Phase 7b is therefore **grammar-zero**: the existing STRING token at factor / assignment-RHS positions already accepts heredocs.  This PR adds the SIR lowering dispatch + explicit test coverage.

### Lowering (in `ruby-to-semantic-ir`)

A new helper `lower_heredoc_literal` strips the opener prefix and the closing tag, emitting:

```
StrLit(body)
```

The `String` case of `lower_factor_atom` now dispatches in lexeme-prefix priority order:
- starts with `` ` `` → backtick command literal (Phase 7a)
- starts with `<<` → heredoc (Phase 7b)
- otherwise → string interpolation lowering (Phase 6y)

The synthetic `StrLit` triggers `Feature::Strings`.

### v0 deferred limitations

- Interpolation inside the body (`#{name}`) is NOT split — the body lowers as a single `StrLit` with `#{...}` markers preserved verbatim.  Follow-up will reuse the Phase 6y splitter.
- Non-interpolating heredocs (`<<'TAG'`) and the `<<"TAG"` form are not yet distinguished — the lexer doesn't carry the quote state, so every heredoc is treated the same.
- Escape sequences inside the body are kept literal (the lexer's heredoc capture does not unescape; same v0 stance as backticks).

### Tests

- `coding-adventures-ruby-parser`: 125 → **128** (+3 grammar tests):
  - `test_parse_plain_heredoc_assignment` — `<<EOF`.
  - `test_parse_dash_indent_heredoc_assignment` — `<<-EOF` with indented closer.
  - `test_parse_tilde_indent_heredoc_assignment` — `<<~EOF` with indent stripping.

## [0.28.0] - 2026-05-25

### Added (Phase 7a — backtick command literals in parser)

The lexer's Phase-4m `backtick_body` state emits `` `cmd args` `` as a single `TokenType::String` token whose value is the verbatim source **including the surrounding backticks** (`` `cmd args` ``) — the same lexeme-prefix sentinel trick used by percent literals and heredocs.  Phase 7a is therefore **grammar-zero**: the existing STRING token at factor / call-arg / assignment-RHS positions already accepts backtick literals.  This phase adds the SIR lowering dispatch + explicit test coverage on both sides of the pipeline.

### Lowering (in `ruby-to-semantic-ir`)

A new helper `lower_backtick_command_literal` strips the wrapping backticks and emits:

```
BuiltinCall {
    name: "backtick",
    args: [StrLit(body)],
    effects: MayBlock | MayPrint | MayThrow,
}
```

The triple-effect set reflects that command execution may **block** on the child process, **print** stdout/stderr, and **throw** if the command can't be invoked (`Errno::ENOENT` and friends).

The `String` case of `lower_factor_atom` dispatches by lexeme prefix:
- starts with `` ` `` → backtick command literal (Phase 7a)
- otherwise → string interpolation lowering (Phase 6y)

### v0 deferred limitations

- Interpolation inside the body (`` `echo #{name}` ``) is NOT split — the body lowers as a single `StrLit` with `#{...}` markers preserved verbatim.  A future phase will reuse the Phase 6y interpolation splitter inside the body.
- Escape sequences inside the body (`` \` ``, `\n`, `\t`) are already resolved by the lexer (Phase 4m's body state).

### Tests

- `coding-adventures-ruby-parser`: 122 → **125** (+3 grammar tests):
  - `test_parse_backtick_command_literal_assignment` — `` x = `ls -la` ``.
  - `test_parse_backtick_command_literal_in_call_arg` — `` puts(`pwd`) ``.
  - `test_parse_empty_backtick_command_literal` — `` x = `` ``.

## [0.27.0] - 2026-05-25

### Added (Phase 6z — float / hex / bin / oct numeric literal parsing)

The lexer's Phase-4k float fusion (`1.5`, `1e10`, `1.5e-3`) and Phase-4l radix-prefix fusion (`0x1F`, `0b1010`, `0o17`, `0d42`) emit each literal as a single `TokenType::Number` token whose value is the verbatim source text.  Phase 6z is therefore **grammar-zero**: the existing NUMBER token at factor position already accepts every shape.  This phase adds the SIR lowering dispatch + explicit test coverage on both sides of the pipeline.

### Lowering (in `ruby-to-semantic-ir`)

A new helper `lower_numeric_literal` dispatches on shape:

| Source       | SIR shape                                |
|--------------|------------------------------------------|
| `42`         | `IntLit { value: 42 }`                   |
| `1_000_000`  | `IntLit { value: 1000000 }`              |
| `0x1F`       | `IntLit { value: 31 }` (radix 16)        |
| `0xDEAD_BEEF`| `IntLit { value: 3735928559 }`           |
| `0b1010`     | `IntLit { value: 10 }` (radix 2)         |
| `0o17`       | `IntLit { value: 15 }` (radix 8)         |
| `0d42`       | `IntLit { value: 42 }` (radix 10 explicit) |
| `1.5`        | `FloatLit { value: 1.5 }` + Feature::Floats |
| `1e10`       | `FloatLit { value: 1e10 }` + Feature::Floats |
| `1.5e-3`     | `FloatLit { value: 0.0015 }` + Feature::Floats |

Underscore separators (`_`) are stripped before parsing.  Radix detection checks `bytes[1] ∈ {x,X,b,B,o,O,d,D}` after a leading `0`.  Float detection is a single scan for `.` or `e`/`E` in the cleaned digit string; the two checks are mutually exclusive in the Ruby grammar.

### v0 deferred limitations

- Ruby's Rational (`r`) / Complex (`i`) numeric suffixes (lexed by Phase 4f) are rejected — a future phase will route those into `BuiltinCall("rational", ...)` / `BuiltinCall("complex", ...)` markers.
- Negative literals are still handled by the unary-minus path (Phase 6k); this routine sees only the magnitude.
- Ruby's legacy octal syntax (`017` without `0o` prefix) is not supported; use `0o17`.

### Tests

- `coding-adventures-ruby-parser`: 117 → **122** (+5 grammar tests):
  - `test_parse_float_literal_assignment` — `x = 1.5`.
  - `test_parse_float_literal_with_exponent` — `x = 1.5e-3`.
  - `test_parse_hex_integer_literal` — `x = 0xDEAD_BEEF`.
  - `test_parse_binary_integer_literal` — `x = 0b1010`.
  - `test_parse_octal_integer_literal` — `x = 0o17`.

## [0.26.0] - 2026-05-25

### Added (Phase 6y — string interpolation expression parsing)

The lexer's Phase-3b state machine captures `"foo#{x}bar"` as a single `TokenType::String` token whose value carries the `#{...}` markers verbatim, with `{`/`}` already brace-balanced by the lexer's `interp_brace_depth` tracking.  Phase 6y is therefore **grammar-zero**: the existing STRING token at factor / call-arg / assignment-RHS positions already accepts interpolated forms.  This phase adds explicit test coverage for the parser side and the SIR lowering for the interpolation split.

### Lowering (in `ruby-to-semantic-ir`)

The SIR lowerer's String case now scans the raw content for `#{...}` segments and emits:

| Source              | SIR shape                                                                                       |
|---------------------|-------------------------------------------------------------------------------------------------|
| `"plain"`           | `StrLit("plain")` (zero-cost fast path — unchanged)                                             |
| `"#{x}"`            | `VarRef("x")` (single non-literal segment, no wrapper)                                          |
| `"hi #{name}"`      | `BuiltinCall("string_concat", [StrLit("hi "), VarRef("name")])`                                 |
| `"sum=#{1+2}"`      | `BuiltinCall("string_concat", [StrLit("sum="), BuiltinCall("__interp__", [StrLit("1+2")])])`    |

Bare-identifier interp bodies lower to a `VarRef` with the same `Scope::Param` / `Scope::Local` dispatch as the regular factor-atom Name case.  More complex interp bodies (arithmetic, calls, nested strings) emit a marker `BuiltinCall("__interp__", [StrLit(raw_body)])`, matching the marker pattern used by Phase 6v rescue/ensure — downstream Ruby emitters can re-emit the marker verbatim as `#{<raw>}`.

### v0 deferred limitations

- Complex interp bodies are carried as the raw `__interp__` marker rather than being recursively parsed — a future phase will invoke the parser/lowerer on the body so the SIR carries semantic info.
- Escape sequences inside the string literal pass through unchanged (the lexer hasn't unescaped them yet).

### Tests

- `coding-adventures-ruby-parser`: 113 → **117** (+4):
  - `test_parse_interpolated_string_assignment` — `x = "hello #{name}"`.
  - `test_parse_interpolated_string_in_call_arg` — `puts("sum=#{1+2}")`.
  - `test_parse_interpolated_string_only_interp` — `x = "#{name}"`.
  - `test_parse_interpolated_string_multiple_segments` — `x = "a=#{a}, b=#{b}"`.

## [0.25.0] - 2026-05-25

### Added (Phase 6x — instance var `@x`, class var `@@x`, global var `$x` refs)

The lexer (Phase 4i/4j) emits these as a SINGLE Name-typed token whose value carries the leading sigil (`@a`, `@@all`, `$c`).  The parser sees them as bare `NAME` tokens at the factor and assignment-LHS levels — **no new grammar rules required**.  This phase adds explicit test coverage and the SIR lowerer's documentation of how sigil-preserved names flow through.

### Lowering (in `ruby-to-semantic-ir`)

- All sigil-prefixed names route to `Scope::Local` for v0.
- The sigil stays in the bound name (`@a`, `@@all`, `$config`), so downstream emitters can detect the form and route assignment/read appropriately.
- The validator-correct `$x` → `Scope::Global` mapping would require auto-emitting matching `Global` declarations on the module; that's deferred to a follow-up phase.

### v0 deferred limitations

- SIR has no `IVar` / `CVar` / `GVar` scope.  Using `Scope::Global` for `$x` would require module-level `Global` declarations that the validator enforces.  Until those are auto-generated, all sigil vars sit on `Scope::Local` with the sigil preserved in `name`.
- Downstream emitters targeting Ruby (or any language with similar sigil semantics) can still distinguish by checking for the leading `@` / `@@` / `$`.

### Tests

- `coding-adventures-ruby-parser`: 109 → **113** (+4):
  - `test_parse_instance_var_assignment` — `@a = 1`.
  - `test_parse_class_var_assignment` — `@@all = 0`.
  - `test_parse_global_var_assignment` — `$config = 1`.
  - `test_parse_instance_var_in_expression` — `puts(@a)`.

## [0.24.0] - 2026-05-25

### Added (Phase 6w — arrow-lambda literal `->(params){body}`)

New grammar rule:

```
factor         = ( lambda_literal | method_call | NUMBER | ... ) { dot_call } ;
lambda_literal = "->" [ LPAREN [ params ] RPAREN ] block ;
```

Placed BEFORE `method_call` in factor so the `->` literal wins.  Without this, `->` (a Name-typed Op token from the lexer) would be mis-matched by `method_call`'s `(NAME|KEYWORD) LPAREN` prefix when followed by parens.

`lambda { … }` and `proc { … }` continue to lower via `method_with_block` (no new grammar — they're regular keyword-led calls).  The SIR lowerer now recognises `lambda` and `proc` as builtins so both shapes produce the same `BuiltinCall("lambda", …)` form.

#### Parser default era bumped to "3.0"

`create_ruby_parser` now invokes the lexer with `tokenize_ruby_for_version(source, "3.0")` so era-gated lexer fusions are visible to the parser by default.  Most importantly, `->` is now fused into a single Op token (1.9.1+ behaviour) so `lambda_literal` can match it.  Era-specific lexer tests still use the lower-level `tokenize_ruby_for_version` directly.

New public constant: `DEFAULT_RUBY_ERA: &str = "3.0"`.

### Lowering (in `ruby-to-semantic-ir`)

Arrow lambda → `BuiltinCall("lambda", [MakeClosure { fn_name: "__block_<n>", captures: [] }])`.

Body is hoisted to a top-level `Function` (named `__block_<n>`, reusing Phase 6g's counter).  Params are extracted from the parens-list (Phase 6s — splat supported) rather than from the block's `block_params` pipe header.

### v0 deferred limitations

- Block bodies that reference outer locals lose them — captures are NOT computed (same as Phase 6g).
- If the user writes both `->(x) { |y| … }` (parens-params AND block_params), the latter is silently ignored.
- `lambda { ... }` / `proc { ... }` works only at statement position, not as an expression RHS (because `method_with_block` is not in `factor`).  Arrow form (`->(...) { ... }`) is the recommended form for expression-position closures.

### Tests

- `coding-adventures-ruby-parser`: 105 → **109** (+4):
  - `test_parse_arrow_lambda_no_params` — `-> { 1 }`.
  - `test_parse_arrow_lambda_with_params` — `->(x, y) { x + y }`.
  - `test_parse_arrow_lambda_inside_call` — `each(->(x) { x })`.
  - `test_parse_lambda_keyword_with_brace_block` — `lambda { |x| x + 1 }` (regression: NOT a lambda_literal).

## [0.23.0] - 2026-05-25

### Added (Phase 6v — `begin … rescue … ensure … end`)

New grammar rules:

```
statement       = ... | case_statement | begin_statement | return_statement | ... ;
begin_statement = "begin"
                  { !"rescue" !"ensure" !"end" statement }
                  { rescue_clause }
                  [ ensure_clause ]
                  "end" ;
rescue_clause   = "rescue" [ exception_list "=>" NAME ]
                       { !"rescue" !"ensure" !"end" statement } ;
exception_list  = NAME { COMMA NAME } ;
ensure_clause   = "ensure" { !"end" statement } ;
```

#### Grammar design decision

`rescue_clause` accepts the optional exception header ONLY when paired with `=>` binding (`rescue StandardError => e`).  Bare `rescue ExceptionType` (no `=>`) would create grammar ambiguity with the rescue body's first NAME-led statement — `rescue x = 2` would otherwise parse `x` as an exception type and fail at `=`.  Workaround for users wanting bare-type rescue: write `rescue ExceptionType => _`.

#### Lowering (in `ruby-to-semantic-ir`)

SIR has no try/catch primitive.  v0 lowering is lossy: body, rescue, and ensure stmts emit inline with synthetic marker `BuiltinCall`s:

```
begin body
rescue StandardError, IOError => e rescue_body
ensure ensure_body
end
```

→

```
body_stmts...
ExprStmt(BuiltinCall("__rescue_marker__", [StrLit("StandardError,IOError"), StrLit("e")]))
rescue_stmts...
ExprStmt(BuiltinCall("__ensure_marker__", []))
ensure_stmts...
```

Markers carry the `Effect::MayThrow` tag.  Downstream emitters targeting languages with real exceptions can re-stitch the form via marker detection; for v0 the rescue body is unreachable in SIR's effect model.

`lower_statement_inner_multi` (already routing `multi_assignment` to a Vec<Stmt> path from Phase 6r) is extended to dispatch `begin_statement` to a new `lower_begin_statement` helper.

#### v0 deferred limitations

- Bare `rescue ExceptionType` (no `=>`) — see grammar design note.
- `else` clause inside begin (executes when no exception raised) — not supported.
- Real try/catch propagation through SIR's effect lattice — markers only.
- `retry` / `raise` inside rescue body lower like any other call.

### Tests

- `coding-adventures-ruby-parser`: 101 → **105** (+4):
  - `test_parse_begin_with_rescue` — bare `rescue` body.
  - `test_parse_begin_with_rescue_typed_and_var` — `rescue StandardError => e`.
  - `test_parse_begin_with_ensure` — `begin … ensure … end`.
  - `test_parse_begin_with_rescue_and_ensure` — full three-section form.

## [0.22.0] - 2026-05-25

### Added (Phase 6u — `case … when … else … end`)

New grammar rules:

```
statement      = ... | until_statement | case_statement | return_statement | ... ;
case_statement = "case" expression { when_clause } [ else_clause ] "end" ;
when_clause    = "when" expression { COMMA expression }
                      { !"when" !"else" !"end" statement } ;
```

`else_clause` is reused from the existing `if_statement` definition — the else body parses identically to `if … else … end`.

Multi-value `when` lists are supported (`when 1, 2, 3` parses with three expression children inside the clause).  Each when_clause's body uses the same negative-lookahead repetition (`{ !"when" !"else" !"end" statement }`) as if/else, so it stops cleanly at the next clause boundary.

### Lowering (in `ruby-to-semantic-ir`)

Chained `Expr::If` with `==` comparisons:

```
case x
when v1, v2 then a
when v3     then b
else c
end
```

becomes

```
if ((x == v1) || (x == v2)) then a
else if (x == v3) then b
else c
```

- Each when_clause becomes one nested `If` step.
- Multi-value `when 1, 2, 3` lists OR-fold left-to-right using `BuiltinCall("or", ...)`.
- The else_clause (or implicit `NilLit` block) caps the chain.

### v0 deferred limitations

- `when` uses `==` instead of Ruby's `===` (case-equality, class-aware).  Phase 7d will add full `case/in` pattern matching with proper match semantics.
- Range/Regex/Class values in `when` lists parse syntactically but don't match Ruby's case-equality semantics under v0's `==` lowering.
- Splat in `when` argument lists (`when *arr`) is not supported.

### Tests

- `coding-adventures-ruby-parser`: 98 → **101** (+3):
  - `test_parse_case_single_when` — one when, no else.
  - `test_parse_case_multiple_whens_and_else` — two whens + else.
  - `test_parse_when_with_multiple_values` — `when 1, 2, 3`.

## [0.21.0] - 2026-05-25

### Added (Phase 6t — `yield` keyword with optional args)

New grammar rule:

```
statement       = ... | next_statement | yield_statement | multi_assignment | ... ;
yield_statement = "yield" [ yield_args ] ;
yield_args      = LPAREN [ call_arg { COMMA call_arg } ] RPAREN
                | call_arg { COMMA call_arg } ;
```

All three surface forms supported:

| Source | AST |
|---|---|
| `yield` | yield_statement with no `yield_args` wrapper |
| `yield(x, y)` | yield_statement → yield_args → LPAREN call_arg COMMA call_arg RPAREN |
| `yield x, y` | yield_statement → yield_args → call_arg COMMA call_arg |
| `yield(*arr)` | yield_statement → yield_args → LPAREN call_arg(*arr) RPAREN |

Placed AFTER `next_statement` and BEFORE the catch-all `multi_assignment` / `modifier_statement` / `assignment` / `method_with_block` / `method_call` family so `yield` (which the lexer reclassifies as a KEYWORD token) doesn't fall through to `method_call_no_paren` (which would lose the yield-specific lowering).

Args reuse Phase 6s's `call_arg`, so splat (`yield *arr`) and double-splat (`yield **hsh`) work uniformly with method-call args.

### Lowering (in `ruby-to-semantic-ir`)

`yield ...` → `Stmt::ExprStmt(Expr::BuiltinCall("yield", lowered_args, EffectSet::PURE))`.

Effects are PURE — the block's effects bubble up through its construction site (via the `MakeClosure` effect set) rather than via the `yield` call.

### Tests

- `coding-adventures-ruby-parser`: 94 → **98** (+4):
  - `test_parse_bare_yield` — `yield` alone.
  - `test_parse_yield_with_paren_args` — `yield(x, y)`.
  - `test_parse_yield_with_parenless_args` — `yield x, y`.
  - `test_parse_yield_with_splat_arg` — `yield(*arr)`.

## [0.20.0] - 2026-05-25

### Added (Phase 6s — splat / double-splat in params and call args)

Grammar additions:

```
params   = param { COMMA param } ;
param    = [ "*" | "**" ] NAME ;
method_call = ( NAME | KEYWORD ) LPAREN [ call_arg { COMMA call_arg } ] RPAREN { dot_call } ;
dot_call    = "." ( NAME | KEYWORD ) [ LPAREN [ call_arg { COMMA call_arg } ] RPAREN ] ;
call_arg = [ "*" | "**" ] expression ;
```

`params` previously bound bare `NAME` tokens; each parameter is now wrapped in a `param` rule that admits the optional splat / double-splat prefix.  Similarly, `method_call` and `dot_call` argument slots are wrapped in a `call_arg` rule.

`method_call_no_paren` intentionally keeps bare `expression` args — wrapping it in `call_arg` would create a grammar ambiguity with binary `*` at expression-start position (`a * b` would parse as `a(splat b)`).  Paren-less splat (`puts *arr`) is therefore a v0 deferred limitation; users can always fall back to the parenned form `puts(*arr)`.

The lexer already emits `**` as one Name-typed Op token (1.8-baseline state machine coalesces it); `*` is a Star token.  Both match by literal value in the grammar.

### Lowering (in `ruby-to-semantic-ir`)

- **Call args**: `f(*arr)` → `BuiltinCall("splat", [VarRef(arr)])`; `f(**hsh)` → `BuiltinCall("double_splat", [VarRef(hsh)])`.  Bare args are unchanged.
- **Params**: v0 lossy — `*args` and `**kwargs` lower to regular `Param { name: "args" / "kwargs" }`.  SIR's `Param` has no variadic flag, so splat-ness is dropped at the SIR level.  Downstream emitters treat the parameter as positional; correctness for variadic call sites is a deferred limitation.

### Tests

- `coding-adventures-ruby-parser`: 88 → **94** (+6):
  - `test_parse_splat_param` — `def f(*args) end`.
  - `test_parse_double_splat_param` — `def f(**kwargs) end`.
  - `test_parse_mixed_params_with_splats` — `def f(a, *rest, **opts) end`.
  - `test_parse_splat_call_arg` — `f(*arr)`.
  - `test_parse_mixed_call_args_with_splats` — `f(1, *arr, **hsh)`.
  - `test_parse_binary_star_still_parses_as_expression` — regression: `a * b` stays binary.

Also updates two pre-existing tests to walk the new `param` / `call_arg` wrappers (`test_parse_def_with_params`, `test_parse_dot_call_with_args`).

## [0.19.0] - 2026-05-25

### Added (Phase 6r — multiple assignment `a, b = 1, 2`)

New grammar rule:
```
statement        = ... | next_statement | multi_assignment | modifier_statement |
                   assignment | method_with_block | method_call | method_call_no_paren |
                   expression_stmt ;
multi_assignment = NAME COMMA NAME { COMMA NAME }
                   EQUALS
                   expression { COMMA expression } ;
```

Placed BEFORE `modifier_statement` and `assignment` so `NAME COMMA NAME ... =` parses as a multi-assignment.  Requires at least two LHS names; single-LHS forms (`a = 1`) still flow through the existing `assignment` rule unchanged because `multi_assignment` fails immediately when only one NAME is present.

#### Lowering (in `ruby-to-semantic-ir`)

Each `(lhs[i], rhs[i])` pair lowers to its own `Stmt::LetBinding` / `Stmt::Assign` — exactly as `lhs[i] = rhs[i]` would.  The lowerer's new `lower_statement_inner_multi` wrapper returns a `Vec<Stmt>` so a single `multi_assignment` source node fans out to N SIR statements at every call site (`lower_program`, `lower_clause_statements`, `lower_def_statement` body, `lower_method_with_block` body).

#### v0 restrictions (deferred)

- LHS count must equal RHS count.  Mismatched arities are rejected with a `RubyLowerError`; the more permissive Ruby semantics (excess LHS gets `nil`, excess RHS dropped) ride with a future phase.
- Single-RHS auto-unpack `a, b = arr` is NOT supported.
- Splat targets `a, *b = 1, 2, 3` ride with Phase 6s.
- Multi-assignment LHS inside a `modifier_statement` (`a, b = 1, 2 if cond`) is NOT supported.

### Tests

- `coding-adventures-ruby-parser`: 84 → **88** (+4):
  - `test_parse_multi_assignment_two_names` — basic `a, b = 1, 2`.
  - `test_parse_multi_assignment_three_names` — three LHS / three RHS.
  - `test_parse_multi_assignment_with_complex_rhs` — `a, b = x + 1, y * 2`.
  - `test_parse_single_assignment_not_consumed_by_multi` — regression: `a = 1` stays an `assignment`.

## [0.18.0] - 2026-05-24

### Added (Phase 6q — modifier conditionals/loops `x if y`, `x unless y`, `x while y`, `x until y`)

Trailing-modifier surface syntax for one-line `if`/`unless`/`while`/`until`.

#### Grammar change

```
statement          = ... | return_statement | break_statement | next_statement |
                     modifier_statement | assignment | method_with_block | method_call |
                     method_call_no_paren | expression_stmt ;
modifier_statement = ( assignment | method_call_no_paren | method_call | expression_stmt )
                     ( "if_modifier" | "unless_modifier" | "while_modifier" | "until_modifier" )
                     expression ;
```

Placement: AFTER the keyword-led statements (so `if y ... end` still wins) and BEFORE the bare statement forms (so we try the modifier wrapper before committing to a plain statement).  PEG-style alternation backtracking unwinds cleanly when the trailing keyword isn't present, falling through to the plain forms.

#### Lexer-side disambiguation

The trailing keyword tokens use special values (`if_modifier` etc., not bare `if`/etc.) because ruby-lexer's `tag_modifier_keywords` post-pass re-tags `if`/`unless`/`while`/`until` Keyword tokens to `*_modifier` when they follow an expression-ending token on the same line.  Leading-keyword forms keep the bare values and continue to match the existing `if_statement` / `unless_statement` / `while_statement` / `until_statement` rules.

This sidesteps the grammar's newline-insensitive default mode (modifier syntax is intrinsically a same-line construct in Ruby).  Without the lexer split, two-line programs like `x = 1\nif y ... end` would mis-parse as `(x = 1) if y` followed by orphaned `end`.

#### Lowering (in `ruby-to-semantic-ir`)

| Source              | Lowered SIR                                              |
|---------------------|----------------------------------------------------------|
| `lhs if cond`       | `Stmt::ExprStmt(Expr::If(cond, [lhs], Nil))`             |
| `lhs unless cond`   | `Stmt::ExprStmt(Expr::If(not(cond), [lhs], Nil))`        |
| `lhs while cond`    | `Stmt::While(cond, [lhs])`                               |
| `lhs until cond`    | `Stmt::While(not(cond), [lhs])`                          |

Same `Expr::If` / `Stmt::While` shapes as the leading-keyword forms — downstream emitters (semantic-ir-to-python, -rust, -typescript, -go) need zero new code paths.

### Tests

- `coding-adventures-ruby-parser`: 78 → **84** (+6):
  - `test_parse_if_modifier_simple` — `puts "hi" if cond` parses with `if_modifier`.
  - `test_parse_unless_modifier_with_assignment_lhs` — `x = 1 unless cond`.
  - `test_parse_while_modifier` — `puts "tick" while cond`.
  - `test_parse_until_modifier_with_assignment_lhs` — `x = 1 until cond`.
  - `test_parse_leading_if_not_tagged_as_modifier` — regression: `if y ... end` stays `if_statement`.
  - `test_parse_two_statements_across_newline` — regression: `x = 1\nif y ... end` stays two statements.

## [0.17.0] - 2026-05-24

### Added (Phase 6p — compound assignment `+=`, `-=`, `*=`, `/=`, `||=`, `&&=`)

Grammar change:

```
assignment = NAME ( EQUALS | "+=" | "-=" | "*=" | "/=" | "||=" | "&&=" ) expression ;
```

The lexer's companion `fuse_compound_assigns` post-pass folds adjacent `Op` + `Equals` token pairs into single Name-typed tokens carrying the fused operator value (`+=`, etc.), so the grammar matches by literal value — same convention as `"=>"`, `"<="`, `"&&"`.

Excludes `==`, `!=`, `<=`, `>=`, `===` — those are comparison operators handled at the `comparison` layer.  The fusion pass deliberately only runs on Op tokens (`Plus`/`Minus`/`Star`/`Slash`) and the two Name-typed logicals (`||`/`&&`) to avoid colliding with comparison ops which the lexer fuses directly.

### Tests (+4 new, total 78)
- `test_parse_plus_equals_assignment` — `x += 1` carries `+=` token.
- `test_parse_all_arithmetic_compound_operators` — `+=`, `-=`, `*=`, `/=` all parse.
- `test_parse_logical_compound_operators` — `||=`, `&&=` parse.
- `test_parse_compound_assign_with_complex_rhs` — `x += 1 + 2` parses with one `+=` and one `+`.

## [0.16.0] - 2026-05-24

### Added (Phase 6o — ternary `cond ? a : b`)

New grammar layer inserted between `expression` and `range`:

```
expression = ternary ;
ternary    = range [ "?" expression ":" expression ] ;
```

**Precedence**: ternary sits above range (looser than `..`/`...`) and below assignment.  So `a..b ? c : d` parses as `(a..b) ? c : d`, and `x = b ? c : d` parses as `x = (b ? c : d)` (since `assignment` is statement-level, outside the expression hierarchy).

**Right-associativity**: `a ? b : c ? d : e` parses as `a ? b : (c ? d : e)`.  The false-branch recurses through `expression` (→ ternary at the top), so the inner ternary nests inside the outer's else.

**Colon ambiguity**: `:foo` opens a `symbol_literal` in `factor`.  But by the time the ternary's `:` is reached, the true-branch's factor has already completed and control has unwound back to the ternary rule.  The grammar parser consumes the `:` as the ternary separator, not as a symbol opener.

**Token typing**: `?` lexes as a `Name`-typed Op token with value `?` (catch-all in `classify_op_token`); `:` lexes as `TokenType::Colon`.  Grammar matches by value (`"?"`, `":"`) — same trick as `"=>"`, `"<="`, etc.

### Tests (+3 new, total 74)
- `test_parse_simple_ternary` — `x = 1 ? 2 : 3` produces a ternary node carrying `?` and `:`.
- `test_parse_ternary_right_associative` — `x = a ? b : c ? d : e` carries two `?` and two `:` tokens.
- `test_parse_ternary_inside_array_literal` — `[1 ? 2 : 3]` works as an array element.

All tests wrap the ternary in an assignment to dodge the bare-NAME-led statement ambiguity (lessons.md).

## [0.15.0] - 2026-05-24

### Added (Phase 6n — range expressions `..` and `...`)

New grammar layer inserted between `expression` and `logical_or`:

```
expression = range ;
range      = logical_or [ ( "..." | ".." ) logical_or ] ;
```

`expression` is now a transparent wrapper over `range`.  When no `..`/`...` token follows the first `logical_or`, the `range` rule passes through the single operand unchanged — so existing parses for non-range expressions continue to produce the same shape (modulo an extra `range` node in the AST).

**Precedence**: range sits OUTER over `logical_or` because Ruby's `..`/`...` bind *looser* than `||`.  So `a || b .. c || d` parses as `(a || b) .. (c || d)`.  (Test that exercises this with explicit parens — see Test notes below.)

**Non-chainable**: the optional (zero-or-one) repetition is intentional; `1..5..10` is a parse error in Ruby and our grammar matches that by not allowing chaining.

**Token typing**: the lexer's `fuse_range_ops` (Phase 4e) pre-fuses two consecutive `.` tokens into a single `Name`-typed token with value `..` (three → `...`).  The grammar matches these by literal *value* (`"..."`, `".."`), the same trick used for `"=>"`, `"<="`, `"&&"`, etc.

**Endless / beginless ranges**: out of scope for this phase.  The lexer already flags `..`/`...` followed by a closer (era ≥ 2.6), but the parser-side support — `(1..)`, `arr[2..]`, `(..5)` — gets its own phase.

### Tests (+6 new, total 71)
- `test_parse_inclusive_range` — `1..5` produces a `range` node carrying a `..` token and two `logical_or` operands.
- `test_parse_exclusive_range` — `1...5` carries `...`.
- `test_parse_range_in_assignment_rhs` — `x = 1..10` nests the range in an assignment.
- `test_parse_range_with_arithmetic_endpoints` — `1 + 2 .. 10 - 3` proves range binds looser than `+`/`-`.
- `test_parse_range_inside_array_literal` — `[1..5]` works as an array element.
- `test_parse_range_with_paren_logical_operands` — `(a || b)..(c || d)` proves operands can be full logical chains.

Note: a precedence test for the unparenthesised form (`a || b .. c || d`) was attempted but hit the v0 `method_call_no_paren` framework ambiguity (lessons.md).  The arithmetic-endpoints and paren-operand tests cover the precedence ladder around range without tripping that issue.

## [0.14.0] - 2026-05-24

### Added (Phase 6m — logical operators `&&`, `||`, `and`, `or`, `not`, `!`)

New grammar layer inserted above comparison:

```
expression   = logical_or ;
logical_or   = logical_and { ( "||" | "or" ) logical_and } ;
logical_and  = logical_not { ( "&&" | "and" ) logical_not } ;
logical_not  = { ( "!" | "not" ) } comparison ;
comparison   = sum { CMP_OP sum } ;    # was the old `expression` body
```

The pre-6m `expression` rule body (the comparison-operator chain) was moved into a new `comparison` rule.  Every previous reference to `expression` (assignment RHS, method args, if/unless cond, etc.) now automatically picks up the broader logical layer because they reference `expression`.

**Precedence**: `||`/`or` < `&&`/`and` < `!`/`not` < comparison.  This matches the *symbol* form precedence.  v0 collapses keyword forms (`and`/`or`/`not`) onto the same precedence level (real Ruby gives them lower precedence than even assignment — uncommon in modern code; if any test program depends on the difference it'll be flagged in a follow-up).

**Left-associativity**: `a || b || c` folds as `(a || b) || c`.

**`logical_not` shape**: the rule uses `{ "!" | "not" }` (zero-or-more leading operators) instead of right-recursive alternation.  Equivalent semantics (`!!x` parses as two leading `!` then a comparison), more parser-friendly.

### Parser-framework limitation discovered

`method_call_no_paren = (NAME|KEYWORD) expression …` in the `statement` alternation can swallow a `def`'s body tail expression when the body is a bare logical/binary chain like `a || b`.  Wrapping the tail in parens (`(a || b)`) is the v0 workaround.  See lessons.md for the full diagnosis and follow-up plan.

### Tests (+5 new, total 64)
- `test_parse_logical_or_symbol_form`, `test_parse_logical_and_symbol_form`, `test_parse_logical_keyword_form`, `test_parse_logical_not_prefix`, `test_parse_logical_chain_and_then_or_precedence`.
- 4 existing comparison tests (`test_parse_simple_comparison_has_sum_subnodes`, `test_parse_equality_in_assignment`, `test_parse_comparison_in_if_condition`, `test_parse_plus_has_lower_precedence_than_comparison`) updated to walk the new `comparison` subnode instead of the old `expression` body.
- `test_parse_logical_or_inside_def_body` pins the parens workaround for the def-body issue.

## [0.13.0] - 2026-05-23

### Added (Phase 6l — method receiver chains `foo.bar.baz`, `foo.bar(args)`, `foo(1).bar`)

Grammar additions in `code/grammars/ruby.grammar`:
- `dot_call = "." ( NAME | KEYWORD ) [ LPAREN [ expression { COMMA expression } ] RPAREN ] ;` — one step of a receiver chain.
- `factor`'s atom alternation is wrapped in `( … ) { dot_call }` so chains apply anywhere an expression goes.
- `method_call` grew a trailing `{ dot_call }` so statement-level `foo(1).bar` works without a parser-position trick.
- `method_call` was promoted to the **first** atom alternative inside `factor`.  Without this, `foo(1).bar` in expression position (e.g. RHS of an assignment) leaves `(1).bar` unconsumed because the bare-NAME branch only matches `foo`.

### Tests (+5 new, total 59)
- `test_parse_single_dot_call` — `foo.bar` produces one `dot_call` subnode.
- `test_parse_chained_dot_calls` — `foo.bar.baz` produces two `dot_call`s.
- `test_parse_method_call_with_dot_chain` — `foo(1).bar` has a `method_call` head and one `dot_call` tail.
- `test_parse_dot_call_with_args` — `foo.bar(1, 2)` has two `expression` direct children under the `dot_call`.
- `test_parse_chain_inside_assignment_rhs` — `x = a.b.c` parses two `dot_call`s in the RHS.

Out of scope for this chunk (will get their own phases):
- Setter calls `foo.bar = 1` (assignment LHS stays as bare NAME).
- Bracket access `arr[0]`.

## [0.12.0] - 2026-05-22

### Added (Phase 6k — unary minus `-5`, `-x`, `-(1+2)`)
- New `factor` alternative `unary_minus = MINUS factor`.  Right-recursive (`--5` parses fine); precedence tighter than binary `+ -`.

### Tests (+5 new, total 54)
- `test_parse_unary_minus_on_number`, `test_parse_unary_minus_on_name`, `test_parse_unary_minus_on_parenthesised_expression`, `test_parse_double_unary_minus_nests`, `test_parse_unary_minus_with_binary_addition`.

## [0.11.0] - 2026-05-22

### Added (Phase 6j — control-flow keywords `return` / `break` / `next`)
- `return_statement = "return" [ expression ] ;`
- `break_statement  = "break"  [ expression ] ;`
- `next_statement   = "next"   [ expression ] ;`

### Tests (+5 new, total 49)
- `test_parse_return_with_value`, `test_parse_bare_return`, `test_parse_break_with_value`, `test_parse_next_keyword`, `test_parse_return_inside_def_body`.

## [0.10.0] - 2026-05-22

### Added (Phase 6i — comparison operators `==`, `!=`, `<`, `>`, `<=`, `>=`)

Inserted a new precedence level into the expression hierarchy:
```
expression  =  sum { ( "==" | "!=" | "<=" | ">=" | "<" | ">" ) sum } ;
sum         =  term { ( PLUS | MINUS ) term } ;
term        =  factor { ( STAR | SLASH ) factor } ;
```

### Tests (+5 new, total 44)
- `test_parse_simple_comparison_has_sum_subnodes`, `test_parse_equality_in_assignment`, `test_parse_comparison_in_if_condition`, `test_parse_chained_inequality_left_associative`, `test_parse_plus_has_lower_precedence_than_comparison`.

## [0.9.0] - 2026-05-22

### Added (Phase 6h — no-paren method calls `puts 1` / `puts 1, 2`)
- `method_call_no_paren = ( NAME | KEYWORD ) expression { COMMA expression } ;`

### Tests (+5 new, total 39)
- `test_parse_no_paren_single_arg`, `test_parse_no_paren_multiple_args`, `test_paren_form_still_wins_over_no_paren`, `test_bare_name_falls_through_to_expression_stmt`, `test_no_paren_with_binary_arg_is_single_call`.

## [0.8.0] - 2026-05-22

### Added (Phase 6g — blocks `do … end` and `method { … }`)
- `method_with_block = ( NAME | KEYWORD ) [ LPAREN [ expression { COMMA expression } ] RPAREN ] block ;`
- `block = do_block | brace_block ;`
- `do_block = "do" [ block_params ] { !"end" statement } "end" ;`
- `brace_block = LBRACE [ block_params ] { statement } RBRACE ;`
- `block_params = "|" NAME { COMMA NAME } "|" ;`

### Tests (+6 new, total 34)
- `test_parse_method_with_do_block_no_params`, `test_parse_method_with_brace_block`, `test_parse_do_block_with_pipe_params`, `test_parse_brace_block_with_two_pipe_params`, `test_parse_method_call_with_args_and_block`, `test_parse_hash_literal_still_works_at_statement_position`.

## [0.7.0] - 2026-05-22

### Added (Phase 6f — `class Foo … end` / `module Foo … end` namespace declarations)
- `class_statement  = "class"  NAME { !"end" statement } "end" ;`
- `module_statement = "module" NAME { !"end" statement } "end" ;`
- Added as alternatives in `statement` right after `def_statement`, so the dispatch order is: def → class → module → if → unless → while → until → assignment → method_call → expression_stmt.
- The body's `Repetition { statement }` uses the same negative-lookahead `!"end"` as `def_statement` so the closing `end` doesn't get eaten by `expression_stmt → factor → KEYWORD`.
- Regenerated `src/_grammar.rs` via `grammar-tools compile-grammar`.

### Tests (+4 new, total 28)
- `test_parse_empty_class` — `class Foo\nend` parses and the first Name token is `"Foo"`.
- `test_parse_class_with_method_body` — `class Foo\n  def bar\n  end\nend` produces a class with at least one body `statement` (the nested `def`).
- `test_parse_empty_module` — `module M\nend` parses to a `module_statement` subnode.
- `test_parse_module_with_assignment_body` — `module M\n  x = 1\nend` keeps the body assignment under the module.

## [0.6.0] - 2026-05-20

### Added (Phase 6e — symbol literals `:foo` / `:"bar"`)
- `symbol_literal = COLON ( NAME | KEYWORD | STRING ) ;`
- Added as a `factor` alternative so symbols can appear wherever an expression is valid.
- Quoted symbols (`:"hello world"`) reuse the existing STRING token shape — the lexer strips the quotes, so the symbol's name is the inner content directly.

### Tests (+3 new, total 24)
- `:foo` (name), `:def` (keyword name — names can be Ruby keywords), `:"hello world"` (quoted).

## [0.5.0] - 2026-05-20

### Added (Phase 6d — array and hash literals)
- `array_literal = LBRACKET [ expression { COMMA expression } ] RBRACKET`
- `hash_literal  = LBRACE [ hash_entry { COMMA hash_entry } ] RBRACE`
- `hash_entry    = NAME COLON expression | expression "=>" expression`
- `factor` now accepts `array_literal` and `hash_literal` as alternatives, so they can appear anywhere an expression is valid.

### Tests (+4 new, total 21)
- Array literal `[1, 2, 3]`, empty array `[]`, hash shorthand `{a: 1, b: 2}`, hash rocket `{a => 1}`.

## [0.4.0] - 2026-05-20

### Added (Phase 6c — `while … end` / `until … end` loops)
- `while_statement = "while" expression { !"end" statement } "end"`
- `until_statement = "until" expression { !"end" statement } "end"`
- Added as alternatives in `statement` after the conditionals.
- Body uses the same `!"end"` negative-lookahead trick from Phase 6a/6b.

### Tests (+3 new, total 17)
- `while` with body, `until`, `while` with empty body.

## [0.3.0] - 2026-05-20

### Added (Phase 6b — `if … else … end` and `unless`)
- New grammar rules in `code/grammars/ruby.grammar`:

      if_statement     = "if" expression { !"else" !"elsif" !"end" statement }
                           { elsif_clause } [ else_clause ] "end"
      elsif_clause     = "elsif" expression { !"else" !"elsif" !"end" statement }
      else_clause      = "else" { !"end" statement }
      unless_statement = "unless" expression { !"else" !"end" statement }
                           [ else_clause ] "end"

- `if_statement` and `unless_statement` are added as alternatives in `statement` right after `def_statement`, so the dispatch order is: def → if → unless → assignment → method_call → expression_stmt.
- The body repetitions all use negative lookaheads (`!"end"`, `!"else"`, `!"elsif"`) so they stop short of the closing keyword — same trick as `def_statement` in Phase 6a.
- Regenerated `src/_grammar.rs`.

### Tests (+4 new, total 14)
- `if` with body, `if`/`else`, `if`/`elsif`/`else`, `unless`.

## [0.2.0] - 2026-05-20

### Added (Phase 6a — `def name(params) … end` method definitions)
- New `def_statement` grammar rule in `code/grammars/ruby.grammar`:
  - `def_statement = "def" NAME [ LPAREN [ params ] RPAREN ] { !"end" statement } "end"`
  - `params = NAME { COMMA NAME }`
- Added as the first alternative in `statement`, so `def`-led statements dispatch ahead of the `assignment` / `method_call` / `expression_stmt` branches.
- The body's `Repetition { statement }` uses a **negative lookahead** `!"end"` so it stops greedy-matching once the trailing `end` keyword comes into view.  Without the lookahead, `expression_stmt → factor → KEYWORD` would happily consume `end` itself as part of the body and leave nothing for the closing literal.
- Regenerated `src/_grammar.rs` via `grammar-tools compile-grammar`.

### Tests (+4 new, total 10)
- `test_parse_def_no_params_no_body` — `def foo()\nend`.
- `test_parse_def_with_params` — `def add(x, y)\nend` produces a `params` subnode with names `["x", "y"]`.
- `test_parse_def_with_body` — `def add(x, y)\n  x + y\nend` carries at least one body `statement`.
- `test_parse_def_without_parens` — `def foo\nend` works (parens optional per Ruby).

## [0.1.0] - 2026-03-21

### Added
- `create_ruby_parser(source)` — factory function that loads `ruby.grammar` and returns a configured `GrammarParser`.
- `parse_ruby(source)` — convenience function that parses Ruby source and returns a `GrammarASTNode`.
- Loads grammar from `ruby.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering assignments, expressions, method definitions, if/else, while loops, multiple statements, empty programs, class definitions, method calls, and the factory function.
