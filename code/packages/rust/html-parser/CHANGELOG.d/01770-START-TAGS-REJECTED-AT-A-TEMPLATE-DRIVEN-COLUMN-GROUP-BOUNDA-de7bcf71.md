- Start tags rejected at a template-driven column-group boundary now report
  the required parse error while remaining ignored. Additional columns,
  nested templates, columns outside templates, and ordinary `colgroup`
  recovery retain their existing diagnostic behavior.
