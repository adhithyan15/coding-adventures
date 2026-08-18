# coding_adventures_ct_compare

Constant-time comparison helpers for byte strings and unsigned counters.

## Why this is a package and not three lines at each call site

A secret comparison that returns early on the first differing byte leaks how far
the match got. An attacker who controls one operand can recover the other a byte
at a time by measuring how long the comparison takes. The defence is to look at
every byte regardless, accumulating differences rather than branching on them.

That is a small amount of code and an easy thing to get subtly wrong — which is
exactly why it belongs in one audited place. Ten other languages in this
repository already have this package; Ruby was the gap, and the D18T durable
epoch-activation profile was hand-rolling its own loop as a result.

## What "constant time" does and does not mean here

These functions have **no data-dependent branches and no early exits**: the work
depends only on the *length* of the inputs, never on their contents. That is the
property callers need.

It is **not** a claim about the machine. Ruby is a managed runtime with a garbage
collector and a JIT, and `String#getbyte` is not a constant-time primitive in the
hardware sense. What this package guarantees is that the *algorithm* leaks
nothing through control flow. Timing that varies because the GC ran is noise an
attacker cannot steer; timing that varies because byte 3 differed is a signal
they can.

Length is deliberately not hidden. `ct_eq` returns `false` immediately for
mismatched lengths, because operand lengths are almost always public — a 32-byte
key is 32 bytes whether or not you know its value — and pretending otherwise
would cost work without buying secrecy.

## Usage

```ruby
require "coding_adventures_ct_compare"

CT = CodingAdventures::CtCompare

CT.ct_eq(stored_mac, computed_mac)          # => true / false, no early exit
CT.ct_eq_fixed(stored_key, candidate_key)   # fixed-width companion
CT.ct_select_bytes(left, right, choice)     # branchless select
CT.ct_eq_u64(stored_counter, seen_counter)  # unsigned 64-bit compare
```

`ct_eq_fixed` is an alias in Ruby. In the statically typed ports it takes
fixed-width arrays, which lets the compiler drop the length check entirely; the
name is kept so the six-language call sites read identically and a reader moving
between them is not left wondering what the difference is.

`ct_select_bytes` avoids branching on `choice` with arithmetic rather than
control flow:

```
result = right ^ ((left ^ right) & mask)
```

With `mask = 0xFF` that reduces to `left`; with `mask = 0x00` it reduces to
`right`. Both inputs are read and the same instructions run either way.

## Development

```sh
bundle install
bundle exec rake test
```
