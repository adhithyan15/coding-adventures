# Fixture completeness does not guarantee TypeScript branch coverage

The TypeScript DER ASN.1 suite executed all 109 neutral cases and all tests
passed, but V8 coverage still reported only 91.11 percent statements and 83.18
percent branches. The closed fixture does not exercise defensive runtime-token
guards, non-DER exception rethrows, or every structural-equality branch. Keep
the strong coverage threshold and add focused native adversarial tests for
those implementation-specific paths instead of weakening the gate.
