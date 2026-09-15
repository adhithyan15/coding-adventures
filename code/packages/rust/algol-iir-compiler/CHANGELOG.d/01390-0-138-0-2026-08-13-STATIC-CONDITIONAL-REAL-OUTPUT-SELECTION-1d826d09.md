## 0.138.0 — 2026-08-13 — static conditional real output selection

Formatter-free real output now evaluates only the selected branch when a
conditional expression's selector is statically known. Dynamic selectors still
require both branches to be statically printable.

