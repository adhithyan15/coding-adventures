- **A disabled card announces as disabled** (`aria-disabled`), rather than
  presenting as an actionable button that does nothing when activated.

`buildEvent` gained an `overrides` channel for payload values the DOM cannot
supply (which card was grabbed, where in the target it landed). It matches by
*own* property: `in` walks the prototype chain, so an emit param named
`constructor` would assign a function off `Object.prototype` — which
`JSON.stringify` then drops, silently delivering an event missing a declared
param — and one named `__proto__` would invoke the setter and add no key at all.

19 new tests.

### Added - HTML event hydration markers

`HostButton` now preserves `onClick`/`onTap` emits as `data-on-click`, and
`HostInput` preserves `onChange`/`onCommit`/`onCancel` emits as
`data-on-change`, `data-on-commit`, and `data-on-cancel`. The HTML backend
stays static and script-free while giving a downstream hydrator the same Mosaic
event names used by the interactive shells.

### Added — UI32-K-html — `--emit-project` standalone-HTML shell

L3 of UI32 (spec PR #4286). Lifts the `index-shell.html` pattern
from UI31-M (PR #4219) into mosaic-compile's single-component
path: `mosaic-compile --backend html --emit-project` now produces
a complete `<!DOCTYPE html>` document alongside the component
fragment, viewable in any browser with zero install.

New public API (mirrors L2 React, PR #4297):

