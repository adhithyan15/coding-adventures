- Full-document parsing now reports the in-body EOF parse error when disallowed
  elements remain on the stack, closing 335 previously silent malformed corpus
  cases without changing DOM recovery or fragment diagnostics.
