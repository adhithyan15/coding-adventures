## 0.353.0 - Runtime real scalar copies

Straight-line bare copies between local ALGOL real scalars now preserve the
provenance required by the shared portable formatter. Arithmetic, unary
wrappers, globals, calls, and control-flow barriers remain conservative.
