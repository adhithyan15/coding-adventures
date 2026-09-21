---
category: Rust
---

# A final carriage return is content unless a line feed was consumed

The CLR15 reader stripped a CR suffix after splitting on LF without retaining
whether LF existed. A literal encoded input_str call with input [13] returned
[]; last+CR also lost its final byte. The final-line preservation contract was
therefore violated even though CRLF and embedded-CR tests passed.

Return delimiter information with the line slice and strip CR only when LF was
consumed. Test lone and repeated final CR bytes separately from CRLF, repeated
CR before LF, mixed readers, peeking, and EOF. The new regression tests failed
on the landed implementation before the repair was applied.
