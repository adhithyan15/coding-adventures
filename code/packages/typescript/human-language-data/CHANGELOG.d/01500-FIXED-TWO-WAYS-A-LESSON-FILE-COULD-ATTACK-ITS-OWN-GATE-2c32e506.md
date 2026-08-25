### Fixed - two ways a lesson file could attack its own gate

Security review of the above, both verified by execution.

**A quadratic comment strip.** `replace(/<!--[\s\S]*?-->/g, "")` looks like the
safe construct and is not. With `/g` the engine retries at every `<!--`, and
when there is no closing `-->` each start expands one character at a time to
EOF before failing -- O(n squared) in the *count* of `<!--` tokens, with no
`-->` needed anywhere. Measured: 500 KB of repeated `<!--` took **13 seconds**,
and a 4 MB lesson would have pinned a core for roughly fifteen minutes. Now a
monotonic `indexOf` scan: the same input takes **22 ms**, and an unterminated
comment keeps the remainder of the file verbatim rather than swallowing it.

**A directory name resolving through `Object.prototype`.** `PERSON_LABELS` is a
plain object indexed by `lesson.language`, which `loader.ts` takes straight from
`readdirSync` -- so a track directory named `constructor`, `toString` or
`__proto__` resolved to an inherited member, passed the `undefined` check, and
threw on `.includes`. This package already exports `hasOwn` for exactly this and
uses it at five sites; `parse.ts` guards its own language lookup the same way
and `ramp.ts` documents this identical bug being fixed once before. The new
module simply skipped the convention.


