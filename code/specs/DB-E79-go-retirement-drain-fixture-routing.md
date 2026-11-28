# DB-E79 — Route the readable NAND drain fixture

Use verified DB05d2b1 at `eaa17bc2e4f`. The planner consumes
`code/specs/fixtures/database-media-v1/ftl-retirement-drain-plan.json`
at SHA-256 `1e88493c5d169659197064c204ee4e7221899a0a794b490683f8f06404d60d58`
alongside its established `ftl-evacuation-plan.json` input. The new
vector has three declarations, including complete occupancy of the
readable source and both valid and refused spare capacity.

After assembling the verified planner and its existing route, add the
new exact file to `repository-database-media-ftl-evacuation-plan-fixture-inputs`.
Keep its one exact root. The tree-to-fixture association applies to the
file only. An edit or deletion selects the Go planner and ordinary
source dependents. It affects the planner's direct fallback key and
appears next to the original vector in its Kiln inputs. The downstream
retirement ledger tests compose the actual APIs and get changes through
the planner dependency. They read their own retirement and factory
vectors through [DB-E77](DB-E77-ftl-retirement-fixture-routing.md).

Check direct cache isolation for the adjacent FTL fixtures and similar
names. Regenerate the shared boundary digest and both projections. The
preceding registry gains exactly one input and one distinct path. Run
Go, C#, Swift, Python, corpus and the real graph checks against the
result. Both native planner and downstream ledger suites must pass
with exact shared vectors, and the actual Go manifest graph must
place the planner before its budget and retirement consumers.
