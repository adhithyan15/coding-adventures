## Unreleased — COBOL reference-modification proofs (VM-045)

Add seven canonical rows for literal/computed substring bounds, omitted length,
IF/EVALUATE, MOVE padding/truncation, and invalid runtime start/end traps.
Trailing markers preserve padding evidence. All rows declare seven standard
backends and are included automatically by normal non-ALGOL BUILD. The LLVM
runner links the production runtime for checked substring helpers. The new
cells exposed and now protect native/LLVM computed-slice lowering and WASM
runtime-string propagation repairs (VM-050–052).

