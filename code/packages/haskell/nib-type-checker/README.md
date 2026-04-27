# nib-type-checker

Haskell `nib-type-checker` performs lightweight semantic checks over the AST
produced by the local `nib-parser`. It keeps the original AST and reports
plain diagnostics so later compiler stages can stay stage-aware.
