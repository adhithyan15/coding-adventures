# @coding-adventures/cas-laplace

Pure TypeScript Laplace transform support for the symbolic IR stack.

The implementation is browser-safe and does not depend on Python, Node-only
APIs, or generated parser code. It ports the Python `cas-laplace` table layer
as direct IR-to-IR operations.

## Exports

- `laplaceTransform(f, t, s)`
- `inverseLaplace(f, s, t)`
- `laplaceHandler`, `iltHandler`, `diracDeltaHandler`, `unitStepHandler`
- `LAPLACE`, `ILT`, `DIRAC_DELTA`, `UNIT_STEP`

Unknown forms fall through to unevaluated IR nodes.
