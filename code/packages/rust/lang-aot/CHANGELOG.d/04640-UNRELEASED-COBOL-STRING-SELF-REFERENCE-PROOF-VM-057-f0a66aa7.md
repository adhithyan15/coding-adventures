## Unreleased — COBOL STRING self-reference proof (VM-057)

Add a seven-backend cell for `STRING <item> DELIMITED BY SIZE INTO <same
item>`: a lone sending field that is also its own `INTO` receiver, the real
COBOL shape that reaches WASM `str_slice`'s destination-aliases-source path
with no intermediate temporary. Expects the receiver unchanged. This
protects VM-057: WASM `str_slice` preserves the aliased source handle.


