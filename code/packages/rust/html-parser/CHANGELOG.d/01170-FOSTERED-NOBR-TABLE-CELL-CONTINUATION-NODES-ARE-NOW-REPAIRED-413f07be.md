- Fostered `nobr` table-cell continuation nodes are now repaired while
  processing the EOF token, retiring the final `finish_document` post-parse
  shim and the now-obsolete focused post-parse repair audit while preserving
  the rows in their table, formatting, shell, and interaction audits.
