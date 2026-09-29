- Repeated `option` and `optgroup` start tags processed with an open `select`
  now report the current-Standard parse error when implied-end-tag recovery
  closes a scoped option or group. The legacy html5lib rows exercise this DOM
  recovery without declaring the branch error.
