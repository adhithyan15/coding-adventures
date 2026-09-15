## Unreleased — COBOL delimiter proofs (VM-046b)

Add five seven-backend rows for STRING first/absent/leading delimiters and
UNSTRING field fitting, dropped excess fields, leading/consecutive empty fields
and untouched receivers after exhaustion. Exercise literal and item delimiters
with bracketed output preserving spaces. Normal non-ALGOL BUILD includes the
new rows automatically; pointer/overflow remains VM-046c. The cases exposed
and now protect native loop-index folding and LLVM runtime-index repairs
(VM-054/055); the LLVM runner links the production checked indexing helper.

