- A `li` start tag now reports the Standard's parse error when implied-end-tag
  recovery closes a non-current list item, covering 2 previously silent
  malformed corpus cases without changing DOM recovery.
