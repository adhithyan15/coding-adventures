# UI76: Dialog and Popover Semantics

## Status

Implemented for Venture's bounded browser profile.

## Goal

Keep dialog and popover state, stacking, dismissal, focus, and accessibility
policy in the shared browser pipeline. Native and web hosts forward pointer,
keyboard, and accessibility actions; they do not own surface visibility.

## Contract

- Closed dialogs and popovers do not participate in layout or hit testing.
- Open surfaces are removed from ancestor clipping and painted in document
  order in a root top layer. Modal dialogs receive a shared backdrop.
- `commandfor`/`command` and `popovertarget` activation use one transaction.
  Auto popovers close conflicting auto popovers; manual popovers remain open.
- Pointer light-dismiss and Escape target only the topmost eligible surface.
  Modal outside-pointer input is consumed even when dismissal is disabled.
- Opening focuses the first enabled descendant control. Closing restores the
  invoker, and modal Tab traversal is constrained to projected descendants.
- Accessibility state includes the stable key, name, kind, mode, modal state,
  dismissal capabilities, open state, and whether the surface is topmost.
- Missing targets and command/target mismatches produce reusable diagnostics.

## Verification

Layout, paint, control-model, browser-session, and deterministic visual-fixture
tests cover hidden surfaces, top-layer order, transformed hit regions, modal
focus, invoker restoration, light-dismiss, Escape, and accessibility actions.
