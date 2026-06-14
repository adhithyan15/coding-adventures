# Phase 22 — MACSYMA Pattern Matching: matchdeclare / defrule / apply1 / apply2 / tellsimp

## Context

Phase 21 (merged as PR #2035) completed the simplification suite: `assume`,
`forget`, `is`, `sign`, `radcan`, `logcontract`/`logexpand`,
`exponentialize`/`demoivre`.

Phase 22 adds MACSYMA's user-defined rewrite-rule system on top of the
existing `cas-pattern-matching` package (which already provides `Blank`,
`Pattern`, `Rule`, `match`, `apply_rule`, and `rewrite`).  The five new
operations let users declare pattern variables, compile named rules, and
apply them at the REPL.

---

## MACSYMA Surface Syntax

```
matchdeclare(x, true)$               /* x matches anything */
matchdeclare(n, integerp)$           /* n matches only integers */
matchdeclare(a, symbolp)$            /* a matches only symbols */

defrule(r1, sin(x)^2 + cos(x)^2, 1)$
apply2(r1, sin(t)^2 + cos(t)^2 + 3);
/* → 4 */

tellsimp(sin(x)^2 + cos(x)^2, 1)$
sin(u)^2 + cos(u)^2;
/* → 1  (fires automatically via the simplifier) */
```

---

## New IR Heads (symbolic-ir → 0.10.0)

| Python constant | IR symbol     | MACSYMA keyword  | Arity |
|-----------------|---------------|------------------|-------|
| `MATCHDECLARE`  | `MatchDeclare`| `matchdeclare`   | 1–2   |
| `DEFRULE`       | `Defrule`     | `defrule`        | 3     |
| `APPLY1`        | `Apply1`      | `apply1`         | 2     |
| `APPLY2`        | `Apply2`      | `apply2`         | 2     |
| `TELLSIMP`      | `TellSimp`    | `tellsimp`       | 2     |

All five heads are **held** in the VM (arguments not pre-evaluated) so
pattern expressions and rule names pass through as IR symbols rather than
being looked up in the environment.  `apply1` and `apply2` handlers
manually evaluate the target expression after extracting the rule name.

---

## Architecture

### New modules in `cas-pattern-matching` (→ 0.2.0)

#### `matchdeclare.py` — `MatchDeclareContext`

Per-VM mutable store mapping symbol names to predicate tags.  Also
provides `compile_pattern(pattern)` which walks an IR tree and substitutes
each declared-variable `IRSymbol` with a `Pattern(name, Blank(constraint))`
node ready for the existing `matcher.match` engine.

```
Predicate tag   → Blank constraint
────────────────────────────────────
"true" / "any"  → Blank()           (unconstrained)
"integerp"      → Blank("Integer")
"symbolp"       → Blank("Symbol")
"floatp"        → Blank("Float")
"rationalp"     → Blank("Rational")
"numberp"       → Blank()           (union — unconstrained fallback)
"listp"         → Blank("List")
"stringp"       → Blank("String")
(unknown)       → Blank()           (safe fallback)
```

#### `defrule_engine.py` — `RuleStore`

Per-VM mutable store mapping rule names (Python `str`) to compiled
`IRApply(Rule, (compiled_lhs, rhs))` nodes.  Wraps a plain `dict`.

### Changes to `symbolic-vm` (→ 0.42.0)

#### `vm.py`

Three new attributes on `VM.__init__`:

```python
self.match_declarations: MatchDeclareContext = MatchDeclareContext()
self.named_rules: RuleStore = RuleStore()
self.tellsimp_rules: list[IRApply] = []
```

`_eval_apply` gains a step **2b** — after backend rules, before handler
dispatch — that tries each rule in `vm.tellsimp_rules`:

```python
# 2b. User-declared tellsimp rules (Phase 22).
for rule in self.tellsimp_rules:
    result = _pm_apply_rule(rule, expr)
    if result is not None:
        return self.eval(result)
```

#### `backends.py`

The five new head names are added to `_HELD_HEADS`:

```python
"MatchDeclare", "Defrule", "Apply1", "Apply2", "TellSimp"
```

#### `cas_handlers.py`

Five new handler functions; all added to `build_cas_handler_table()`:

| Handler | Key |
|---------|-----|
| `matchdeclare_handler` | `"MatchDeclare"` |
| `defrule_handler` | `"Defrule"` |
| `apply1_handler` | `"Apply1"` |
| `apply2_handler` | `"Apply2"` |
| `tellsimp_handler` | `"TellSimp"` |

---

## Handler Semantics

### `matchdeclare_handler`

```
MatchDeclare(sym)           → declares sym with predicate "any" (true)
MatchDeclare(sym, predicate) → declares sym with predicate.name.lower()
```
Returns `IRSymbol("done")`.

### `defrule_handler`

```
Defrule(name, lhs, rhs)
```
1. Calls `vm.match_declarations.compile_pattern(lhs)` to produce a
   matcher-ready LHS (declared variables → Pattern nodes).
2. Constructs `Rule(compiled_lhs, rhs)` (from `cas_pattern_matching.nodes`).
3. Stores it in `vm.named_rules` under `name.name`.
4. Returns `name` (the rule-name symbol).

### `apply1_handler`

```
Apply1(name, expr)
```
1. Evaluates `expr` via `vm.eval(expr.args[1])`.
2. Looks up the compiled rule for `name` from `vm.named_rules`.
3. Calls `apply_rule(rule, target)` — root-only application.
4. Returns `vm.eval(result)` on match, or the evaluated `target` if no match.

### `apply2_handler`

```
Apply2(name, expr)
```
Same as `apply1` but uses `rewrite(target, [rule])` for a full bottom-up
fixed-point traversal.

### `tellsimp_handler`

```
TellSimp(lhs, rhs)
```
1. Compiles `lhs` via `vm.match_declarations.compile_pattern(lhs)`.
2. Constructs `Rule(compiled_lhs, rhs)`.
3. Appends it to `vm.tellsimp_rules`.
4. Returns `IRSymbol("done")`.

---

## Files Changed

| File | Change |
|------|--------|
| `code/specs/phase22-pattern-matching.md` | **NEW** (this file) |
| `symbolic-ir/src/symbolic_ir/nodes.py` | +5 heads |
| `symbolic-ir/src/symbolic_ir/__init__.py` | export 5 heads |
| `symbolic-ir/pyproject.toml` | 0.10.0 |
| `symbolic-ir/CHANGELOG.md` | 0.10.0 entry |
| `cas-pattern-matching/src/cas_pattern_matching/matchdeclare.py` | **NEW** |
| `cas-pattern-matching/src/cas_pattern_matching/defrule_engine.py` | **NEW** |
| `cas-pattern-matching/src/cas_pattern_matching/__init__.py` | export new items |
| `cas-pattern-matching/pyproject.toml` | 0.2.0 |
| `cas-pattern-matching/CHANGELOG.md` | 0.2.0 entry |
| `cas-pattern-matching/tests/test_phase22.py` | **NEW** ≥24 tests |
| `symbolic-vm/src/symbolic_vm/vm.py` | 3 new attrs + step 2b |
| `symbolic-vm/src/symbolic_vm/backends.py` | +5 held heads |
| `symbolic-vm/src/symbolic_vm/cas_handlers.py` | 5 new handlers |
| `symbolic-vm/pyproject.toml` | 0.42.0, cas-pattern-matching>=0.2.0 |
| `symbolic-vm/CHANGELOG.md` | 0.42.0 entry |
| `symbolic-vm/tests/test_phase22.py` | **NEW** ≥20 tests |

---

## Verification

```bash
# cas-pattern-matching
cd code/packages/python/cas-pattern-matching
.venv/bin/pytest tests/ -q   # ≥80% coverage

# symbolic-vm
cd code/packages/python/symbolic-vm
.venv/bin/pytest tests/ -q   # ≥80% coverage, ~1130+ tests pass
```

Spot-checks:
```python
# matchdeclare + defrule + apply2
matchdeclare(x, true)
defrule(r1, sin(x)^2 + cos(x)^2, 1)
apply2(r1, sin(t)^2 + cos(t)^2 + 3)    # → 4

# tellsimp fires automatically in simplifier
matchdeclare(x, true)
tellsimp(sin(x)^2 + cos(x)^2, 1)
sin(u)^2 + cos(u)^2                     # → 1

# integerp predicate
matchdeclare(n, integerp)
defrule(double, n + n, 2*n)
apply1(double, 3 + 3)                   # → 6
apply1(double, x + x)                   # → x+x (no match — x not integer)
```
