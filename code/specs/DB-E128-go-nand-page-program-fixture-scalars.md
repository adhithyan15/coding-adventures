# DB-E128 — Validate escaped Unicode in the Go NAND page-program fixture

The Go fixture adapter validates raw UTF-8 before reading JSON tokens.
The JSON decoder converts an isolated UTF-16 surrogate escape to U+FFFD.
In a disposable copy, substituting either `\ud800` or `\udc00` for the
first case name allowed the entire fixture to be accepted and replayed.
The C bounded JSON adapter requires well-formed scalar escapes.
[Interrupted-program fixture](DB04b1-nand-page-program-cuts-go.md#shared-fixture)

Add the same bounded escape admission used by the Go HDD and NAND
fixture adapters before decoding any fixture tokens. Preserve the
byte, case-count and nesting limits. A high surrogate escape inside
a string requires an immediately following low surrogate escape.
Reject any isolated low surrogate, reversed pair or incomplete
Unicode escape through the existing malformed-fixture result.

Admit a valid surrogate pair, literal U+FFFD, an explicit `\ufffd`
escape, and the escaped backslash in text such as `"\\ud800"`.
Use model-first admission for all canonical declarations. Add a
named negative control that admits both isolated escapes when the
new check is bypassed. The runtime and shared vector retain their
existing exact bytes.

Run both package command lists, the fixture/parser fuzz target,
coverage and the exact emitted recipe. Keep the runtime dependency
graph and fixture routing stable.

## Local verification

The reviewed adapter in `9205450e89f` has SHA-256
`ced80662f62ac05f8ebd18c0ec234507124f19593156aa9658b0143717c9a239`.
An independent caller checked both ends of the surrogate ranges,
paired values, literal and escaped replacement characters, and escaped
backslashes. Five malformed variants and a structurally valid wrong
final declaration caused zero runtime calls. Five well-formed variants
each replayed all 18 declarations.

Both build command lists passed on Linux with race, vet and 100%
runtime statement coverage. The fixture fuzz target completed
100,000 executions. The scalar-validation bypass was detected at the
named negative controls. The emitted recipe passed from a path with
spaces, with the same runtime and canonical vector.
