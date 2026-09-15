# A regression test for a scan can pass against the bug when the scanned function short-circuits

Fixing an occurrences x spans quadratic, I wrote the timing test as
`'"a" look at '.repeat(40_000)` — many quoted spans, many cue occurrences. It passed
instantly against the *unfixed* code. The function returns on the first surviving
occurrence, and the first `look at` was outside the quotes, so it returned after one
iteration and never touched the span list.

The shape that exercises it is `'"look at" '.repeat(40_000)`: every occurrence must be
DROPPED for the loop to run to the end.

**Rule: to benchmark a loop, the input must make the loop run.** For any function with an
early return, the worst case is the input where the early return never fires — which is
usually the *negative* result, not the positive one. Check the timing test actually got
slower against the old code, exactly as you would check a correctness test goes red.
