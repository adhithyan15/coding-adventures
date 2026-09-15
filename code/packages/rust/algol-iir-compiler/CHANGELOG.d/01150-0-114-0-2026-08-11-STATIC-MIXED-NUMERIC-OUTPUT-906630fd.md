## 0.114.0 — 2026-08-11 — static mixed numeric output

Integer literals within binary64's exact integer range may now widen into the
bounded static real evaluator, including assignments to tracked real locals and
mixed arithmetic output. Larger integer literals fail closed instead of losing
precision during compile-time conversion.

