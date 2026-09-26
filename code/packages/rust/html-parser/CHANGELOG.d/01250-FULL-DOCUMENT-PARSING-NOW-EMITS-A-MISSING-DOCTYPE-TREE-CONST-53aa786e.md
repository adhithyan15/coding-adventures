- Full-document parsing now emits a `missing-doctype` tree-construction
  diagnostic when the initial insertion mode sees a non-whitespace,
  non-comment, non-processing-instruction token before a doctype, closing 783
  previously silent malformed corpus cases.
