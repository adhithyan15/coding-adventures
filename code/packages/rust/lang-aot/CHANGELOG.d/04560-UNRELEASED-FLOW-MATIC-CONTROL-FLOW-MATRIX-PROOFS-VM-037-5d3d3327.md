## Unreleased — FLOW-MATIC control-flow matrix proofs (VM-037)

Add seven-backend canonical programs for taken EQUAL, false LESS/GREATER
falling to OTHERWISE, and a two-jump chain. Wrong paths terminate with different
record widths or line counts, so a miscompile fails by output rather than a
hang. The normal non-ALGOL matrix gate automatically includes all three rows.

