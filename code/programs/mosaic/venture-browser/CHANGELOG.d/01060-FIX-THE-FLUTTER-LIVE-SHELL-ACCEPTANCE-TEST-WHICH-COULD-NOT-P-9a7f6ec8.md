- Fix the Flutter live-shell acceptance test, which could not pass: it looked
  up the generated `ElevatedButton`s as `ButtonStyleButton`, which
  `find.byType` never matches because it compares exact runtime types, and
  its 1400x960 test surface overflowed the grown chrome (173 px across the
  toolbar row; 107 px, and still 203 px at 1200 tall, down the column as
  panels opened). It now finds
  `ElevatedButton` and uses an 1800x2400 surface, tall enough for every
  panel the test opens above the fixed 1024x640 content surface. The Flutter test step now
  runs with `RUST_BACKTRACE=1`, so a native-bridge panic reports where it
  happened.

