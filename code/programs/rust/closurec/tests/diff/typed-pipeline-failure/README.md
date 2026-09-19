# Typed-pipeline failure policy

CCR-002 makes SIMPLE and ADVANCED compilation fail closed when the requested
typed pipeline cannot finish. The fixture covers two front-end boundaries:

- `malformed.js` fails in the grammar parser. Google Closure Compiler
  `v20260915` exits 1 with `JSC_PARSE_ERROR` and emits no JavaScript for the
  same source.
- `destructuring.js` is valid and compiles upstream, but closurec's typed AST
  does not yet represent binding patterns. Until CCR-010 closes that language
  gap, closurec reports the bridge stage and rule instead of silently returning
  WHITESPACE_ONLY output under a successful SIMPLE or ADVANCED invocation.

The oracle jar SHA-256 is
`9C8AF06056AA06F968B5A457540A85869C7BA2861C211C56D8D4EF6C35DDF36D`.
`diff_typed_pipeline_failure.rs` runs both inputs at both optimization levels
and verifies exit status, stderr, empty stdout, and absence of an output file.
