# CLR16: preserve final carriage returns in encoded string input

Status: repair contract, committed before production edits.

PR #15809 landed CLR15 as 346fbf47dca84300a864295644485629ad5775bf.
An independent encoded-call probe shows that a final unterminated CR is
incorrectly stripped: input [13] returns [] and last+CR returns last.

Keep the existing CLR15 contract: strip exactly one CR only when it immediately
precedes a consumed LF. Preserve every byte of an unterminated final line,
including one or more trailing CR bytes. Preserve embedded CR bytes, empty
lines, EOF allocation, input_more peeking, arena lifecycle, exact MemberRef
routing, and integer input ASCII trimming. No source adapter or backend changes.

Retain delimiter information in the private line reader so string reads can
normalize CRLF without mistaking content for a delimiter. Integer input keeps
its existing parsing and trimming semantics.

Use literal encoded MemberRef row 8 call bytes in a permanent regression.
Cover lone CR, final last+CR, repeated final CR, CRLF, repeated CR before LF,
embedded CR, mixed sequential integer/string reads, input_more, and repeated
EOF. Demonstrate failure before repair and success afterward. Run the full
simulator suite, focused lang-aot typed/input execution, Clippy, and doc shard
checks. Security review must pass the exact committed head before publication.
