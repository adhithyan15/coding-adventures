# 2026-09-05 — Literal facts need program-point validity

A function-wide map keyed by mutable string register cannot represent two
successive literal assignments, even without branches. Printing between writes
must observe the earlier value. Treat multiply written variables as runtime
handles and propagate their representation; retain a sequential output test,
not only tests that inspect the final receiver value.
