- Obsolete `menuitem` end tags now report the in-body parse error when no
  matching `menuitem` is current, covering 4 previously silent malformed
  corpus cases without changing DOM recovery or undeclared-diagnostic coverage.
