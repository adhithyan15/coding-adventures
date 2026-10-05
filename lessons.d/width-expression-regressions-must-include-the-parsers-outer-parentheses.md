---
category: Mosaic compiler pipeline
---

# Width expression regressions must include the parser's outer parentheses

A structural XAML table width test built Expr("size") directly, but real Mosaic layout parsing supplied Expr("( size )"). The first regression passed while generated VisiCalc headers still omitted their width binding. Compare identifier expressions through the existing balanced-parenthesis helper and keep a parser-shaped expression in the regression. Fresh generated-app inspection remains necessary after emitter tests pass.
