---
category: Mosaic compiler pipeline
---

# A runtime that rebuilds innerHTML on every host response must restore focus, or a controlled field accepts one character

**What happened.** The HTML backend's `--emit-project` runtime re-renders by
assigning `root.innerHTML` after every host response, and the Web Component's
`_render` does the same to `shadowRoot.innerHTML` on every attribute change.
Both destroy the focused element. For HTML, whose `onChange` fires per
keystroke, a host that echoes the value back (the normal controlled-field
loop) left the field unfocused after one character: typing stopped. The Web
Component dodged this by binding text `onChange` to the DOM `change` event,
which fires on blur, so its "every keystroke" contract (UI25) silently became
"once, when you leave the field". Every unit test passed, because they assert
the generated strings, not the typing loop.

**Fix.** Capture the focused control before the rebuild (tag, accessible name
and event markers as its identity, plus which of the matching controls it
was; caret; scroll) and restore it after. Then text `onChange` can be
`input`-driven everywhere. Hold `input` events during IME composition and
dispatch on `compositionend`, or the re-render cancels a CJK composition.

**Do differently.** For any runtime that re-renders by replacing markup, test
the controlled-field loop end to end: a fake host that echoes each change,
then type several characters and assert the value, `activeElement` and caret.
A jsdom script does it in seconds. And when one backend binds an event that
does not match the spec (here `change` for a per-keystroke event), ask what it
is working around before "fixing" it.
