### Fixed -- a percentage `border-radius` is resolved against the part's box (#15225)

Every backend lowers `border-radius` through a pixel parser -- Compose
`RoundedCornerShape(N.dp)`, SwiftUI `.cornerRadius(N)`, Qt `radius: N`, XAML
`CornerRadius="N"`, Flutter `BorderRadius.circular(N)` -- so `50%` matched
none of them and was dropped.

Trestle authors it **six times per theme**, and every one rendered as a
SQUARE on the five native backends: both pill status dots, the progress
ring's fill and hole, and both theme-toggle buttons. A progress ring drawn
as a square is not a subtle defect.

Resolved here for the same reason as `currentColor`: the answer is
arithmetic over the authored box, not a property of any target language.

**Only a square box with literal sides is resolved.** A percentage radius on
a non-square box is an ellipse, which none of these frameworks expresses as
a plain corner radius, and a content-sized box has no pixel value to resolve
against at emit time. Both are left exactly as authored -- the web keeps
resolving them natively, the native backends keep dropping them, and neither
is ever silently wrong. A malformed percentage is declined rather than
coerced to `0`, because a zero radius is a square, which is the bug itself.

Three details the arithmetic needs to be right about:

- **Clamped to half the side.** CSS's overlap rule scales adjacent radii, so
  `border-radius: 100%` on a square renders exactly as `50%`. Compose,
  SwiftUI and QML clamp for us; XAML does not, so resolving to the full side
  would leave one backend at double the others.
- **Rounded.** f64 `Display` never uses exponent notation in *either*
  direction, so an unrounded product is either ugly (`0.30000000000000004`
  from a 3px box at 10%) or enormous -- a subnormal side produces a
  several-hundred-character literal shipped to every backend.
- **The side must be a length every backend accepts.** `6e0` parses as a
  float but is dropped by Compose's character-class parser, which would
  leave a content-sized box carrying a radius resolved against a width it
  never applied.


