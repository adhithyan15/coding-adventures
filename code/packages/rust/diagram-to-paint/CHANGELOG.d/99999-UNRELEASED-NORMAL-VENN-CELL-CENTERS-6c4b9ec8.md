## Unreleased — normal Venn text grid transport

Attached Venn text uses row-major cell centers from one shared normal/debug
grid. Exact ordered structural memberships retain authored group/text order,
including repeated/reversed/NUL members and the existing unknown-member policy.
First-radius/last-center lookup and nonempty region-label OR remain distinct.
Checked numeric admission, complete debug geometry, styles and fonts stay intact;
the renderer, dependencies and production count limits do not change.

At source `6c4b9ec8`, all261 tests across17 harnesses pass without failures,
ignores or filters; two empty doctests and both all-target strict checks pass.
This includes32 whole source/IR/layout/Scene/query cases and32 direct controls;
the old136 source/96 direct complete Scene cases retain every assertion with
only independently declared normal coordinates revised. Nine example targets
were checked on Windows; no Apple rendering execution is claimed.

See [the contract and execution checkpoint](../../../../specs/CV04b-current-main-venn-normal-cell-centers.md).
Current-main/hosted/merge gates and full Closure parity/lineage remain OPEN.
Miri **DEFERRED/PENDING**, never passed; the frozen run and its inputs remain
untouched pending genuine final-source validation after implementation delivery.
