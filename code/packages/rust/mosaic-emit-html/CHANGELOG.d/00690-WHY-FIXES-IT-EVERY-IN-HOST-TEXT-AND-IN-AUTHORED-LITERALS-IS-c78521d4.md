- **Why `&` fixes it:** every `&` in host text and in authored literals is
  escaped, so neither can spell a name containing a raw `&`.

### Security -- runtime `escapeHtml` also encodes `=`, `{` and `}`

