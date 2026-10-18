- The Flutter live test runs in a 1024x700 window. It had been enlarged to
  1800x2400 to get past overflowing rows: the toolbar overflowed by 173 px at
  1400 wide, and at 1024 the page-actions row by 159 px and library-actions by
  0.4 px. The emitter has since lowered the address field's and the content
  surface's `flex-grow` to `Expanded`, and the two button rows now set
  `flex-wrap: wrap` (light and dark styles), lowered to `Wrap`. The whole run
  (bookmarks, page info, zoom, source, history, scroll, hover, link) passes
  at 1024x700 and 1400x960 with no overflow. At an ordinary size again, the
  test's own "any RenderFlex overflow fails" check guards the layout. The
  changelog moves to `CHANGELOG.d/`, one file per entry.
