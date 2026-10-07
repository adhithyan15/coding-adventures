---
category: Security boundaries
---

# Reject and destroy a deep pass candidate on its large worker stack

The CV02 fold error latch initially returned its candidate AST from a 128 MiB
worker before propagating failure on the caller. Independent review reproduced
an abort with a 128 KiB caller and 4,096 member wrappers around a traced fold.
The caller recursively destroyed the rejected candidate on its smaller stack.

Return Result from the worker and reject inside it, before transferring any
candidate AST. The isolated child-process regression leaks only its borrowed
input to distinguish candidate destruction from unrelated input destruction;
that bounded test input is reclaimed when the child exits.
