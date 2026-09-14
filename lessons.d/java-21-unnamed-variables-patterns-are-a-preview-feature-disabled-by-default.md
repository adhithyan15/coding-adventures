---
category: Compiler / VM / language pipeline
---

# Java 21: unnamed variables/patterns (`_`) are a preview feature, disabled by default

`case Foo _` and `case Foo(_, var x)` trigger `unnamed variables are a preview feature and are disabled by default` on Java 21 (JEP 443 — finalized only in Java 22). Fix: replace `_` with a named binding: `ignored`, `op2`, `nm`, `pat2`, etc. Record patterns with named bindings (`case Foo(var x, var y)`) and type patterns with names (`case Foo bar`) ARE finalized in Java 21.
