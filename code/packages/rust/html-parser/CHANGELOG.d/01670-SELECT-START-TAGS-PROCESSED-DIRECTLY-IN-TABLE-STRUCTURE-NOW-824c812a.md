- `select` start tags processed directly in table structure now report the
  required general parse error before retaining their existing foster-parented
  DOM placement. Selects outside tables and inside cells remain quiet.
