"""coding-adventures-sir-runtime-symbolic -- the SIR23 Tier A symbolic-
expression + pattern/rewrite runtime.

Semantic-IR-emitted Python imports this package when a compiled module
declares the SIR23 domain's Tier A pattern-matcher features (``code/specs/
SIR23-symbolic-pattern-semantic-ir.md``): ``Feature::SymbolicExpr`` /
``Feature::PatternMatching`` / ``Feature::Rationals``. A `SymSymbol` /
`SymRational` / `SymApply` / `SymPatternBlank` / `SymPatternNamed` /
`SymRule` / `SymReplaceAll` IR node each lower to a call into this package::

    from coding_adventures_sir_runtime_symbolic import (
        apply, int_, named, blank, rule, replace_all, sym, unwrap,
    )

    # x_ + 0 -> x_, applied once, everywhere in the tree
    x_pat = named("x", blank())
    add_zero_rule = rule(apply(sym("Add"), [x_pat, int_(0)]), x_pat)
    expr = apply(sym("Add"), [apply(sym("Add"), [sym("z"), int_(0)]), int_(0)])
    unwrap(replace_all(expr, [add_zero_rule]))   # IRSymbol(name='z')

This mirrors the TypeScript backend's *imported-package* model
(``semantic-ir-to-typescript`` imports ``@coding-adventures/
sir-runtime-symbolic``) rather than ``semantic-ir-to-python``'s usual
inline-runtime convention for its OOP/exceptions/pairs concerns -- see the
SIR23 spec's "Backend impact" section, and this repo's own
``coding-adventures-sir-runtime-array`` (SIR22) precedent, for why Python
follows TypeScript here.

**Tier A only.** This package implements exactly the seven `Expr` variants
above -- the term-tree type, the structural pattern matcher, and the two
tree-rewrite operators. It has no general expression EVALUATOR (`Add`/
`Sin`/`D`/etc. numeric or symbolic folding is explicitly out of scope, per
the SIR23 spec's own "Explicitly out of scope" section) -- a `SymApply`
builds an inert term tree, nothing more.

See ``code/specs/SIR23-symbolic-pattern-semantic-ir.md`` and this package's
own README/``runtime.py`` module docstring for the full API and the depth-
cap / iteration-cap DoS-guard design notes.
"""

from __future__ import annotations

from .runtime import (
    BLANK,
    MAX_TERM_DEPTH,
    PATTERN,
    RULE,
    RULE_DELAYED,
    Bindings,
    DepthLimitError,
    IRApply,
    IRNode,
    RewriteCycleError,
    apply,
    apply_rule,
    blank,
    blank_typed,
    int_,
    is_blank,
    is_pattern,
    is_rule,
    match_pattern,
    named,
    number_node,
    rational,
    replace_all,
    replace_repeated,
    rule,
    rule_delayed,
    string_node,
    substitute,
    sym,
    unwrap,
)

# Public alias: the emitted-code import header and ordinary callers bind
# this as `int`; internally `runtime.py` uses the trailing-underscore name
# `int_` to avoid shadowing the builtin `int` type it still needs for its
# own type annotations -- mirrors this repo's own
# `coding-adventures-sir-runtime-array`'s identical `range_`/`range` and
# `set_`/`set` precedent exactly.
int = int_  # noqa: A001  (intentional re-export under the SIR name)

__all__ = [
    "BLANK",
    "MAX_TERM_DEPTH",
    "PATTERN",
    "RULE",
    "RULE_DELAYED",
    "Bindings",
    "DepthLimitError",
    "IRApply",
    "IRNode",
    "RewriteCycleError",
    "apply",
    "apply_rule",
    "blank",
    "blank_typed",
    "int",
    "int_",
    "is_blank",
    "is_pattern",
    "is_rule",
    "match_pattern",
    "named",
    "number_node",
    "rational",
    "replace_all",
    "replace_repeated",
    "rule",
    "rule_delayed",
    "string_node",
    "substitute",
    "sym",
    "unwrap",
]
