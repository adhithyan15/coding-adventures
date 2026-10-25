# Repaired the C# document-AST Windows build gate

The C# document-AST Windows front now executes its native tests on both clean
and warm runs instead of allowing setup conditionals to swallow the command
chain. Both platform fronts now produce real production-assembly coverage and
enforce the 95% library line target.
