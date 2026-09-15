## Unreleased — COBOL pointer/overflow proofs (VM-046c)

Add eight seven-backend STRING/UNSTRING programs that jointly observe text,
pointer writeback and overflow/success branches. Cover in-range offsets, exact
fit, partial transfer, invalid starting pointers, exhausted input and a trailing
delimiter. Bracketed output preserves spaces; normal non-ALGOL BUILD includes
all new rows automatically.

