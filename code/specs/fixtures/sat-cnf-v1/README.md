# SAT CNF v1 shared stateful corpus

`cases.json` freezes valid DIMACS inputs and ordered public solver operations
for both the standalone TypeScript and native Go `sat-solver` packages. A
consumer must pin the SHA-256 and exact case-ID set, run each operation against
the public API, independently evaluate every SAT model, and brute-force check
each failed core against the original permanent formula. Returned core order
and minimality are intentionally unspecified.

The corpus exercises valid transport variants, assumption outcomes, clause
addition, reset, budget exhaustion, and recovery. Malformed DIMACS is not a
shared valid case: each package must test documented refusals and record
intentional differences separately. JSON schema validation alone is not
conformance.
