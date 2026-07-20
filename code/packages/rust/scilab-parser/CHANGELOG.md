# Changelog

## [0.1.0] - 2026-07-20

### Added

- Initial grammar-driven Rust Scilab parser (MA10 §6, task MA-10c).
- `code/grammars/scilab/scilab.grammar`, forked from
  `code/grammars/matlab/matlab.grammar` at the grammar-source level (copied,
  then diverged) — this crate does not depend on `matlab-parser` (MA10 §5).
  Matrix literals, ranges, and indexing are inherited near-verbatim; the
  12-tier operator-precedence cascade (MA10 §3) is confirmed tier-for-tier
  identical in relative order to MATLAB's own cascade.
- Three parser-level divergences from `matlab.grammar` (MA10 §3):
  1. `stmt_sep = "then" | "do" | COMMA | NEWLINE ;` — one new production,
     reused at exactly six header sites (`if`, `elseif`, `select`, `case`,
     `while`, `for`), each individually replaceable by a bare comma or
     newline instead of the linker keyword. Modeled as a single
     REQUIRED-but-flexible separator (not two optional pieces stacked),
     since real Scilab's `then`/`do` replace the punctuation rather than
     requiring it in addition.
  2. `endfunction` kept as its own distinct closing production
     (`func_def`'s only closer), textually separate from the generic `end`
     `if_stmt`/`while_stmt`/`for_stmt`/`select_stmt` all reduce to (MA10 §1
     finding 7) — a bare `end` cannot close a function by construction.
  3. `$` (`DOLLAR`) replaces MATLAB's context-sensitive `end`-as-last-index
     retagging hook entirely — added as an ordinary `primary` alternative
     (the same tier as `NUMBER`/`NAME`), so `A($)`, `A($-1)`, and a bare `$`
     all parse as an ordinary atom composing with arithmetic. `PERCENT_CONST`
     (the eight `%`-prefixed special constants) is added at the identical
     tier for the identical reason.
- `select_stmt`/`case_clause` — Scilab's own multi-way conditional, replacing
  MATLAB's `switch`/`otherwise` entirely (neither spelling exists in
  `scilab.tokens`, MA10 §1 finding 4): `select expr stmt_sep { case_clause }
  [ else_clause ] "end"`, `case_clause = "case" expr stmt_sep block_body`.
  `select` shares the same `else_clause` production `if_stmt` uses.
- No `try`/`catch`/`return`/`global`/`persistent`/lambda productions at all —
  none of those tokens exist in `scilab.tokens` (MA10 §4 scopes them out).
- A KNOWN, EXPECTED `grammar-tools validate` cross-check false positive:
  `primary`'s `STRING` reference is reported as undefined because
  `scilab.tokens` deliberately leaves `DQ_STRING` un-aliased (so its own
  `collapse_dq_string_escapes` post-hook can tell the difference between "not
  yet decoded" and "already STRING") — unlike `matlab.tokens`, which declares
  `-> STRING` and registers the name statically. Both `STRING_PLACEHOLDER`-
  and `DQ_STRING`-derived tokens are relabelled to `type_name = "STRING"` by
  `scilab-lexer`'s own post-tokenize hooks before parsing begins, and
  `GrammarParser::match_token_reference` matches purely on live `type_name`,
  so `STRING` here is functionally correct despite the static validator's
  false positive — documented at length in `scilab.grammar`'s own header
  comment and this crate's own README rather than silently worked around
  (fixing it at the source would require modifying the already-merged
  `scilab.tokens`, out of scope for this PR).
- A bespoke `MAX_RULE_DEPTH = 125` recursion-depth cap. Seven structurally
  distinct self-referential shapes were measured independently
  (parenthesised nesting, a flat right-recursive power (`^`) chain, a unary
  prefix chain (`- - - … x` / `~ ~ ~ … x`), chained assignment
  (`x=x=x=…=5`), deeply nested `if`/`end`, function-call/cell-index argument
  nesting (`f(f(f(…)))`), and matrix-literal nesting), per MA10 §6's own
  directive and the "measure, don't assume one shape's floor bounds the
  others" methodology `apl-parser`/`j-parser`/`maple-parser` each
  independently established — reinforced here by a security-review round
  that caught two shapes (chained assignment, argument nesting) and a third
  acknowledged-but-unmeasured shape (unary prefix) missing from the initial
  four-shape survey. `select`/`end` shares the identical `statement ->
  if_stmt` reachability as nested `if`, cell-index nesting shares
  call-argument nesting's identical `arg_list`-mediated reachability, and
  cell-literal nesting shares matrix-literal's identical shape, so none of
  those three were separately measured (a provable rule-graph identity, not
  an assumed shape resemblance). Every "flat chain of one operator"
  production written with EBNF `{ x }` repetition costs zero native stack
  regardless of width, confirmed by reading `parser::grammar_parser`'s own
  `Repetition` implementation directly.
- A genuine surprise once each shape's *nesting-count* crash floor was
  converted into *rule-frame* terms (the units `MAX_RULE_DEPTH` actually
  bounds): chained assignment tolerates far more nesting levels than the
  power chain (162 vs. 101) yet has the *lower* rule-frame floor (179 safe /
  180 crash, vs. 220/221) — each assignment link's persisting per-level cost
  is a single rule-frame, cheaper in frame-count terms than every other
  measured shape, yet its specific self-referential call path evidently
  costs more native-stack bytes per crossing. Neither "the shape that
  tolerates the fewest levels must bind" nor "parenthesised nesting binds,
  since it does for nearly every sibling `*-parser` crate in this repo"
  holds here — both would have shipped a cap unsafe specifically for chained
  assignment. `125` sits about 30.2% below the binding 179 floor (comparable
  to `reduce-parser`'s ~28.5%, `apl-parser`'s ~26.5%, `j-parser`'s ~30%,
  `derive-parser`'s ~33%, `maple-parser`'s ~31.2%), and therefore safely
  below all six other rule-frame floors (295, 220, 268, 289, 277, 219) too.
  Full measurement tables and reasoning in `MAX_RULE_DEPTH`'s own doc comment
  (`src/lib.rs`). Known, disclosed limitation: `while`/`for`/nested-`function`
  bodies form the same `statement`-cycle shape as nested `if` (measured floor
  268) but were not independently measured — they are structurally closer to
  that shape than to the ones that turned out to diverge (chained assignment,
  unary prefix), and 125 sits 143+ units below 268, so risk is assessed as
  low, but this is a completeness gap for a future audit to close rather than
  a claim of exhaustive coverage.
- 55 tests + 1 doctest covering: every control-flow construct with and
  without its linker keyword (`if`/`elseif`/`else`/`end`,
  `select`/`case`/`else`/`end`, `while`/`end`, `for`/`end`), confirming a
  bare comma/newline alone (no linker keyword at all) is also valid at every
  one of the six `stmt_sep` sites; `function ... endfunction` (multiple
  return values, no return value, no parameters at all, and a regression
  confirming a bare `end` does NOT close a function); `switch`/`otherwise`
  confirmed to remain ordinary syntax errors (Scilab has neither spelling);
  the full precedence cascade (one test per tier boundary, from
  `additive`/`multiplicative` up through `logical_or` being loosest, `unary`
  binding looser than `power`, right-associative `power`, and postfix
  transpose binding tightest); `$` as an ordinary expression atom (`A($)`,
  `A($-1)` composing with `additive`, a bare `$` as a legal statement, and
  `$` composing with a following transpose); `PERCENT_CONST` as a primary
  (all eight constants); both not-equal spellings (`~=`/`<>`) reaching the
  same `comparison` tier; matrix/cell literals and ranges (inherited,
  confirmed still correct); and 4 depth-guard tests exercising all four
  measured shapes at once (deep adversarial input on an enlarged-stack
  thread returns a clean error for every shape, the cap trips before the
  native stack would overflow even on a default-stack thread for every
  shape, reasonable hand-written nesting for every shape stays well under
  the cap, and an exact boundary test proving the measured real-input
  headroom for every shape parses cleanly one level below the cap and trips
  cleanly one level at the cap).
- `code/grammars/scilab/scilab.grammar` validated with
  `grammar-tools validate-grammar` (44 rules, clean) and cross-validated
  against `code/grammars/scilab/scilab.tokens` with `grammar-tools validate`
  (one known, expected, documented false positive on `STRING` — see above —
  and two consequent "unused token" warnings for `STRING_PLACEHOLDER`/
  `DQ_STRING`, which the grammar correctly never references directly).
