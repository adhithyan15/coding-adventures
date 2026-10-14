# An opcode-presence oracle can preserve wrong unordered comparison semantics

The existing f64_greater_uses_dcmpg test demanded dcmpg for greater-than even
though unordered then compares high and incorrectly returns true. The real
JVM IEEE table checks all six relations with NaN on either side; it passes
with dcmpl plus ifle, keeping unordered greater-than false. Preserve the old
full-suite failure and update its byte assertion to that exact opcode/branch
pair. Do not delete the test or accept opcode presence as a semantic proof.
