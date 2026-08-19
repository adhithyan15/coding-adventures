"""sir-runtime-symbolic -- the SIR23 Tier A symbolic-expression + pattern/
rewrite runtime imported by Semantic-IR-emitted Python, bound at the single
``coding_adventures_sir_runtime_symbolic`` import site (see
``code/specs/SIR23-symbolic-pattern-semantic-ir.md``'s "Backend impact"
section). A compiled Wolfram/Macsyma/Maxima program's ``SymSymbol``/
``SymRational``/``SymApply``/``SymPatternBlank``/``SymPatternNamed``/
``SymRule``/``SymReplaceAll`` IR nodes all become calls into this module at
runtime.

Why this package is thin
-------------------------

The hard part -- a term-tree type and a faithful structural pattern
matcher/substitution algorithm -- already exists as two separate, already
*published* packages this one builds on rather than re-invents:

- ``coding-adventures-symbolic-ir`` -- the term-tree type (``IRNode``: one
  of ``IRSymbol``/``IRInteger``/``IRRational``/``IRFloat``/``IRString``/
  ``IRApply``) this whole domain is built on.
- ``coding-adventures-cas-pattern-matching`` -- the ``Bindings``/``match``/
  ``apply_rule`` five-case structural matcher (``Blank()``, ``Blank(head)``,
  ``Pattern(name, inner)``, compound-vs-compound, structural equality) --
  see that package's own docs for the full algorithm.

This module re-exports those primitives under SIR23's own vocabulary (see
``__init__.py``), and adds exactly the three things neither sibling package
has: (1) a Python-idiomatic constructor surface (``sym``/``int_``/
``number_node``/``rational``/``string_node``/``apply``) matching the
published TypeScript sibling package's own constructor names, (2)
``replace_all``/``replace_repeated`` -- the ``/.``/``//.`` tree-wide
replacement operators SIR23 requires, which neither dependency exposes as a
matched pair, and (3) an explicit recursion-depth cap on the parts of this
module that walk a full, potentially attacker-influenced runtime expression
tree (see "Depth safety" below).

This is a direct structural port of the published TypeScript package
``@coding-adventures/sir-runtime-symbolic`` (``code/packages/typescript/
sir-runtime-symbolic/src/index.ts``) -- same public surface, same
algorithms, adapted to Python's own ``symbolic-ir``/``cas-pattern-matching``
sibling packages (which differ in a few naming/shape details from their
TypeScript counterparts -- see individual docstrings below for each spot
that diverges and why).

Two new operations, one existing one reused as-is
---------------------------------------------------

``cas_pattern_matching.rewrite()`` already conflates "walk bottom-up" with
"retry every rule at each node until none fire" (a fixed point) -- that IS
``replace_repeated``'s exact contract, so ``replace_repeated`` below mirrors
its algorithm closely (see "Depth safety" for why it is a parallel
implementation rather than a direct call). But ``cas_pattern_matching`` has
no equivalent of Wolfram's ``/.`` (``ReplaceAll``): try each rule **once**
per subtree, first match wins, no retry at that position, no fixed point.
``replace_all`` below is genuinely new code, not a port of anything in the
Rust, TypeScript, or Python reference packages.

Depth safety
------------

``replace_all``/``replace_repeated`` walk the *entire* target expression
tree -- ordinary runtime data a compiled program can build up to unbounded
depth (e.g. many rounds of nested computation) -- so both functions cap that
walk's recursion at ``MAX_TERM_DEPTH`` (below). ``cas_pattern_matching.
rewrite()`` has no such cap; carrying that gap forward into a runtime meant
to execute compiled, potentially attacker-influenced programs would reopen
the exact class of stack-overflow DoS (CWE-674) this repo's other SIR
passes already guard against (``semantic-ir::limits::MAX_IR_DEPTH``, the
``semantic-ir`` walker's depth-bounded ``Visitor`` default implementations,
and the TypeScript/JavaScript/Ruby siblings' own ``MAX_TERM_DEPTH``
guards). Fixed at **512** to match every other backend's cap exactly
(cross-backend consistency is deliberate, not a coincidence: the same
compiled program should hit the same depth ceiling no matter which backend
emitted it).

**A rule's own ``lhs``/``rhs`` needs the SAME cap, independent of the
target's depth -- found by this package's own ``/security-review``, not
assumed safe by design.** An earlier version of this module claimed
``match_pattern``/``substitute``/``apply_rule`` needed no cap because a
rule's pattern/RHS is "authored by a compiler frontend... not by runtime
data" -- but nothing in this runtime actually enforces that: a compiled
SIR23 program can build an arbitrarily deep ``lhs``/``rhs`` via an ordinary
loop calling ``apply``/``named`` (the exact same constructors used to build
the *target*), with no dependency on the target's own depth at all.
``replace_all(shallow_target, [rule(blank(), huge_rhs)])`` used to raise an
uncaught Python ``RecursionError`` from inside ``substitute``/``match``'s
own recursion -- neither of which the target-tree walk's cap ever reaches,
since that recursion is driven by the *rule's* structure, not the target's.
``replace_all``/``replace_repeated`` now validate every rule's ``lhs``/
``rhs`` against ``MAX_TERM_DEPTH`` up front (via ``_rules_exceed_depth``,
an ITERATIVE, non-recursive check -- so checking a maliciously deep rule
cannot itself blow the stack) before ever starting the target-tree walk;
see that function's own docstring and
``test_deep_rule_rhs_reports_depth_limit_error_not_a_crash``/
``test_deep_rule_lhs_pattern_chain_reports_depth_limit_error_not_a_crash``
for the regression coverage. ``match_pattern``/``substitute``/``apply_rule``
themselves remain uncapped as raw, lower-level primitives (matching
``cas_pattern_matching``'s own uncapped design) -- SIR23 Tier A codegen
never calls them directly, only through the now-guarded
``replace_all``/``replace_repeated``.

``replace_repeated`` additionally needs a **separate**, independent
``max_iterations`` cap (default 100, matching every sibling backend's own
default) on its fixed-point loop -- an unbounded ``//.`` is a guaranteed
non-terminating program for some inputs (SIR23 spec, "Matcher semantics"
point 6). This is CPU-time bounding, not stack-depth bounding, and the two
guards are enforced independently: a rule firing repeatedly at ONE tree
position must cost O(1) native stack frames, not O(firings) -- see
``replace_repeated``'s own docstring for the loop-vs-recursion distinction
this guards against.

A deliberate Python-native divergence from the TypeScript reference: no
``equals()`` helper
--------------------

The TypeScript reference imports an explicit ``equals()`` helper from
``symbolic-ir`` because JS's ``===``/``==`` do not perform deep structural
comparison on plain objects. This package's ``IRNode`` subclasses are all
``@dataclass(frozen=True, slots=True)`` (see ``symbolic_ir.nodes``), which
already generates a recursively-structural ``__eq__`` -- ``IRApply``'s
``args`` field is itself a tuple of ``IRNode``\\ s, so Python's own ``==``
cascades correctly with no help needed. ``replace_repeated`` below therefore
just uses plain ``!=`` where the TypeScript reference calls ``!equals(...)``.

Another deliberate divergence: no ``is_depth_limit_error``/
``is_rewrite_cycle_error`` helper functions
---------------------------------------------

The TypeScript reference needs ``isDepthLimitError``/``isRewriteCycleError``
functions because a plain ``{ kind: "depth-limit", ... }`` object literal
has no distinguishing runtime type TypeScript's structural typing can probe
directly. This package instead defines :class:`DepthLimitError` and
:class:`RewriteCycleError` as real classes, so ``isinstance(result,
DepthLimitError)`` already does the job -- exporting a same-named wrapper
function around ``isinstance`` would add a layer with no behavior of its
own, so this package simply doesn't.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from typing import Literal, TypeGuard, cast

from cas_pattern_matching import (
    BLANK,
    PATTERN,
    RULE,
    RULE_DELAYED,
    Bindings,
    Blank,
    Pattern,
    Rule,
    RuleDelayed,
    is_blank,
    is_pattern,
    is_rule,
)
from cas_pattern_matching import apply_rule as _cas_apply_rule
from cas_pattern_matching import match as _cas_match
from cas_pattern_matching import pattern_name as _pattern_name
from symbolic_ir import (
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRString,
    IRSymbol,
)

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

# ---------------------------------------------------------------------------
# Leaf/compound term constructors (symbolic-ir, Python-idiomatic naming)
# ---------------------------------------------------------------------------
#
# These exist so a compiled `SymSymbol`/`SymRational` node -- or a bare
# `IntLit`/`FloatLit`/`StrLit` appearing as a child of a `SymApply`/
# `SymRule`/`SymReplaceAll` -- has somewhere to get wrapped into a real
# `IRNode` at the single import site a generated module uses; see the
# SIR23 codegen in `semantic-ir-to-python`'s `emit_sym_operand`.
#
# Named to match the published TypeScript sibling package's own
# `sym`/`int`/`numberNode`/`rational`/`stringNode`/`apply` names, snake_cased
# -- except `int`, which would shadow the builtin `int` type for the rest of
# this module; the real implementation is `int_` (mirrors this repo's own
# `coding-adventures-sir-runtime-array` `range_`/`range` and `set_`/`set`
# precedent), re-exported as `int` from `__init__.py` only, where nothing
# else in that thin module needs the builtin name.


def sym(name: str) -> IRNode:
    """Build a bare symbolic-expression symbol -- Wolfram `x`, `Plus`, `f`
    used as data. The runtime representation of a `SymSymbol` IR node."""
    return IRSymbol(name)


def int_(value: int) -> IRNode:
    """Wrap a host integer as a symbolic-expression leaf term. Python's
    `int` is already arbitrary-precision, so no bigint class is needed --
    this is a thin, allocating wrapper, not a conversion."""
    return IRInteger(value)


def number_node(value: float) -> IRNode:
    """Wrap a host float as a symbolic-expression leaf term."""
    return IRFloat(value)


def rational(numer: int, denom: int) -> IRNode:
    """Build an exact rational scalar in reduced form -- Wolfram `1/3`,
    `Rational[1, 3]`. The runtime representation of a `SymRational` IR
    node. `IRRational.__post_init__` reduces `numer`/`denom` by their gcd
    and normalizes the sign into the numerator at construction time (see
    `symbolic_ir.nodes.IRRational`'s own docstring) -- this function does
    not duplicate that logic, it is a pure pass-through to the dataclass
    constructor which already does it."""
    return IRRational(numer, denom)


def string_node(value: str) -> IRNode:
    """Wrap a host string as a symbolic-expression leaf term."""
    return IRString(value)


def apply(head: IRNode, args: Sequence[IRNode]) -> IRNode:
    """Build a symbolic-expression compound term `head(args...)` -- the
    runtime representation of a `SymApply` IR node. `args` is copied into a
    tuple (`IRApply` stores its arguments as a tuple, not a list, so the
    node stays hashable)."""
    return IRApply(head, tuple(args))


# ---------------------------------------------------------------------------
# Pattern / rule vocabulary
# ---------------------------------------------------------------------------
#
# `BLANK`/`PATTERN`/`RULE`/`RULE_DELAYED` are re-exported unchanged above
# (they are already SCREAMING_SNAKE `IRSymbol` singletons in
# `cas_pattern_matching.nodes`, matching SIR23's own vocabulary names
# exactly). `is_blank`/`is_pattern`/`is_rule` are likewise re-exported
# unchanged. The functions below wrap `cas_pattern_matching`'s
# `Blank`/`Pattern`/`Rule`/`RuleDelayed` constructors under the names the
# published TypeScript sibling package uses (snake_cased), and split
# `cas_pattern_matching.Blank`'s single `head: str | None = None` parameter
# into two separate functions (`blank`/`blank_typed`) to match that same
# TypeScript surface -- `cas_pattern_matching.Blank` itself already accepts
# `head` as a plain `str` (unlike the TypeScript reference's `blankTyped`,
# which takes a full `IRNode` head-constraint), which is simpler and is used
# as-is here rather than re-wrapped into an `IRNode`-typed parameter.


def blank() -> IRNode:
    """Build an anonymous wildcard pattern -- Wolfram `_`. Matches any
    single expression, unconstrained."""
    return Blank()


def blank_typed(head: str) -> IRNode:
    """Build a head-constrained wildcard pattern -- Wolfram `_h`. Matches
    only a subtree whose own "head" (per Wolfram's `Head[]` convention,
    `symbol_matching.matcher._effective_head_name`'s type-tag for a leaf --
    `"Integer"`, `"Symbol"`, `"Rational"`, `"Float"`, `"String"` -- or a
    compound's head-symbol name) equals `head` exactly."""
    return Blank(head)


def named(name: str, pattern: IRNode) -> IRNode:
    """Build a named pattern variable -- Wolfram `x_` (`named("x",
    blank())`) or `x_h` (`named("x", blank_typed("h"))`). Binds `name` to
    whatever subtree `pattern` matches, for the rest of that match attempt.
    The runtime representation of a `SymPatternNamed` IR node."""
    return Pattern(name, pattern)


def rule(lhs: IRNode, rhs: IRNode) -> IRNode:
    """Build an eager-substitution rewrite rule -- Wolfram `lhs -> rhs`.
    The runtime representation of a `SymRule { delayed: false }` IR node.

    Current behavior note: `rule` and :func:`rule_delayed` produce data
    that is matched and substituted *identically* today -- this package has
    no general expression evaluator (per the SIR23 spec's own "Explicitly
    out of scope" section; Tier B, wiring the `cas-*` algorithm surface
    into this runtime, is separate, later work), so there is nothing yet
    for "eager" (evaluate the RHS once, at rule-construction time) to
    actually evaluate. The `delayed` bit still round-trips faithfully
    through the data model -- `rule`/`rule_delayed` construct distinct
    `Rule`/`RuleDelayed` sentinel heads -- so a future PR that adds a real
    evaluator has a clean, already-tested seam to branch on. Mirrors the
    published TypeScript sibling package's identical `rule` docstring."""
    return Rule(lhs, rhs)


def rule_delayed(lhs: IRNode, rhs: IRNode) -> IRNode:
    """Build a delayed-substitution rewrite rule -- Wolfram `lhs :> rhs`.
    The runtime representation of a `SymRule { delayed: true }` IR node.
    See :func:`rule`'s docstring for the current (identical-to-`rule`)
    behavior and what is deferred."""
    return RuleDelayed(lhs, rhs)


def match_pattern(
    pattern: IRNode, target: IRNode, bindings: Bindings | None = None
) -> Bindings | None:
    """Try to match `pattern` against `target`. Returns the resulting
    :class:`Bindings` on success or `None` on failure. Pass an existing
    `bindings` to extend it; the matcher never mutates it. Thin wrapper
    over `cas_pattern_matching.match`, renamed to match the published
    TypeScript sibling package's own `matchPattern` export (Python's
    `cas_pattern_matching` chose the shorter name `match` for the same
    function)."""
    return _cas_match(pattern, target, bindings)


def substitute(template: IRNode, bindings: Bindings) -> IRNode:
    """Replace every named-pattern reference in `template` with its
    captured binding. `template` is typically a rule's RHS -- an ordinary
    IR tree in which any `Pattern(name, _)` (i.e. a `named(...)` result)
    refers to a value `bindings` captured during matching; it expands to
    that captured value. An unbound pattern reference in `template` is left
    as-is (matches `apply_rule`'s own forgiving convention -- a stricter
    implementation would raise, but this keeps half-finished rules usable
    during exploration).

    `cas_pattern_matching.rewriter` has an identical `_substitute` helper,
    but it is module-private (leading underscore, not part of that
    package's public `__all__`) -- unlike the published TypeScript sibling
    package, whose `cas-pattern-matching` dependency exports `substitute`
    publicly. Reaching into another package's private name is exactly the
    kind of implicit cross-package coupling this repo's own conventions
    avoid, so this function is a small, self-contained reimplementation of
    the same algorithm (using only `cas_pattern_matching`'s PUBLIC
    `is_pattern`/`pattern_name`... surface plus `symbolic_ir.IRApply`)
    rather than a private-attribute reach-through."""
    if is_pattern(template):
        assert isinstance(template, IRApply)  # for type-narrowing
        name = _pattern_name(template)
        if name in bindings:
            return bindings[name]
        return template

    if isinstance(template, IRApply):
        new_head = substitute(template.head, bindings)
        new_args = tuple(substitute(a, bindings) for a in template.args)
        return IRApply(new_head, new_args)

    return template


def apply_rule(rule_node: IRNode, expr: IRNode) -> IRNode | None:
    """Try `rule_node` against `expr` at the root. Return the rewritten IR
    or `None` if the rule's `lhs` did not match. `rule_node` must be a
    `Rule(lhs, rhs)`/`RuleDelayed(lhs, rhs)` apply (i.e. built via
    :func:`rule`/:func:`rule_delayed`, or satisfying :func:`is_rule`) --
    passing anything else raises a `ValueError` (from
    `cas_pattern_matching.apply_rule`'s own `is_rule` check). Thin,
    unmodified re-export of `cas_pattern_matching.apply_rule` under this
    package's own name -- exposed here (rather than left for callers to
    import from `cas_pattern_matching` directly) because
    :func:`replace_all`/:func:`replace_repeated` below both build on it,
    matching the published TypeScript sibling package's own re-export.

    `rule_node` is typed as the general `IRNode` (not `IRApply`) precisely
    so a malformed non-rule value reaches `cas_pattern_matching`'s own
    `ValueError` at runtime instead of an unrelated `AssertionError` from a
    type-narrowing `assert` in this wrapper -- `cast` (not `assert`)
    below narrows for mypy without adding its own runtime check."""
    return _cas_apply_rule(cast("IRApply", rule_node), expr)


# ---------------------------------------------------------------------------
# Depth guard
# ---------------------------------------------------------------------------

#: Maximum recursion depth for :func:`replace_all`/:func:`replace_repeated`'s
#: tree walk. See the module docstring's "Depth safety" section for why only
#: these two functions (not the matcher primitives above) need this cap.
#: Fixed at the SAME value as every other Semantic-IR backend's own
#: `MAX_TERM_DEPTH` (TypeScript, JavaScript, Ruby) -- this is a deliberate
#: cross-backend constant, NOT independently re-derived per backend, so a
#: compiled program hits the same depth ceiling regardless of which backend
#: emitted it.
MAX_TERM_DEPTH = 512

#: Default fixed-point iteration cap for :func:`replace_repeated`, matching
#: every sibling backend's own default.
_DEFAULT_MAX_ITERATIONS = 100


@dataclass(frozen=True, slots=True)
class DepthLimitError:
    """Returned by :func:`replace_all`/:func:`replace_repeated` when the
    walk's recursion depth would exceed :data:`MAX_TERM_DEPTH`. A real class
    (not a Rust/TypeScript-style tagged dict) so `isinstance` alone
    distinguishes it -- see the module docstring's divergence note."""

    kind: Literal["depth-limit"] = "depth-limit"
    max_depth: int = MAX_TERM_DEPTH


@dataclass(frozen=True, slots=True)
class RewriteCycleError:
    """Returned by :func:`replace_repeated` when `max_iterations` fixed-
    point retries are exhausted without reaching a stable term -- an
    unbounded `//.` is a guaranteed non-terminating program for some rule
    sets (SIR23 spec, "Matcher semantics" point 6)."""

    kind: Literal["rewrite-cycle"] = "rewrite-cycle"
    max_iterations: int = _DEFAULT_MAX_ITERATIONS


_WalkError = DepthLimitError | RewriteCycleError


def unwrap(result: IRNode | DepthLimitError | RewriteCycleError) -> IRNode:
    """Unwrap a :func:`replace_all`/:func:`replace_repeated` result,
    raising a plain, catchable `ValueError` if the tree walk hit its depth
    cap or (for `replace_repeated`) its iteration cap instead of returning
    a real `IRNode`. Both functions return an error *value* rather than
    raising directly (see their own docstrings), because returning a value
    lets a caller that wants to inspect the failure kind do so without a
    `try`/`except`. Semantic-IR-emitted code has no such caller today -- a
    `SymReplaceAll` is an ordinary expression that must evaluate to a term
    value or fail loudly, never silently hand a `DepthLimitError`/
    `RewriteCycleError` sentinel to code expecting an `IRNode` -- so every
    compiled call site routes through this helper instead of using the raw
    result directly (mirrors `coding-adventures-sir-runtime-array`'s own
    plain-`ValueError`-with-descriptive-message convention)."""
    if isinstance(result, (DepthLimitError, RewriteCycleError)):
        raise ValueError(f"sir-runtime-symbolic: {result.kind}")
    return result


def _is_walk_error(value: IRNode | _WalkError) -> TypeGuard[_WalkError]:
    """`TypeGuard`, not a plain `bool` return -- lets mypy narrow `value`
    to `_WalkError` (in the `if` branch) or `IRNode` (after it) at each
    `replace_repeated` call site below, with no extra `assert` needed."""
    return isinstance(value, (DepthLimitError, RewriteCycleError))


# ---------------------------------------------------------------------------
# Rule pre-flight depth check -- SECURITY (CWE-674), see this section's own
# docstrings for the vulnerability this closes
# ---------------------------------------------------------------------------
#
# `_walk_once`/`replace_repeated`'s `walk` cap the recursion that descends
# into the TARGET expression (`expr`). They do NOT, on their own, bound the
# recursion `apply_rule` performs inside `cas_pattern_matching.match`/this
# module's own `substitute` -- and that recursion's depth is driven by the
# RULE's `lhs`/`rhs` structure, not by `expr`'s. This module's own earlier
# docstrings claimed that recursion was safe because a rule's pattern/RHS is
# "authored by a compiler frontend or written as a rule literal, not by
# runtime data" -- but nothing in this runtime actually enforces that: a
# compiled SIR23 program can build an arbitrarily deep `lhs`/`rhs` via an
# ordinary loop calling `apply`/`named` (exactly the same constructors used
# to build the target), completely independent of how deep the target
# itself is. `replace_all(shallow_target, [rule(blank(), huge_rhs)])`
# previously raised an uncaught Python `RecursionError` instead of the
# documented `DepthLimitError` -- found by this package's own
# `/security-review`; see `test_deep_rule_rhs_reports_depth_limit_error_
# not_a_crash`/`test_deep_rule_lhs_pattern_chain_reports_depth_limit_error`
# for the regression coverage.
#
# The fix: validate every rule's `lhs`/`rhs` depth ONCE, up front, before
# `replace_all`/`replace_repeated` ever start walking `expr` -- using an
# ITERATIVE (explicit-stack, non-recursive) walk, so checking a
# maliciously-deep rule cannot itself overflow the stack (the exact class
# of bug this check exists to prevent). Rules are immutable and fixed for
# the whole call (never mutated mid-walk), so one up-front check is
# sufficient for the ENTIRE `replace_all`/`replace_repeated` call -- there
# is no need to re-check on every `apply_rule` attempt.


def _term_exceeds_depth(node: IRNode, max_depth: int) -> bool:
    """Iterative (non-recursive) check: does `node`'s `IRApply` nesting
    exceed `max_depth` anywhere? Uses an explicit stack, not native Python
    recursion -- checking an attacker-deep `node` must never itself risk a
    `RecursionError`, which would defeat the entire point of this guard."""
    stack: list[tuple[IRNode, int]] = [(node, 0)]
    while stack:
        current, depth = stack.pop()
        if depth > max_depth:
            return True
        if isinstance(current, IRApply):
            stack.append((current.head, depth + 1))
            for arg in current.args:
                stack.append((arg, depth + 1))
    return False


def _rules_exceed_depth(rules: Sequence[IRNode], max_depth: int) -> bool:
    """True if any element of `rules` is a `Rule`/`RuleDelayed` apply whose
    `lhs` or `rhs` exceeds `max_depth`. A malformed (non-rule) element is
    silently skipped here -- `apply_rule`'s own `is_rule` check raises its
    normal `ValueError` for that element later, when the walk actually
    tries to apply it; this pre-flight check exists only to bound
    recursion depth, never to duplicate rule-shape validation."""
    for candidate in rules:
        if not (isinstance(candidate, IRApply) and len(candidate.args) == 2):
            continue
        lhs, rhs = candidate.args
        if _term_exceeds_depth(lhs, max_depth) or _term_exceeds_depth(rhs, max_depth):
            return True
    return False


# ---------------------------------------------------------------------------
# replace_all -- `expr /. rules`, one pass
# ---------------------------------------------------------------------------


def replace_all(expr: IRNode, rules: Sequence[IRNode]) -> IRNode | DepthLimitError:
    """`expr /. rules` -- Wolfram's `ReplaceAll`, one pass over the whole
    tree.

    Walks `expr` **bottom-up** (post-order: a node's `head` and every
    `args` element are visited -- and possibly replaced -- before the node
    itself is tried against `rules`). This matches
    `cas_pattern_matching.rewrite`'s own traversal order.

    At each subtree (after its children are already finalized), `rules`
    are tried **in order**; the first structural match wins, and its
    substituted replacement takes that subtree's place. Unlike
    :func:`replace_repeated`, each subtree is visited and tried **exactly
    once** -- a freshly-substituted replacement is not re-walked or
    retried against `rules` at the same position, matching Wolfram's `/.`
    (single-pass) contract exactly.

    Always terminates in a single bounded walk over `expr`'s existing node
    count -- there is no fixed-point loop, so (unlike
    :func:`replace_repeated`) there is no `max_iterations` parameter and no
    :class:`RewriteCycleError` outcome. The only failure mode is
    :data:`MAX_TERM_DEPTH` -- checked both against `expr`'s own structure
    (by the walk below) AND, up front, against every rule's `lhs`/`rhs`
    (see the "Rule pre-flight depth check" section above for why a rule
    can drive unbounded recursion independent of `expr`'s depth).

    Example::

        # x_ + 0  ->  x_   applied once, everywhere in the tree
        x_pat = named("x", blank())
        r = rule(apply(sym("Add"), [x_pat, int_(0)]), x_pat)
        expr = apply(sym("Add"), [apply(sym("Add"), [sym("z"), int_(0)]), int_(0)])
        replace_all(expr, [r])  # => sym("z")  (both `+ 0`s fire, one pass each)
    """
    if _rules_exceed_depth(rules, MAX_TERM_DEPTH):
        return DepthLimitError()
    return _walk_once(expr, rules, 0)


def _walk_once(node: IRNode, rules: Sequence[IRNode], depth: int) -> IRNode | DepthLimitError:
    if depth > MAX_TERM_DEPTH:
        return DepthLimitError()

    current: IRNode = node
    if isinstance(node, IRApply):
        new_head = _walk_once(node.head, rules, depth + 1)
        if isinstance(new_head, DepthLimitError):
            return new_head
        new_args: list[IRNode] = []
        for arg in node.args:
            next_arg = _walk_once(arg, rules, depth + 1)
            if isinstance(next_arg, DepthLimitError):
                return next_arg
            new_args.append(next_arg)
        current = IRApply(new_head, tuple(new_args))

    for candidate in rules:
        replacement = apply_rule(candidate, current)
        if replacement is not None:
            return replacement  # first match wins; no retry at this position
    return current


# ---------------------------------------------------------------------------
# replace_repeated -- `expr //. rules`, fixed point
# ---------------------------------------------------------------------------


def replace_repeated(
    expr: IRNode,
    rules: Sequence[IRNode],
    max_iterations: int = _DEFAULT_MAX_ITERATIONS,
) -> IRNode | RewriteCycleError | DepthLimitError:
    """`expr //. rules` -- Wolfram's `ReplaceRepeated`, a fixed point.

    Like :func:`replace_all`, walks bottom-up, but at each subtree keeps
    retrying `rules` until none fire, re-processing any fresh replacement's
    own children (so *its* sub-parts also converge) before moving up to the
    parent. This is `cas_pattern_matching.rewrite`'s algorithm --
    reimplemented here (calling the same `apply_rule` primitive `rewrite`
    itself uses) rather than called directly, because `rewrite` has no
    recursion-depth parameter to hook a cap into (see the module
    docstring's "Depth safety" section). Otherwise the algorithm is
    identical: bottom-up traversal, per-node local fixed point, and a
    **global** `max_iterations` cap shared across the whole walk (not
    per-node) -- the same cap shape `rewrite` uses, raising
    :class:`RewriteCycleError` on the same terms `rewrite` raises its own
    `RewriteCycleError` exception on (this package returns the failure as
    a *value* instead of raising -- see :func:`unwrap`'s docstring for
    why).

    **A gap this implementation deliberately does NOT port from
    `cas_pattern_matching.rewrite`, found by the published TypeScript
    sibling package's own `/security-review`:** `rewrite`'s retry-on-fire
    step is a *recursive* call (``current = walk(replacement)``) -- one
    more native stack frame per firing, chained for as long as retries
    keep happening at one tree position -- so its native recursion depth
    is bounded only by `max_iterations`, not by any tree-depth cap; a
    caller passing a large `max_iterations` (well beyond the "well-behaved
    rules converge in 2-5 passes" norm the default assumes) could exhaust
    the stack through that path alone, independent of `MAX_TERM_DEPTH` and
    independent of how deep or shallow `expr` itself is. The version below
    instead loops **locally** (a plain `while True:`, not a recursive call)
    when a rule fires at the current position -- `depth` only ever
    increases on a genuine descent into `head`/`args`, so `max_iterations`
    now bounds only iteration *count* (CPU time), never native recursion
    depth, however large a value a caller passes. This is the exact same
    fix the TypeScript/JavaScript/Ruby sibling backends already carry.

    Like :func:`replace_all`, `MAX_TERM_DEPTH` is checked both against
    `expr`'s own structure (by `walk` below) AND, up front, against every
    rule's `lhs`/`rhs` (see the "Rule pre-flight depth check" section
    above) -- a rule's own pattern/RHS depth can drive unbounded recursion
    inside `apply_rule` independent of how deep `expr` is.

    Example::

        # x_ + 0  ->  x_   applied repeatedly, to a fixed point
        x_pat = named("x", blank())
        r = rule(apply(sym("Add"), [x_pat, int_(0)]), x_pat)
        expr = apply(sym("Add"), [apply(sym("Add"), [sym("z"), int_(0)]), int_(0)])
        replace_repeated(expr, [r], 100)  # => sym("z")
    """
    if _rules_exceed_depth(rules, MAX_TERM_DEPTH):
        return DepthLimitError()

    counter = 0

    def walk(node: IRNode, depth: int) -> IRNode | RewriteCycleError | DepthLimitError:
        nonlocal counter
        if depth > MAX_TERM_DEPTH:
            return DepthLimitError()

        current: IRNode = node
        # Outer loop: each pass (re)processes `current`'s children
        # bottom-up, then tries firing a rule at this position. A fired
        # rule's replacement becomes the new `current` and the loop
        # repeats -- ITERATIVELY, at this SAME call frame, not via a
        # recursive call -- so however many times a rule fires at one tree
        # position, that costs O(1) native stack frames, not O(firings).
        # `depth` is unchanged across iterations of this loop (nothing
        # here descends to a child); it only increases via the genuine
        # `head`/`args` recursion below.
        while True:
            if isinstance(current, IRApply):
                new_head_result = walk(current.head, depth + 1)
                if isinstance(new_head_result, (DepthLimitError, RewriteCycleError)):
                    return new_head_result
                new_head: IRNode = new_head_result
                new_args: list[IRNode] = []
                for arg in current.args:
                    next_arg_result = walk(arg, depth + 1)
                    if isinstance(next_arg_result, (DepthLimitError, RewriteCycleError)):
                        return next_arg_result
                    new_args.append(next_arg_result)
                current = IRApply(new_head, tuple(new_args))

            fired = False
            for candidate in rules:
                replacement = apply_rule(candidate, current)
                if replacement is not None and replacement != current:
                    counter += 1
                    if counter > max_iterations:
                        return RewriteCycleError(max_iterations=max_iterations)
                    current = replacement
                    fired = True
                    break
            if not fired:
                return current
            # Loop again: `current` (the fresh replacement) may itself
            # contain sub-structure that hasn't been processed yet, so it
            # goes through the same "process children, then try rules"
            # pass before this position is considered stable -- matching
            # `rewrite`'s own "re-walk the replacement" behavior, just
            # without recursing to do it.

    return walk(expr, 0)
