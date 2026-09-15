## 0.74.0 — 2026-08-09 — nested capturing direct procedure actuals

Regression coverage now proves that a direct nested procedure may capture an
enclosing value formal and still travel through a `procedure` formal. The
specialised wrapper calls the nested sibling directly while the existing
capture substrate supplies its outer value, without a closure descriptor.

