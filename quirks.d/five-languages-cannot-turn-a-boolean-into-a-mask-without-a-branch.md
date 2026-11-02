---
package: */ct-compare
lane: go java kotlin ruby elixir
rung: E5
status: open
---

# Five languages cannot turn a boolean into a mask without a branch

CT01 forbids branching on the secret selection bit. Go has no bool-to-integer
conversion, Java and Kotlin have none for `boolean`, and in Ruby and Elixir
every conversion is an `if`, a `?:` or a pattern match. A boolean `choice` in
those languages therefore cannot become a mask without a branch. Ruby's
validation `[true, false].include?(choice)` also did one comparison for `true`
and two for `false`.

**Fix in progress** (adhithyan15/coding-adventures#18782): CT01 lets such lanes take an integer bit (exactly 0 or 1, as
Go's `crypto/subtle` does), validated by testing `choice & ~1`, which takes the
same path for both valid values. Callers must also produce the bit without a
branch; bit-returning equality variants are a follow-up.
