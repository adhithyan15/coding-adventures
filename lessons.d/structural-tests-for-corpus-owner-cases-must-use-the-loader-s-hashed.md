---
category: Testing & coverage
---

# Structural tests for corpus owner cases must use the loader's hashed filename grammar

The first HL47 ownership test named the planned real-corpus shards with plain
descriptive `*.case.ts` names. That contradicted the corpus loader's existing
`<lowercase-kebab-title>-<eight-lowercase-hex>.case.ts` contract before the
files even existed. A structural test is part of the design: if its expected
owner names violate discovery, implementation cannot satisfy both gates.

Derive the suffixes before writing the ownership expectation, and keep the
exact discoverable filenames in that test. Run the actual loader entry point
after the split so discovery, not a direct file invocation, proves the names.

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.
