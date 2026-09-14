# Depth limits do not bound breadth

`glyph-parser` capped composite nesting at 10 levels, which reads like a
complete guard against a malicious font. It is not. Depth bounds the *height*
of the tree and says nothing about its *width*: a glyph nine levels deep —
inside the cap — that fans out to eight components per level asks for 8^9, over
134 million resolutions, from a few hundred bytes.

A recursion guard needs a **total work budget**, not just a depth counter, and
the test has to build the wide-and-shallow font specifically. A chain deep
enough to trip the depth limit passes while proving nothing about breadth — my
first attempt did exactly that, and the assertion on which error fired is what
exposed it.
