- **Fixed: a controlled field keeps focus while the host re-renders.** The
  `--emit-project` runtime's `render()` replaced `root.innerHTML` wholesale on
  every host response, destroying the focused element. A text field whose host
  echoes each keystroke therefore lost focus after one character, and typing
  stopped. `render()` now captures the focused control, its caret and scroll,
  and restores them on the rebuilt tree. It finds the control again by what it
  is (tag, accessible name, event markers) and which matching control it was,
  not by child index, which would land elsewhere once an `If` or `For` changes
  the tree.
- **IME composition is sent once, when it ends.** An `input` event during
  composition is held back and `compositionend` dispatches the change, so a
  re-render cannot cancel a CJK or dead-key composition mid-word.
- Verified in jsdom with a host that echoes every change: 7/7 behaviour checks
  (focus, caret at end and mid-text, a second field, composition) against 1/7
  for the previous runtime. Venture's HTML interaction test passes.
