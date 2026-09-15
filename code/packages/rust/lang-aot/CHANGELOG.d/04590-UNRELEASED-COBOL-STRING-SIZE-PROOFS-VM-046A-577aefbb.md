## Unreleased — COBOL STRING SIZE proofs (VM-046a)

Add three seven-backend rows for full-width sending fields with mixed literal
input, truncated/exact-fit receivers and preserved nonblank tails after source
reassignment. Visible markers keep spaces observable. Normal non-ALGOL BUILD
includes these rows automatically; delimiter and pointer/overflow proofs remain
separate backlog slices. The repeated-write row protects the WASM same-block
string reassignment repair (VM-053), where the first print incorrectly used
the later literal.

