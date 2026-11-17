---
category: Testing & coverage
---

# encoding/json keeps the last duplicate key and replaces bad UTF-8 and lone surrogates, so an untrusted Go corpus needs its own strict reader

Same class as the TypeScript and Python lessons, with Go's own spelling. `encoding/json` into `any` keeps the
last of two equal keys without an error, turns every number into a `float64` (an integer above 2^53 rounds;
with `UseNumber` it is kept as text but a fraction, an exponent and `-0` are still accepted where a count
belongs), and replaces invalid UTF-8 and a lone surrogate escape such as `"\ud800"` with U+FFFD. For a corpus
that carries digests, counts and 64-bit values, any of these makes two readers disagree about one document.

The Go Warrant runner has a small one-pass reader that refuses duplicate keys, invalid UTF-8, an unpaired or
reversed surrogate escape, a fraction, an exponent, `-0`, an integer above 2^53 - 1, nesting deeper than 256
levels and trailing data, and writes canonical output (sorted keys, two-space indent, ASCII-only escapes) that
is byte-identical to the Python and TypeScript lanes. `U64` values travel as tagged decimal text and are
parsed with `strconv.ParseUint`, never as JSON numbers.

Do differently: if JSON is a trust boundary in Go, do not call `json.Unmarshal` on it; use a token-level check
or your own reader, and test each of those inputs.
