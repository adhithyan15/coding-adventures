- Duplicate `html`, `head`, and `body` start tags now emit tree-construction
  diagnostics while retaining the Standard's attribute-merge recovery for
  `html` and `body`, closing 28 previously silent malformed corpus cases.
