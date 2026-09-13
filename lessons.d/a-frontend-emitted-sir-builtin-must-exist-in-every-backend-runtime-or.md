# A frontend-emitted SIR builtin must exist in EVERY backend runtime, or `case` panics at runtime

Driving a real Ruby `case`/`when` program through the SIR pipeline to all
backends revealed that `case`/`when` (and `case`/`in`) **panicked at runtime on
Go, Rust, AND JavaScript** with `unknown builtin: case_eq` — it only worked on
Python/TypeScript. Root cause: the Ruby→SIR frontend lowers every `when` arm to
`BuiltinCall("case_eq", [pattern, scrutinee])` (Ruby's `===`), but three of the
five backend runtimes never implemented that builtin. Because an arbitrary
`BuiltinCall` name is *open-ended* (backends fall through to a
`call_builtin_by_name` dispatcher whose floor is `panic!("unknown builtin")`),
there is **no compile-time gate** — the feature manifest can't reject it, so the
gap only surfaces when the emitted program actually runs.

Why it went unnoticed: the per-backend `compile_and_run_*` tests **hand-build
SIR modules** exercising one feature each; none had ever hand-built a `case_eq`
call, and no single test drove real Ruby *source* → Go/Rust/JS end-to-end. The
Python/TS backends implement `case_eq` via their runtime *packages*, so those
arms passed and masked the gap on the inline-runtime backends.

**Lessons:**
- When the frontend starts emitting a new builtin name, grep EVERY backend
  runtime (`semantic-ir-to-{go,rust,javascript}` inline runtimes + the
  `sir-runtime-*` packages) for it before assuming cross-backend support.
- A per-backend exec test that hand-builds SIR proves only the feature it builds;
  it cannot catch a builtin the frontend emits but the test never constructs.
  A Ruby-source → all-5-backends golden suite is the real guard (still a TODO).
- `case_eq` = Ruby `===`, keyed to the *pattern* type (Range→membership,
  Regexp→match, else `==`); `when SomeClass` is lowered to `.is_a?` at the
  frontend, so class patterns never reach `case_eq`. Backends with no
  Range/Regexp value type implement it as plain structural equality.
