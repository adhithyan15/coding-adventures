---
category: Mosaic compiler pipeline
---

# A hand-written host props mapper silently drops every new core prop

Venture's WinUI host copies the core's chrome props onto the generated
component by hand, one line per prop, in `ApplyResponse`
(`code/programs/mosaic/venture-browser/host/xaml/MosaicHost.cs`). When the
core gained page zoom, print/share and Page Information props, the Qt host
picked them up, but the XAML host never did. Its Zoom Reset button's label
stayed `null`. The acceptance check waited for "125%" and failed on every
Windows run, while the core had zoomed correctly (status "Zoom: 125%").

It was first read as CI load and then as a parser regression. The cause only
became clear once the failure reported what the host actually saw:
`statusText`, the label, and the last bridge outcome.

What to do:
- When adding a prop to `BrowserChromeProps`, add it to every hand-written host
  mapper (XAML `ApplyResponse`, and any other host that maps by name).
- Make acceptance failures report the observed state, not just "did not
  reflow". The first run with that detail named the cause.
