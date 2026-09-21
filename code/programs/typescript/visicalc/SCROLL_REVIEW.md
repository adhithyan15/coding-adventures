# Keyboard scrolling acceptance — September 20, 2026

Generated VisiCalc, real Rust WASM adapter, in-app browser (1280x720).

Before the repair, fifteen rapid ArrowDown presses selected A16 while the
rendered row window drifted to rows 46–56; F2 left no visible editor.
The generated table helper interpreted asynchronous native scroll events from
its own selection-reveal writes and intermediate render geometry as user travel.
Repeated ResizeObserver deliveries could also forget that a window came from
physical scrolling and reveal the old selection again.

After the shared helper repair:

- Fifteen ArrowDown presses then F2 revealed/focused Cell A16 in both themes
  at 100%, 150% and 200% text in a 900px application container.
- Enter committed 42 to A16 and selected A17. Focus returned to Data table;
  the live region reported the update and the screenshot showed both rows.
- PageDown physically moved the window; F2 returned to selected A1 and focused
  its editor instead of leaving editing active offscreen.
- At 200%, a 400px wheel gesture moved the window from rows 15–18 to 21–24;
  F2 returned to Cell A16 in rows 16–19.

One immediate fill during layout changes reported a stale browser input target;
inspection showed Cell A16 focused and visible. Repeating the fill after that
inspection succeeded. The six-scale/theme navigation checks independently
verified the final focused editor.

Regression coverage simulates delayed native scroll delivery, a callback ref
rebind before spacer measurement, real physical travel, and repeated observer
callbacks following a user-originated window. All 42 web tests and 247 React
emitter unit tests plus its doc test passed; production build passed.

This is browser keyboard/wheel evidence, not touch, screen-reader, or native
framework acceptance. Those remain under the migration epic #14267.
