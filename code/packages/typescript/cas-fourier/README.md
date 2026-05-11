# @coding-adventures/cas-fourier

Pure TypeScript Fourier transform support for the symbolic IR stack.

This package ports the Python `cas-fourier` table layer into browser-safe
TypeScript. It works directly over symbolic IR nodes and has no Python or
parser dependency.

## Exports

- `fourierTransform(f, t, omega)`
- `ifourierTransform(f, omega, t)`
- `fourierHandler` and `ifourierHandler`
- `FOURIER` and `IFOURIER`

Unknown forms fall through to unevaluated IR nodes.
