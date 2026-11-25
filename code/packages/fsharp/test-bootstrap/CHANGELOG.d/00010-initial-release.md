## 0.1.0

- Initial F# binding over the C# pbt-v1 engine (VER05l): `Gen<'T>` with
  `map`/`filter`/`bind`/`oneOf`/`frequency`/`list`/`tuple`, the `gen { }`
  computation expression, `Prop.check`/`Prop.forAll`, a discriminated-union
  `Verdict`, `uint64` seeds and `Result`-returning shrinkers.
- Replays the whole shared pbt-v1 corpus (25/25 exact, step counts included)
  through the F# API, with per-op negative controls and F#-layer sabotage seams.
