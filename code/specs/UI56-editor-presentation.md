# UI56: Editor Presentation and Clipboard

## Status

Implemented by `browser-form-controls` and `venture-browser-core`.

## Contract

Editable controls retain scalar-indexed selection, composition, viewport
offsets, pointer anchors, and a deterministic caret phase. Hosts supply logical
text metrics and elapsed milliseconds; they do not own editing policy or read a
wall clock inside the reducer.

`ControlEditorPresentation` returns logical rectangles for the visible caret,
selection segments, composition underlines, the control content viewport, and
the IME candidate anchor. Horizontal and multiline vertical offsets move only
enough to reveal the focused caret. Scene producers clip presentation geometry
to that viewport and preserve fixed-position control behavior.

Pointer presses map control-local coordinates to the nearest Unicode scalar.
Pointer drags retain the press anchor and clamp beyond the control to the first
or last available scalar. Copy and cut return an explicit host clipboard
payload; paste consumes an explicit payload. Password controls reject copy and
cut so their underlying value cannot escape through presentation or clipboard
interfaces.

## Determinism

Caret visibility uses a 1,000 ms cycle with a 500 ms visible half. Focus,
selection, text replacement, and composition reset the phase. Hosts advance it
with elapsed time and repaint only when visibility changes. Tests use fixed
metrics and timestamps for identical native and web fixtures.
