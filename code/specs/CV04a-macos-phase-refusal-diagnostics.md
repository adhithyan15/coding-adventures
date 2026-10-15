# CV04a: retain bounded diagnostics for a failed macOS phase probe

Specification before tests or implementation. Actual published8f02dcc0 macOS
job116607025275 passed all12 portable bootstrap tests, then production bootstrap
refused its cabal-refuse-pgmc result. The public job log contains the traceback
and a runner-local log path, but not the captured Cabal/GHC diagnostic. Raw log
SHAae97853624f71915d915504414340f3e8b077e31d75f8fc700e31acf8b9e4d5e
is preserved. The configured compiler-phase refusal is not proven, and the root
cause cannot be inferred from the final assertion alone. No hosted pass claimed.

Preserve every command/flag, compiler/hsc2hs route, exact refusal identity check,
runtime42 assertion, original config and publish-only-after-success behavior.
Do not accept a path mentioned in arbitrary text, a prefix lookalike, ignored
negative control, unrelated nonzero failure or missing/wrong hsc2hs trace.

After tests-first red, append diagnostic_tail=<JSON string> to errors from a
failed child command or unproven phase refusal. The JSON value is at most the
last4096 characters of that child's captured stdout+stderr, including an empty
tail when output is empty. JSON quoting keeps physical newlines/control bytes
out of the exception message and prevents text from creating workflow command
lines. Keep the complete existing log and its SHA receipt. Do not dump environment
variables, the original Cabal config, other files, or unrelated successful logs.

Portable tests must exercise actual bootstrap failure paths, verify exact decoded
tails, byte/control escaping and truncation, original log retention and unchanged
GitHub environment/no success publication. Retain all12 original tests and their
exact compiler/refusal/signal/wrapper controls. Pinned Python3.13 Ruff0.15.20,
unittest and diff checks are required. This repairs diagnostic visibility only;
the actual macOS compiler-phase failure remains OPEN until the real selected
route, all three exact negative probes and positive link/run checks pass.

Work in a separate managed checkout while R1 native validation is frozen; no
Rust source or Cargo target mutation. No new PR. Independent security review
must accept the exact source/carrier before normal same-PR delivery. Final-source
Miri DEFERRED/PENDING, never passed; frozen checkout/process/inputs/target/receipts
remain untouched and unpolled. Full Closure parity/provenance objective OPEN.
