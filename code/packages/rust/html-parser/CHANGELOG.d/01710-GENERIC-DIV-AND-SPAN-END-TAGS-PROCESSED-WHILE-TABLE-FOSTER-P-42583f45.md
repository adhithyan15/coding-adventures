- Generic `div` and `span` end tags processed while table foster parenting is
  active now report the required in-table parse error without changing DOM
  recovery. The same end tags outside tables and inside cells remain quiet.
