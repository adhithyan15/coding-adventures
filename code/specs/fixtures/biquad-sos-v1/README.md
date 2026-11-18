# Biquad and SOS impulse fixture, version 1

`cases.json` is the shared Python/TypeScript conformance input for exactly two
operations. It contains numeric binary64 JSON values and exact finite expected
samples. Consumers compare finite values exactly and require equal lengths;
`+0` and `-0` compare equal. The fixture does not define a general audio,
units, or coefficient schema.

The input is the 16-sample unit impulse. The first section is
`[0.5, 0.25, 0.5, 2, -2, 0.5]`, in `[b0,b1,b2,a0,a1,a2]` order. After
normalizing by `a0=2`, the recurrence is `y[n]=b0*x[n]+b1*x[n-1]+b2*x[n-2]
-a1*y[n-1]-a2*y[n-2]`, with `x[n]=0` and `y[n]=0` for negative indices.
Consequently `h[0]=1/4` and `h[n]=(6n-3)/2^(n+2)` for `1 <= n < 16`.

The cascade record appends `[1,-1,0,1,0,0]`, which contributes the FIR
factor `1-z^-1`. Thus `g[0]=h[0]`, `g[1]=h[1]-h[0]=1/8`, and
`g[n]=h[n]-h[n-1]=(15-6n)/2^(n+2)` for `2 <= n < 16`. Every expected value
is an exact dyadic rational, with at most seven significant bits and a
denominator no larger than `2^19`; the JSON decimal spellings round-trip to
the same binary64 values. This is independent fixture arithmetic, not a proof
of either implementation's source or native floating-point behavior.

`biquad.process` also records chunk lengths `[3,1,12]`; the full conformance
contract tests every nonempty contiguous partition of the 16 samples. State
is preserved across chunks/calls within a run. `sosfilt` is one-shot and
reinitializes section state per call; no streaming SOS behavior is specified.
