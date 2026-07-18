# Changelog

## [0.1.0] - 2026-07-18

### Added

- Initial `reduce-runtime` crate (MA08 R-4, front2 Wave 5): lowers the
  `reduce-parser` (R-3) `GrammarASTNode` CST into `symbolic_ir::IRNode` and
  evaluates it via `symbolic_vm::VM` over the shared `SymbolicBackend` —
  unchanged, no custom `Backend` — mirroring `derive-runtime`'s identical
  reuse story for the other Wave-5 CAS-family language.
- `lower` module: the full precedence-cascade lowering (`assignment`
  through `atom`, plus `if_expr`/`group_expr`/`cons`/`list_literal`), the
  lowercase-surface→canonical head bridge (`list`→`List`, `first`→`First`,
  `second`→`Second`, `third`→`Third`, `rest`→`Rest`, `part`→`Part`,
  `append`→`Append`, `reverse`→`Reverse`), `:=` assignment-vs-procedure-
  definition disambiguation by LHS shape (Reduce, like Derive, has only one
  assignment token), and the cons-onto-a-literal-list fold MA08 §3
  documents (`a . {b, c}` → `List[a, b, c]`, not a standalone `Cons` node;
  right-associative, so `a . b . {c}` folds through every link).
- `printer` module: the inverse — canonical heads back to Reduce surface
  notation (infix arithmetic/comparison/logic, curly-brace `{a, b, c}`
  lists, `if ... then [... else ...]`, `<< s1; s2; ... >>`), with its own
  precedence ladder mirroring `reduce.grammar`'s cascade (one tier more
  than `derive-runtime::printer`'s table, for the `cons` tier Derive has no
  analogue of).
- `ReduceSession`/`eval`: a string-in/string-out facade mirroring
  `derive-runtime`'s session pattern, **without** a numbered-worksheet
  prefix — MA08 §2/§5 are explicit that Reduce's own session transcript has
  no numbered-input convention the way Derive's `#n:` or Wolfram's
  `In[n]:=` do, so `Output` here carries only rendered text and `feed`
  never prints a prefix.
- Robustness: `MAX_INPUT_LEN` (64 KiB) bounds total input;
  `MAX_STATEMENT_TOKENS` (2000, measured against the real `reduce-lexer`
  token stream) closes the "long flat chain folds into a deep lowered
  tree" vector that `reduce-parser`'s own `MAX_RULE_DEPTH` cannot cover
  (grammar repetitions, not recursive rule calls, aren't bounded by that
  cap). Unlike `derive-runtime`'s identical-in-spirit guard (reset on a
  significant `NEWLINE`), this crate resets on Reduce's own real
  terminators, `SEMI`/`DOLLAR` — but **only at bracket depth 0** (tracked
  over `LPAREN`/`RPAREN`, `LBRACE`/`RBRACE`, `GROUP_OPEN`/`GROUP_CLOSE`),
  a `/security-review` fix over resetting unconditionally: a `;`/`$`
  lexically inside a *parenthesized* `<< ... >>` group construct that is
  itself embedded as one operand of a much larger enclosing chain
  (`1 + 1 + (<<0;0>>) + 1 + 1 + ...`) previously reset the outer chain's
  own count too, even though the outer chain still folds every operand
  into one deeply nested tree regardless — empirically, a chain 16× the
  documented cap sailed through undetected before this fix. Two shapes are
  guarded: the additive/multiplicative left-fold chain (shared with
  Derive's identical vector) and, new here, a chained postfix call
  `f(x)(x)(x)…` (`postfix`'s own call-chaining loop folds the same way).
  Evaluation runs on a 512 MiB-stack worker thread inside `catch_unwind`,
  rebuilding the session after any caught panic; a thread-spawn failure
  itself (a second `/security-review` fix, over an earlier `.expect()`
  that panicked on the *calling* thread, outside that boundary) is now
  folded into the same ordinary `Err` path instead.
- **Disclosed spec/reality gap** (documented in `crate::lower`'s module
  doc comment, this crate's README, and MA08 itself — see that spec's own
  R-4 status note): grepping `symbolic-vm::handlers::build_handler_table`
  confirms **no** handler exists for `CompoundExpression`, `First`/
  `Second`/`Third`/`Rest`/`Part`/`Append`/`Reverse`, or `Cons` — MA08 §5's
  claim that these are "already implemented for Macsyma/Wolfram/Derive" is
  true only via a *bespoke* `Backend` specific to each of those languages,
  not the shared `SymbolicBackend` R-4 is required to reuse unchanged. This
  crate still lowers to the structurally-correct heads MA08 §3 documents
  (arguments evaluate, including any `Assign`/`Define` side effects, in
  order), but the calls themselves stay unevaluated rather than performing
  the operation — the same "no handler, no crash" contract an undefined
  user function already has. Also disclosed: MA08 §3's own prose describes
  the arithmetic heads as `Plus`/`Subtract`/`Times`/`Power` (with `/`
  and unary `-` expanded into `Times`/`Power` applications); none of those
  spellings exist in `symbolic-ir` at all. This crate uses the real,
  already-reused heads instead — `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`,
  exactly what `derive-runtime`/`macsyma-compiler` already lower to — so
  Reduce agrees with its CAS-family siblings on every arithmetic result.
- 85 tests total: `lower`/`printer` unit tests covering every row of MA08
  §3's surface table, plus end-to-end session tests (arithmetic, persistent
  bindings/procedures, `if` with/without `else`, lists and cons-folding,
  both robustness guards including the group-statement-nested case and the
  embedded-group-inside-a-larger-chain regression, panic recovery, and the
  disclosed `CompoundExpression`/list-accessor gap), plus a doctest on
  `ReduceSession::feed`.
