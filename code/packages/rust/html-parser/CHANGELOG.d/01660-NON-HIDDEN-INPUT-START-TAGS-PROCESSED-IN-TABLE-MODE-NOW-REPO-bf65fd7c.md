- Non-hidden `input` start tags processed in table mode now report the required
  general parse error before retaining their existing foster-parented DOM
  placement. Hidden inputs keep their specialized in-table behavior and
  diagnostic, while inputs inside cells remain quiet.
