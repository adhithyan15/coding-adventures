- The WinUI host now copies all of the core's chrome props onto the generated
  component. `ApplyResponse` had never mapped `zoom-label`, the three
  `zoom-*-disabled` flags, `print-page-disabled`, `share-page-disabled` or the
  six `page-info-*` props, so the Zoom Reset button's label stayed empty and
  the acceptance's page-zoom check always failed although the core had zoomed
  ("Zoom: 125%"). A failed zoom check also now reports the label it saw,
  whether Zoom In was enabled and invoked, the status text and the last chrome
  event's bridge outcome.

