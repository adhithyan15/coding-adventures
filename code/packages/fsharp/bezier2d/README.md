# CodingAdventures.Bezier2D.FSharp

Quadratic and cubic Bezier curves over the shared 2D point primitives.

`ToPolyline(tolerance)` follows the shared G2D02 flattening corpus. It uses
iterative de Casteljau subdivision, measures control-point distance to the
finite chord segment, preserves parameter order, and fails closed for invalid
or non-finite geometry. Tolerance must be finite and greater than zero; work is
bounded to depth 32 and 65,535 subdivisions.
