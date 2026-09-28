- `body` and `html` end tags now report the Standard's parse error when
  disallowed elements remain on the stack of open elements, closing 7
  previously silent malformed corpus cases without changing DOM recovery.
