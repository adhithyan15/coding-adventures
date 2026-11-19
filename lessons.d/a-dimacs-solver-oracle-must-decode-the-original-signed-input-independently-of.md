---
category: Testing & coverage
---

# A DIMACS solver oracle must decode the original signed input independently of the production parser

A shared-fixture test first parsed DIMACS with the production parser and then
used those parsed literals as the "original" clauses for model and failed-core
checking. A parser error could therefore change both the solver input and the
oracle input in the same way. The TypeScript and Go consumers now decode the
fixture's signed DIMACS integers separately, compare production parser output
against that decode, and replay models/cores against the independent signed
formula. For transport-backed solver evidence, retain the input bytes as the
oracle source; do not promote a production parser's result into its own
independent reference.
