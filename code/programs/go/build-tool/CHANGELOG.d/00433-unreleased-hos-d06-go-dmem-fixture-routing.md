## Unreleased — route Go D-MEM fixture inputs precisely

The Go RISC-V simulator now hashes the D-MEM input, independent oracle, captured
trace, external adapter, and the referenced legacy D05 input and trace. The
existing D05 inputs remain unchanged and are not duplicated in the package hash.
Fixture schemas, README files, and lookalike paths do not select the package.
