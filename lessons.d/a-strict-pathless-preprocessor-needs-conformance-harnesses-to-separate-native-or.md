---
category: Compiler / VM / language pipeline
---

# A strict pathless preprocessor needs conformance harnesses to separate native oracle headers from frontend input

The three-way C/SIR/Ruby test corpus included `<stdio.h>` and `<stdint.h>` so
the native C oracle could compile. The old frontend path silently removed those
directives. When `compile_source` began using the bounded PREP01 path, its
fail-closed system-include policy correctly rejected them, and macOS CI failed
in the conformance tests even though the focused frontend tests passed.

Keep the native C source intact for the oracle. Strip only the corpus's exact
two leading oracle headers before passing its program body to the pathless
frontend; let any other directive reach PREP01. Add a host-independent test
that lowers every corpus case, so this integration failure does not depend on
native compiler linking or Ruby availability to be caught locally.
