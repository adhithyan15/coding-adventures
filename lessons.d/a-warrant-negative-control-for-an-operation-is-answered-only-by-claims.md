---
category: Testing & coverage
---

# A Warrant negative control for an operation is answered only by claims that start with that operation, so pin each side of a two-operation claim in both orders

The x25519 pilot (WX-N) failed its own negative controls: `flip-result-bit` on
`x25519` was "accepted by every suite of the operation" in the exhaustive,
property and fuzz kinds, though the RFC examples caught it. Each control attacks
one operation and is answered only by the claims whose *first* operation it is
(the claim's `op`). The claims that started with `x25519` (`x25519(k, u) ==
x25519(clamp_scalar(k), u)`, the zero-point refusal) are consistent under the
fault, because both sides go through the attacked adapter or the result is an
`Err`; the one claim that pins `x25519` against an honest oracle,
`x25519_base(k) == x25519(k, nine)`, started with `x25519_base`, so it only
answered that operation's control.

Fix: write a two-operation pinning claim in both orders (`x25519(k, nine) ==
x25519_base(k)` and the reverse), and do not bind a fault the operation cannot
show (`missing-error` on `x25519_base`, which has no reachable error). Also: a
wrong-length value that a later call receives is "unreachable by type", a
discard, not a failure, so a claim that composes the attacked operation with
itself (`flip4(flip4(a)) == a` under `drop-last-result-byte`) does not detect the
fault; add a length claim (`len(flip4(a)) == 4`).
