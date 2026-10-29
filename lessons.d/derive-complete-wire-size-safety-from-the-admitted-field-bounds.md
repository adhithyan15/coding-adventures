---
category: Cryptography & security review
---

# Derive complete wire-size safety from the admitted field bounds

The ALPN client-offer constructor initially retained checked-addition and
extension-length failure branches after already limiting the offer to 64 names
of at most 255 bytes. Those public bounds make the largest encoded value only
16,386 bytes, so the later `u16` extension-size failure was unreachable. The
dead defensive branch obscured the actual proof and appeared as an uncovered
security-relevant path.

For bounded wire constructors, calculate the maximum complete encoding from
the admitted count and field widths first. Validate those public bounds, state
the derived invariant with a debug assertion, and keep runtime failure paths
only for states callers can actually reach. This makes both the resource proof
and coverage evidence describe the real API surface.
