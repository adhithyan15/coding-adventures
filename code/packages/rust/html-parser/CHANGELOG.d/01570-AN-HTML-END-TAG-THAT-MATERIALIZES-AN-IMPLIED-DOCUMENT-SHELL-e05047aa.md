- An `html` end tag that materializes an implied document shell after leading
  body text now enters the after-html insertion mode, so following character
  data and start tags report their required parse error. This covers 2
  previously silent malformed corpus cases without changing DOM recovery or
  undeclared-diagnostic coverage.
