# G2D02 conservative flattening fixtures (v1)

These process-free cases pin the safety contract in
[G2D02](../../G2D02-bezier2d.md). They are not recordings of any one language's
output, and they do not claim that an existing native package already conforms.
The separate native-lane conformance work item owns that migration.

Each valid case supplies a finite, positive tolerance, a witness point on the
curve, and the witness's distance to the **finite** endpoint segment. A
conforming implementation must not return the endpoint chord when the case
says `subdivide`; `chord` means the maximum control-point distance to that
segment is at most the tolerance. `invalid-tolerance` cases must fail before
subdivision and return no partial polyline. The corpus pins a recursion depth
of 32 and at most 65,535 subdivisions; reaching either budget without meeting
the error bound is an error, not a successful approximation.

The `quadratic-budget-exhaustion` case has a separate
`expected_termination: budget-error`. Its arch is $y(x)=2x(1-x)$, so the
vertical midpoint residual of any chord spanning parameter width $w$ is
$w^2/2$. Every chord has slope at most 2 in magnitude, so its Euclidean
point-to-segment error is at least $w^2/(2\sqrt{5})$. At most 65,535 splits
produce at most 65,536 chords; at least one spans $w \geq 1/65{,}536$, with
error greater than the case's $10^{-12}$ tolerance.
This proves that no permitted split polyline can succeed, independent of the
implementation's traversal order.

The symmetric S case defeats a midpoint-only flatness test: its midpoint lies
on the chord, while its quarter-point is 0.28125 away. The collinear overshoot
case defeats distance to the infinite chord line: its midpoint lies on that
line, but 2.125 beyond the finite segment. The coincident-endpoint loop checks
the zero-length chord path.

`schema.json` closes the transport shape. The independent Python oracle
recomputes de Casteljau witnesses, finite-segment distances, and expected
root dispositions, rejects duplicate/nonfinite JSON transport, and enforces
the pinned limits. Run it with:

```sh
python -B code/scripts/bezier2d_flattening_conformance.py
python -B -m unittest discover -s code/scripts/tests -p test_bezier2d_flattening_conformance.py
```

Both commands are scheduled in the unconditional repo-wide metadata-contracts
CI step for every pull request. Passing the neutral oracle does not imply that
each native Bezier2D reader already satisfies the flattening contract.

Adapters should report the observed root disposition and validate the full
flattened polyline against the G2D02 error and termination contract. They
should not compare exact subdivision vertices across languages: many safe
polylines satisfy the same geometric tolerance.
