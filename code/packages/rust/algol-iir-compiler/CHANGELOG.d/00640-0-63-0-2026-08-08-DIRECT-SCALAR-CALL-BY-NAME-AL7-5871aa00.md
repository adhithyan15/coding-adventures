## 0.63.0 — 2026-08-08 — direct scalar call-by-name (AL7)

Direct calls can now use `integer`, `real`, and `boolean` name formals. Each
call emits a specialised typed sibling function whose formal reads re-evaluate
the caller expression and whose writes target an assignable caller variable or
array element. Regression coverage includes state-sensitive re-evaluation, a
Jensen-style sum, real and boolean formals, and rejection of a written literal
actual.

