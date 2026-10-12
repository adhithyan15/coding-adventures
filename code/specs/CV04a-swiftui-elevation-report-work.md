# CV04a: bounded SwiftUI elevation reporting

Independent review withheld imported main033/b0e794b9: valid flat authored
`style Card { part root {` followed by N `box-shadow: none;` declarations and
optional final `elevation: raised;` causes the public artifact reporter to scan
all base properties once per dropped shadow. The new elevation predicate makes
N-squared visits without elevation, N*(N+1) with a supported final declaration.
This is source-derived work amplification, not a timing benchmark or UB claim.

Before repair add a private callback observer around the existing predicate;
public API invokes it with a no-op, preserving the old per-shadow scan for the
red. Real mosstyle compilation must preserve the complete duplicate declarations.
Assert exact elevation predicate visits N or N+1 at N32 and128 for no/late
elevation, retaining exact report semantics and order. Use direct public IR for
unsupported elevation because the authored compiler rejects that enum value.
Include inset/standalone/other-drop ordering controls and multiple parts.
Save actual failing work-count receipts before changing the scan location.

Repair by computing the exact existing supported-elevation `any` predicate once
per expanded base part, before the drops loop, and reuse that Boolean. Preserve
the predicate, expansion, emitter/report source sharing, returned fields/order,
gap/align/per-edge filtering and supported/inset/standalone semantics. No new
public API, allocation, dependency, resource exemption or parser limit change.
The observer is a private generic no-op callback in production; tests count real
visits rather than elapsed time, fake formula output or token presence. The bound
concerns this elevation lookup, not unrelated legacy reporter work.

Run the entire affected six-package SwiftUI reverse cone, TaskApp Rust package,
strict all-target lint, native-control Python suite/Ruff, docs/lessons/whitespace.
Use a distinct fresh Venture profile and preserve the shared profile hash.
Retain native66 diagram/Arc821/37 plus compiler48 receipts only through exact
source/input identities; preserve b0e native634/30 execution-only and withheld
review separately. Final source/payload proof, independent exact-final security,
current-main audit and fresh required hosted checks gate samePR delivery/merge.

Full final-source Miri **DEFERRED/PENDING**, frozen checkout/497inputs/receipts/
target untouched without polling/termination/duplicate runs. No native device/
deployment/full Closure parity/lineage completion claim; base ESTree/owned types/
no new crates.io dependencies remain.
