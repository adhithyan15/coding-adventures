- **Fixed: IME composition is sent once, when it ends.** An `input` event
  during composition is held back and `compositionend` dispatches the change,
  so a re-render cannot cancel a CJK or dead-key composition mid-word.
