# starlark-compiler

Go opcode metadata for the Coding Adventures Starlark bytecode compiler.

This package mirrors the Rust `starlark-compiler` opcode contract, including
stable byte values, byte-to-opcode conversion, high-nibble categories, and
operator maps used by compiler front ends.


## Shared opcode-provider conformance

The native tests consume all 359 records in the language-neutral
[STP01 corpus](../../../specs/fixtures/starlark-opcode-provider-v1/cases.json):
61 assigned opcodes, all 256 decoder bytes (195 holes), 34 exact operator
mappings and eight unknown lookups. They compare the actual public enumeration,
byte/name/category methods, map constructors and direct helpers. Independently
copied ownership baselines test two held results and fresh results after mutation.

The test-only standard-library reader rejects malformed transport before byte
narrowing: bounded UTF-8/scalar JSON, duplicate decoded keys, trailing content,
missing/null/type confusion, noninteger/out-of-range bytes and closed table
shape. Semantic corruption is checked against actual APIs; the separate neutral
validator retains its independent literal oracle. No runtime parser dependency,
Starlark execution, module loading or host authority is introduced.

Run `go test ./... -v -cover`, `go vet ./...` and `go build ./...` from this
package. Both BUILD fronts run the native tests. An exact shared cases.json edit
also schedules this Go reader in CI, without claiming other lanes are adopters.
No current Go package imports this provider; AST emission and VM qualification
remain separate contracts.
