---
category: Mosaic compiler pipeline
---

# Measure virtual row capacity from viewport space rather than retained content height

Native resize testing found two feedback traps: cached offscreen ItemsRepeater cells can retain unarranged geometry, and a short current window has a short ActualHeight. Inspect geometry only in the proposed visible window and derive capacity from the finite viewport, honoring MaxHeight. Anchor the virtual table at the top so centering does not change its viewport origin. Exercise both shrinking and growing in the actual host.
