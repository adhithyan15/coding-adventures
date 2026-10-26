---
category: Rust
---

# A validated candidate must be revalidated at the final commit boundary if it remains publicly mutable

An earlier validation proves only the bytes that existed at that instant. If a
public compatibility API can mutate the candidate afterward, an infallible
commit silently turns the earlier check into stale evidence and can clock an
invalid architectural value into authoritative DFF Q. Either make the settled
value immutable or make commit fallible and revalidate immediately before its
first side effect. Exercise the actual external API sequence—settle, mutate to
an invalid state, commit—and compare the complete Q snapshot to prove rejection
happens before any clock.

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.
