- A `dt` or `dd` start tag now reports the in-body parse error when
  implied-end-tag recovery closes a non-current description-list item,
  covering the remaining silent in-body list case without changing DOM
  recovery or adjacent description-list diagnostics.
