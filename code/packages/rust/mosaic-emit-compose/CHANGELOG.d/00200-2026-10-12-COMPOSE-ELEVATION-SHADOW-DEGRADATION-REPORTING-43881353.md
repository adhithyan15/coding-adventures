## 2026-10-12 (Compose elevation shadow degradation reporting)

- A non-inset CSS `box-shadow` paired with a supported Mosaic `elevation`
  token no longer appears as a dropped style property after Compose emits the
  equivalent native `Modifier.shadow`. Standalone shadows and inset drawing
  effects remain explicit degradation debt (#17356).
