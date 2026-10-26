---
category: Compiler / VM / language pipeline
---

# Trace the number parser to the C function's return before stating what value a release gives a malformed literal

**What went wrong.** The LANG84 phase 7c slice 2 plan said that CPython 2.2
and 2.4 lex a bare `0x` and that `parsenumber` reads it as "the float
`0.0`". It was committed alone as the spec. The claim came from reading
`parsenumber`'s fall-through to `atof` without checking the line before it.
For a spelling with a leading `0`, `parsenumber` (`Python/compile.c`) calls
`PyOS_strtoul(s, &end, 0)`. In `Python/mystrtoul.c` at those tags, base 0
consumes `0x`, sets base 16, finds no hex digit, and leaves `end` at the
terminator. So `*end == '\0'` holds and the result is the int `0`. The float
path is never reached.

**The fix.** The implementation commit corrected the plan's text and noted
the change in its "Status" paragraph. The pairs refuse `0x` as a lexer error
(a stated narrowing), so no expectation or test depended on the wrong
value.

**Next time.** Before a spec or `.cite` states what a release does with an
edge-case spelling, follow the whole path in the tag's source: the
tokenizer, then `parsenumber`, then the conversion function, to the point
where it returns. Write down the line that decides, not the function's
general shape. Do this before the spec commit, because the spec is
committed alone and cited by later work.
