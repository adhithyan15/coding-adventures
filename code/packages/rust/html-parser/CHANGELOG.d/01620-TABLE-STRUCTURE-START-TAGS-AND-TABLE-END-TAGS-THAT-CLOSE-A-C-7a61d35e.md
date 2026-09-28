- Table-structure start tags and `table` end tags that close a caption now
  report the current-Standard parse error when implied-end-tag generation
  leaves a non-caption node current. Caption-scoped `table` end tags also close
  the caption before reprocessing the token in table mode. The checked legacy
  corpus does not cover this caption-plus-formatting branch.
