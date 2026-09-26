- **Escape survives a re-render.** Once a drag is in flight the keyboard handler
  no longer requires the event to come from the grabbed card. The first host
  response replaces the subtree, detaching the focused element and dropping focus
  to `<body>` — a focus-gated Escape would become unreachable exactly when it is
  needed, leaving a keyboard-only user with an unresolvable drag.
