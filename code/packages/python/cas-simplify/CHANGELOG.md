# Changelog

## 0.3.0 — 2026-05-04

**Phase 21 — assumption framework, radcan, logcontract/logexpand, exponentialize/demoivre.**

### New modules

- **`assumptions.py`** — `AssumptionContext`: per-VM mutable store of
  per-symbol facts (positive, negative, zero, nonzero, nonneg, nonpos,
  integer).  Methods: `assume_relation`, `assume_property`,
  `forget_relation`, `forget_all`, `is_positive`, `is_negative`,
  `is_nonneg`, `is_integer`, `sign_of`, `is_true_relation`,
  `has_any_facts`.

- **`radcan.py`** — `radcan(expr, ctx=None)`: radical canonicalization.
  Rules: Sqrt product merge, perfect-square extraction, common rational
  exponent collection, Pow(Sqrt(x),2)→x, Exp(Log(x))→x / Log(Exp(x))→x.

- **`logcontract.py`** — `logcontract(expr)` and `logexpand(expr, ctx=None)`.
  Contract: log(a)+log(b)→log(ab), n*log(a)→log(a^n), log(a)-log(b)→log(a/b).
  Expand: log(a^n)→n*log(a), log(ab)→log(a)+log(b), log(a/b)→log(a)-log(b).

- **`exponentialize.py`** — `exponentialize(expr)` and `demoivre(expr)`.
  Exponentialize: sin/cos/tan/sinh/cosh/tanh → exp form.
  DeMoivre: exp(a+bi) → exp(a)·(cos b + i·sin b).

### Updated modules

- **`heads.py`** — 9 new `IRSymbol` sentinels: `ASSUME`, `FORGET`, `IS`,
  `SIGN`, `RADCAN`, `LOGCONTRACT`, `LOGEXPAND`, `EXPONENTIALIZE`, `DEMOIVRE`.

- **`__init__.py`** — exports all new public symbols.

### Version bump

`symbolic-ir` requirement unchanged (`>=0.5.0`; new heads added there
in 0.9.0 are not imported by this library directly).

---

## 0.1.0 — 2026-04-25

Initial release — Phase C foundation.

- ``canonical(expr)`` — flatten, sort, drop singletons, fold empty
  containers.
- ``simplify(expr)`` — fixed-point loop: canonical + numeric-fold +
  identity-rule application.
- Identity rule database (Add/Mul/Pow/Sub/Div + a starter set of
  elementary-function identities like ``Log(Exp(x)) → x``).
- ``Simplify`` and ``Canonical`` IR head sentinels for backends to
  install handlers against.
- Type-checked, ruff- and mypy-clean.

Deferred: ``Expand``, ``Collect``, ``Together``, ``Apart``,
``cas-trig-simplify``.
