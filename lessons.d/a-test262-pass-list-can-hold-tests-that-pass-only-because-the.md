---
category: Compiler / VM / language pipeline
---

# A test262 pass list can hold tests that pass only because the parser misreads them

`annexB/language/comments/single-line-html-close-asi.js` was on
`v8-test262/expected/parse.txt` although javascript-lexer had no Annex B
HTML-comment support at all. Its body is `var foo = [23]⏎-->[0];`. The lexer
produced `--` `>`, and the newline-blind grammar read the line as
`[23]-- > [0]`, an invalid update target that the parse level never checks. So
the test "passed" for the wrong reason. #17875 then correctly split a
newline-led `--` into a new statement (`[23]; -- > [0]`), and the gate reported
a regression in a crate that had done nothing wrong. Every other
`single-line-html-close*` test failed both before and after, which was the
clue.

The fix went in the layer that owned the gap: the lexer now treats a
line-leading `-->` as a comment. Making the parser split skip `-->` would only
have restored the misparse.

When a parser change "regresses" a listed test, first check whether the test
ever passed for the right reason. Run its sibling tests (here, the same
directory: 2 of 13 passed before and 9 after the fix). Read the token stream of
the old passing parse, not only the error from the new one. Once the real fix
lands, add its new passes to the list (a filtered `--update` adds only them),
so the gate guards the correct behaviour from then on.
