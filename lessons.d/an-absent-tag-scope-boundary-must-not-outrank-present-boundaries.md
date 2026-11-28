# An absent tag-scope boundary must not outrank present boundaries

### The failure

An indexed stack parser may represent "no open tag" with `SIZE_MAX` so it can
also represent a missing index. Applying `max()` directly to that sentinel and
real stack positions makes an absent scope look newer than every present
scope. Implied-end selection then silently misses valid list or glossary
boundaries.

### The fix

Select the newest *present* boundary first, then compare the candidate element
against it. Keep absence separate from a numeric stack position, and exercise
each combination of present and absent boundaries in direct parser tests.
