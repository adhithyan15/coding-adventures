# Changelog

## 0.2.2 - 2026-06-06

- Add bounded-over-diverging limit recognition at infinity, closing
  `limit(sin(x)/x, x, inf)` and `limit(cos(x)/(x^2+1), x, minf)` to exact
  `0` instead of returning an unevaluated `Limit(...)`.

## 0.1.0

- Add pure TypeScript parity for Rust `cas-limit-series` direct limits and
  polynomial Taylor expansion.
